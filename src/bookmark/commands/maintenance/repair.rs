use std::collections::HashSet;

use crate::bookmark::storage::db_path;
use crate::bookmark::undo::record_undo_batch;
use crate::bookmark_core::BookmarkSource;
use crate::bookmark_state::{Bookmark, Store};
use comfy_table::{Attribute, Cell, Color, Table};
use console::Term;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};

use crate::cli::DedupCmd;
use crate::model::{DedupMode, ListFormat, parse_dedup_mode};
use crate::output::{CliError, CliResult, can_interact};
use crate::output::{apply_pretty_table_style, format_age, print_table};
use crate::store::now_secs;

use super::report::resolve_output_format;

pub(crate) fn cmd_dedup(args: DedupCmd) -> CliResult {
    let mode = parse_dedup_mode(&args.mode).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid mode: {}.", args.mode),
            &["Fix: Use one of: path | name"],
        )
    })?;
    let format = resolve_output_format(&args.format)?;

    let file = db_path();
    let mut store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let before = store.clone();
    let groups = build_duplicate_groups(&store.bookmarks, mode);

    if groups.is_empty() {
        ui_println!("No duplicates found.");
        return Ok(());
    }

    if format == ListFormat::Tsv || !can_interact() || format == ListFormat::Json {
        if format == ListFormat::Json {
            let mut items = Vec::new();
            for (key, ids) in &groups {
                for bookmark in collect_group_bookmarks(&store.bookmarks, ids) {
                    items.push(serde_json::json!({
                        "key": key,
                        "name": bookmark_display_name(&bookmark),
                        "path": bookmark.path,
                        "visits": bookmark.visit_count.unwrap_or(0),
                        "last_visited": bookmark.last_visited.unwrap_or(0),
                        "tags": bookmark.tags,
                    }));
                }
            }
            out_println!("{}", serde_json::Value::Array(items));
            return Ok(());
        }
        for (key, ids) in groups {
            for bookmark in collect_group_bookmarks(&store.bookmarks, &ids) {
                out_println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    key,
                    bookmark_display_name(&bookmark),
                    bookmark.path,
                    bookmark.visit_count.unwrap_or(0),
                    bookmark.last_visited.unwrap_or(0),
                    bookmark.tags.join(",")
                );
            }
        }
        return Ok(());
    }

    let mut changed = false;
    for (key, ids) in groups {
        let items = collect_group_bookmarks(&store.bookmarks, &ids);
        if items.len() < 2 {
            continue;
        }

        let mut table = Table::new();
        apply_pretty_table_style(&mut table);
        table.set_header(vec![
            Cell::new("Name")
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new("Path")
                .add_attribute(Attribute::Bold)
                .fg(Color::Magenta),
            Cell::new("Tags")
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
            Cell::new("Visits")
                .add_attribute(Attribute::Bold)
                .fg(Color::Green),
            Cell::new("Last")
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
        ]);
        for bookmark in &items {
            let tags = if bookmark.tags.is_empty() {
                Cell::new("-")
                    .fg(Color::DarkGrey)
                    .add_attribute(Attribute::Dim)
            } else {
                Cell::new(bookmark.tags.join(",")).fg(Color::Yellow)
            };
            table.add_row(vec![
                Cell::new(bookmark_display_name(bookmark))
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
                Cell::new(bookmark.path.clone())
                    .fg(Color::DarkGrey)
                    .add_attribute(Attribute::Dim),
                tags,
                Cell::new(bookmark.visit_count.unwrap_or(0)).fg(Color::Green),
                Cell::new(format_age(bookmark.last_visited.unwrap_or(0))).fg(Color::Yellow),
            ]);
        }

        let label = if mode == DedupMode::Path {
            "Duplicate path"
        } else {
            "Duplicate name"
        };
        ui_println!("{}: {}", label, key);
        print_table(&table);

        let mut choices = vec!["skip".to_string()];
        for bookmark in &items {
            choices.push(format!("keep: {}", bookmark_choice_label(bookmark)));
        }

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an action")
            .default(0)
            .items(&choices)
            .interact_on(&Term::stderr());

        let Ok(sel) = selection else {
            return Err(CliError::new(3, "Cancelled."));
        };
        if sel == 0 {
            continue;
        }

        let keep_idx = sel - 1;
        let keep = &items[keep_idx];

        if !args.yes {
            let confirm = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!(
                    "Merge duplicates into '{}' and delete others?",
                    bookmark_choice_label(keep)
                ))
                .default(false)
                .interact_on(&Term::stderr());

            if !matches!(confirm, Ok(true)) {
                ui_println!("Skipped.");
                continue;
            }
        }

        let keep_id = keep.id.clone();
        if apply_dedup_merge(&mut store, &keep_id, &ids) {
            changed = true;
            ui_println!("Merged into '{}'.", bookmark_choice_label(keep));
        }
    }

    if changed {
        store
            .save(&file, now_secs())
            .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
        let after = store.clone();
        if let Err(err) = record_undo_batch(&file, "dedup", &before, &after) {
            crate::output::emit_warning(format!("Undo history not recorded: {}", err.message), &[]);
        }
    }

    Ok(())
}

fn build_duplicate_groups(bookmarks: &[Bookmark], mode: DedupMode) -> Vec<(String, Vec<String>)> {
    let mut groups: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for bookmark in bookmarks {
        let Some(key) = dedup_key(bookmark, mode) else {
            continue;
        };
        groups.entry(key).or_default().push(bookmark.id.clone());
    }
    groups.into_iter().filter(|(_, ids)| ids.len() > 1).collect()
}

fn dedup_key(bookmark: &Bookmark, mode: DedupMode) -> Option<String> {
    match mode {
        DedupMode::Path => Some(bookmark.path_norm.clone()),
        DedupMode::Name => bookmark.name_norm.clone(),
    }
}

fn collect_group_bookmarks(bookmarks: &[Bookmark], ids: &[String]) -> Vec<Bookmark> {
    ids.iter()
        .filter_map(|id| bookmarks.iter().find(|bookmark| bookmark.id == *id).cloned())
        .collect()
}

fn apply_dedup_merge(store: &mut Store, keep_id: &str, ids: &[String]) -> bool {
    let items = collect_group_bookmarks(&store.bookmarks, ids);
    if items.len() < 2 {
        return false;
    }

    let Some(keep_idx) = items.iter().position(|bookmark| bookmark.id == keep_id) else {
        return false;
    };
    let mut merged = items[keep_idx].clone();
    let remove_ids: HashSet<String> = items
        .iter()
        .filter(|bookmark| bookmark.id != keep_id)
        .map(|bookmark| bookmark.id.clone())
        .collect();

    merge_bookmark_metadata(
        &mut merged,
        &items
            .iter()
            .enumerate()
            .filter(|(index, _bookmark)| *index != keep_idx)
            .map(|(_index, bookmark)| bookmark)
            .cloned()
            .collect::<Vec<_>>(),
    );

    let Some(position) = store.bookmarks.iter().position(|bookmark| bookmark.id == keep_id) else {
        return false;
    };
    store.bookmarks[position] = merged;
    store
        .bookmarks
        .retain(|bookmark| bookmark.id == keep_id || !remove_ids.contains(&bookmark.id));
    true
}

fn merge_bookmark_metadata(target: &mut Bookmark, others: &[Bookmark]) {
    let mut tag_set: HashSet<String> = target.tags.iter().map(|tag| tag.to_lowercase()).collect();
    let mut visit_sum = target.visit_count.unwrap_or(0);
    let mut has_visit_count = target.visit_count.is_some();
    let mut frecency_sum = target.frecency_score;

    for other in others {
        for tag in &other.tags {
            if tag_set.insert(tag.to_lowercase()) {
                target.tags.push(tag.clone());
            }
        }
        if let Some(visit_count) = other.visit_count {
            visit_sum = visit_sum.saturating_add(visit_count);
            has_visit_count = true;
        }
        target.last_visited = match (target.last_visited, other.last_visited) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        };
        target.pinned = target.pinned || other.pinned;
        if target.desc.trim().is_empty() && !other.desc.trim().is_empty() {
            target.desc = other.desc.clone();
        }
        if target.workspace.is_none() && other.workspace.is_some() {
            target.workspace = other.workspace.clone();
        }
        target.created_at = match (target.created_at, other.created_at) {
            (0, value) => value,
            (left, 0) => left,
            (left, right) => left.min(right),
        };
        target.source = preferred_source(target.source, other.source);
        frecency_sum += other.frecency_score;
    }

    target.visit_count = has_visit_count.then_some(visit_sum);
    target.frecency_score = frecency_sum;
}

fn preferred_source(left: BookmarkSource, right: BookmarkSource) -> BookmarkSource {
    match (source_rank(left), source_rank(right)) {
        (left_rank, right_rank) if left_rank <= right_rank => left,
        _ => right,
    }
}

fn source_rank(source: BookmarkSource) -> u8 {
    match source {
        BookmarkSource::Explicit => 0,
        BookmarkSource::Imported => 1,
        BookmarkSource::Learned => 2,
    }
}

fn bookmark_display_name(bookmark: &Bookmark) -> String {
    bookmark
        .name
        .clone()
        .unwrap_or_else(|| format!("(unnamed:{})", short_id(&bookmark.id)))
}

fn bookmark_choice_label(bookmark: &Bookmark) -> String {
    format!(
        "{} [{}]",
        bookmark_display_name(bookmark),
        match bookmark.source {
            BookmarkSource::Explicit => "explicit",
            BookmarkSource::Imported => "imported",
            BookmarkSource::Learned => "learned",
        }
    )
}

fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark::undo::{record_undo_batch, run_undo_steps};
    use std::path::Path;
    use tempfile::tempdir;

    fn sample_store_for_dedup() -> Store {
        let mut store = Store::new();
        store.set("home", "C:/work/home", Path::new("C:/work"), None, 10).unwrap();
        store.set("main", "C:/work/home", Path::new("C:/work"), None, 20).unwrap();
        store.bookmarks[0].tags = vec!["a".to_string()];
        store.bookmarks[0].visit_count = Some(2);
        store.bookmarks[1].tags = vec!["b".to_string()];
        store.bookmarks[1].visit_count = Some(3);
        store
    }

    #[test]
    fn build_duplicate_groups_by_path_uses_store_ids() {
        let store = sample_store_for_dedup();
        let groups = build_duplicate_groups(&store.bookmarks, DedupMode::Path);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].1.len(), 2);
    }

    #[test]
    fn apply_dedup_merge_combines_tags_and_visits() {
        let mut store = sample_store_for_dedup();
        let ids: Vec<String> = store.bookmarks.iter().map(|bookmark| bookmark.id.clone()).collect();
        let keep_id = ids[0].clone();

        assert!(apply_dedup_merge(&mut store, &keep_id, &ids));
        assert_eq!(store.bookmarks.len(), 1);
        assert_eq!(store.bookmarks[0].visit_count, Some(5));
        assert!(store.bookmarks[0].tags.iter().any(|tag| tag == "a"));
        assert!(store.bookmarks[0].tags.iter().any(|tag| tag == "b"));
    }

    #[test]
    fn dedup_merge_can_be_undone_through_delta_history() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".xun.bookmark.json");

        let before = sample_store_for_dedup();
        let ids: Vec<String> = before.bookmarks.iter().map(|bookmark| bookmark.id.clone()).collect();
        let keep_id = ids[0].clone();

        let mut after = before.clone();
        assert!(apply_dedup_merge(&mut after, &keep_id, &ids));

        record_undo_batch(&db, "dedup", &before, &after).unwrap();
        let mut current = after.clone();
        run_undo_steps(&db, &mut current, 1).unwrap();

        assert_eq!(current.bookmarks.len(), 2);
        assert!(current.bookmarks.iter().any(|bookmark| bookmark.name.as_deref() == Some("home")));
        assert!(current.bookmarks.iter().any(|bookmark| bookmark.name.as_deref() == Some("main")));
    }
}
