use std::path::Path;

use crate::bookmark::storage::db_path;
use crate::bookmark::undo::record_undo_batch;
use comfy_table::{Attribute, Cell, Color, Table};

use crate::bookmark::path_probe::{BookmarkPathStatus, path_status};
use crate::bookmark_core::BookmarkSource;
use crate::bookmark_state::Store;
use crate::cli::GcCmd;
use crate::model::ListFormat;
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, print_table};
use crate::store::now_secs;

use super::report::resolve_output_format;

pub(crate) fn cmd_gc(args: GcCmd) -> CliResult {
    let file = db_path();
    let mut store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;

    let dead: Vec<_> = store
        .bookmarks
        .iter()
        .filter(|bookmark| matches!(path_status(Path::new(&bookmark.path)), BookmarkPathStatus::Missing))
        .filter(|bookmark| {
            if args.learned {
                bookmark.source != BookmarkSource::Explicit
            } else {
                true
            }
        })
        .map(|bookmark| {
            (
                bookmark.id.clone(),
                bookmark.name.clone().unwrap_or_else(|| "(unnamed)".to_string()),
                bookmark.path.clone(),
            )
        })
        .collect();

    if dead.is_empty() {
        ui_println!("No dead links found.");
        return Ok(());
    }

    let format = resolve_output_format(&args.format)?;

    if args.dry_run || !args.purge {
        match format {
            ListFormat::Tsv => {
                for (id, name, path) in &dead {
                    out_println!("{id}\t{name}\t{path}");
                }
            }
            ListFormat::Json => {
                let items: Vec<_> = dead
                    .iter()
                    .map(|(id, name, path)| serde_json::json!({ "id": id, "name": name, "path": path }))
                    .collect();
                out_println!("{}", serde_json::Value::Array(items));
            }
            _ => {
                let mut table = Table::new();
                apply_pretty_table_style(&mut table);
                table.set_header(vec![
                    Cell::new("Dead Bookmark")
                        .add_attribute(Attribute::Bold)
                        .fg(Color::Red),
                    Cell::new("Missing Path")
                        .add_attribute(Attribute::Bold)
                        .fg(Color::Magenta),
                ]);
                for (_, name, path) in &dead {
                    table.add_row(vec![
                        Cell::new(name).fg(Color::Red).add_attribute(Attribute::Bold),
                        Cell::new(path).fg(Color::DarkGrey).add_attribute(Attribute::Dim),
                    ]);
                }
                print_table(&table);
            }
        }
        if args.dry_run {
            return Ok(());
        }
    }

    if args.purge {
        let before = store.clone();
        let ids: std::collections::HashSet<String> = dead.iter().map(|(id, _, _)| id.clone()).collect();
        store.bookmarks.retain(|bookmark| !ids.contains(&bookmark.id));
        store
            .save(&file, now_secs())
            .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
        let after = store.clone();
        if let Err(err) = record_undo_batch(&file, "gc", &before, &after) {
            crate::output::emit_warning(format!("Undo history not recorded: {}", err.message), &[]);
        }
        ui_println!("Purged {} dead bookmarks.", dead.len());
    }
    Ok(())
}
