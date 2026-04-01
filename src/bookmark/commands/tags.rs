use crate::bookmark::undo::record_undo_batch;
use crate::bookmark::storage::db_path;
use crate::bookmark_state::Store;
use comfy_table::{Attribute, Cell, Color, Table};

use crate::cli::{TagAddCmd, TagCmd, TagListCmd, TagRemoveCmd, TagRenameCmd, TagSubCommand};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, prefer_table_output, print_table};
use crate::store::now_secs;
use crate::util::parse_tags;

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
    let mut store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let before = store.clone();
    let added = store
        .add_tags(&args.name, &tags)
        .map_err(|e| CliError::new(2, e.to_string()))?;

    if added == 0 {
        ui_println!("No new tags added.");
        return Ok(());
    }

    store
        .save(&file, now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
    let after = store.clone();
    if let Err(err) = record_undo_batch(&file, "tag:add", &before, &after) {
        crate::output::emit_warning(format!("Undo history not recorded: {}", err.message), &[]);
    }
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
    let mut store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let before = store.clone();
    let removed = store
        .remove_tags(&args.name, &tags)
        .map_err(|e| CliError::new(2, e.to_string()))?;

    if removed == 0 {
        ui_println!("No tags removed.");
        return Ok(());
    }

    store
        .save(&file, now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
    let after = store.clone();
    if let Err(err) = record_undo_batch(&file, "tag:remove", &before, &after) {
        crate::output::emit_warning(format!("Undo history not recorded: {}", err.message), &[]);
    }
    ui_println!("Removed {} tag(s) from '{}'.", removed, args.name);
    Ok(())
}

pub(crate) fn cmd_tag_rename(args: TagRenameCmd) -> CliResult {
    let old_key = args.old.to_lowercase();
    let new_tag = args.new;

    let file = db_path();
    let mut store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let before = store.clone();
    let changed_tags = store.rename_tag_globally(&old_key, &new_tag);
    let changed_entries = store
        .bookmarks
        .iter()
        .filter(|bookmark| bookmark.tags.iter().any(|tag| tag.eq_ignore_ascii_case(&new_tag)))
        .count();

    if changed_tags == 0 {
        return Err(CliError::with_details(
            2,
            format!("Tag '{}' not found.", args.old),
            &["Hint: Run `xun bookmark tag list` to see existing tags."],
        ));
    }

    store
        .save(&file, now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
    let after = store.clone();
    if let Err(err) = record_undo_batch(&file, "tag:rename", &before, &after) {
        crate::output::emit_warning(format!("Undo history not recorded: {}", err.message), &[]);
    }
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
    let store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;

    let mut tags: std::collections::BTreeMap<String, (String, u32)> =
        std::collections::BTreeMap::new();
    for e in &store.bookmarks {
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
