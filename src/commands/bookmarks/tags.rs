use std::collections::HashSet;

use comfy_table::{Attribute, Cell, Color, Table};

use crate::cli::{TagAddCmd, TagCmd, TagListCmd, TagRemoveCmd, TagRenameCmd, TagSubCommand};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, prefer_table_output, print_table};
use crate::store::{Lock, db_path, save_db};
use crate::util::parse_tags;

use super::load_bookmark_db;

pub(crate) fn cmd_tag(args: TagCmd) -> CliResult {
    match args.cmd {
        TagSubCommand::Add(a) => cmd_tag_add(a),
        TagSubCommand::Remove(a) => cmd_tag_remove(a),
        TagSubCommand::List(a) => cmd_tag_list(a),
        TagSubCommand::Rename(a) => cmd_tag_rename(a),
    }
}

pub(crate) fn cmd_tag_add(args: TagAddCmd) -> CliResult {
    let tags = parse_tags(&args.tags);
    if tags.is_empty() {
        ui_println!("No tags to add.");
        return Ok(());
    }

    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load_bookmark_db(&file)?;

    let Some(entry) = db.get_mut(&args.name) else {
        return Err(CliError::with_details(
            2,
            format!("Bookmark '{}' not found.", args.name),
            &["Fix: Run `xun list` to see existing bookmarks."],
        ));
    };

    let mut existing: HashSet<String> = entry.tags.iter().map(|t| t.to_lowercase()).collect();
    let mut added = 0usize;
    for t in tags {
        if existing.insert(t.to_lowercase()) {
            entry.tags.push(t);
            added += 1;
        }
    }

    if added == 0 {
        ui_println!("No new tags added.");
        return Ok(());
    }

    save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
    ui_println!("Added {} tag(s) to '{}'.", added, args.name);
    Ok(())
}

pub(crate) fn cmd_tag_remove(args: TagRemoveCmd) -> CliResult {
    let tags = parse_tags(&args.tags);
    if tags.is_empty() {
        ui_println!("No tags to remove.");
        return Ok(());
    }

    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load_bookmark_db(&file)?;

    let Some(entry) = db.get_mut(&args.name) else {
        return Err(CliError::with_details(
            2,
            format!("Bookmark '{}' not found.", args.name),
            &["Fix: Run `xun list` to see existing bookmarks."],
        ));
    };

    let remove_set: HashSet<String> = tags.iter().map(|t| t.to_lowercase()).collect();
    let before = entry.tags.len();
    entry
        .tags
        .retain(|t| !remove_set.contains(&t.to_lowercase()));
    let removed = before.saturating_sub(entry.tags.len());

    if removed == 0 {
        ui_println!("No tags removed.");
        return Ok(());
    }

    save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
    ui_println!("Removed {} tag(s) from '{}'.", removed, args.name);
    Ok(())
}

pub(crate) fn cmd_tag_rename(args: TagRenameCmd) -> CliResult {
    let old_key = args.old.to_lowercase();
    let new_tag = args.new;

    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load_bookmark_db(&file)?;

    let mut changed_tags = 0usize;
    let mut changed_entries = 0usize;

    for entry in db.values_mut() {
        let mut updated = false;
        for t in entry.tags.iter_mut() {
            if t.to_lowercase() == old_key {
                *t = new_tag.clone();
                updated = true;
                changed_tags += 1;
            }
        }
        if updated {
            let mut seen: HashSet<String> = HashSet::new();
            entry.tags.retain(|t| seen.insert(t.to_lowercase()));
            changed_entries += 1;
        }
    }

    if changed_tags == 0 {
        return Err(CliError::with_details(
            2,
            format!("Tag '{}' not found.", args.old),
            &["Hint: Run `xun tag list` to see existing tags."],
        ));
    }

    save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
    ui_println!(
        "Renamed tag '{}' -> '{}' ({} tags in {} bookmarks).",
        args.old,
        new_tag,
        changed_tags,
        changed_entries
    );
    Ok(())
}

pub(crate) fn cmd_tag_list(_args: TagListCmd) -> CliResult {
    let file = db_path();
    let db = load_bookmark_db(&file)?;

    let mut tags: std::collections::BTreeMap<String, (String, u32)> =
        std::collections::BTreeMap::new();
    for e in db.values() {
        for t in &e.tags {
            let key = t.to_lowercase();
            let entry = tags.entry(key).or_insert((t.clone(), 0));
            entry.1 += 1;
        }
    }

    if tags.is_empty() {
        ui_println!("No tags found.");
        return Ok(());
    }

    if !prefer_table_output() {
        for (_k, (disp, count)) in tags {
            out_println!("{}\t{}", disp, count);
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Tag")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Count")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
    ]);
    for (_k, (disp, count)) in tags {
        table.add_row(vec![
            Cell::new(disp).fg(Color::Cyan),
            Cell::new(count).fg(Color::Green),
        ]);
    }
    print_table(&table);
    Ok(())
}
