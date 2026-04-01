use std::path::Path;

use crate::bookmark::path_probe::{BookmarkPathStatus, path_status};
use crate::bookmark::storage::db_path;
use comfy_table::{Attribute, Cell, Color, Table};

use crate::bookmark_state::Store;
use crate::cli::CheckCmd;
use crate::model::ListFormat;
use crate::output::CliResult;
use crate::output::{apply_pretty_table_style, format_age, print_table};
use crate::store::now_secs;

use super::report::resolve_output_format;

pub(crate) fn cmd_check(args: CheckCmd) -> CliResult {
    let file = db_path();
    let store =
        Store::load_or_default(&file).map_err(|e| crate::output::CliError::new(1, format!("Failed to load store: {e}")))?;
    if store.bookmarks.is_empty() {
        ui_println!("No bookmarks found.");
        return Ok(());
    }

    let format = resolve_output_format(&args.format)?;
    let now = now_secs();
    let stale_before = now.saturating_sub(args.days.saturating_mul(24 * 60 * 60));

    let mut missing: Vec<(String, String)> = Vec::new();
    let mut stale: Vec<(String, String, u64)> = Vec::new();
    let mut by_path: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for bookmark in &store.bookmarks {
        by_path
            .entry(bookmark.path_norm.clone())
            .or_default()
            .push(bookmark.name.clone().unwrap_or_else(|| "(unnamed)".to_string()));

        if matches!(path_status(Path::new(&bookmark.path)), BookmarkPathStatus::Missing) {
            missing.push((
                bookmark.name.clone().unwrap_or_else(|| "(unnamed)".to_string()),
                bookmark.path.clone(),
            ));
        }
        if let Some(last) = bookmark.last_visited
            && last < stale_before
        {
            stale.push((
                bookmark.name.clone().unwrap_or_else(|| "(unnamed)".to_string()),
                bookmark.path.clone(),
                last,
            ));
        }
    }

    let mut duplicates: Vec<(String, Vec<String>)> =
        by_path.into_iter().filter(|(_, names)| names.len() > 1).collect();
    duplicates.sort_by(|a, b| a.0.cmp(&b.0));

    match format {
        ListFormat::Json => {
            let mut items: Vec<serde_json::Value> = Vec::new();
            for (name, path) in &missing {
                items.push(serde_json::json!({
                    "kind": "missing",
                    "name": name,
                    "path": path
                }));
            }
            for (name, path, last_visit) in &stale {
                items.push(serde_json::json!({
                    "kind": "stale",
                    "name": name,
                    "path": path,
                    "age_secs": now.saturating_sub(*last_visit)
                }));
            }
            for (path_key, names) in &duplicates {
                items.push(serde_json::json!({
                    "kind": "duplicate",
                    "path_key": path_key,
                    "names": names
                }));
            }
            out_println!("{}", serde_json::Value::Array(items));
            Ok(())
        }
        ListFormat::Tsv => {
            for (name, path) in &missing {
                out_println!("missing\t{name}\t{path}");
            }
            for (name, path, last_visit) in &stale {
                out_println!("stale\t{name}\t{path}\t{}", now.saturating_sub(*last_visit));
            }
            for (path_key, names) in &duplicates {
                out_println!("duplicate\t{}\t{}", path_key, names.join(","));
            }
            Ok(())
        }
        _ => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            table.set_header(vec![
                Cell::new("Kind").add_attribute(Attribute::Bold).fg(Color::Yellow),
                Cell::new("Name").add_attribute(Attribute::Bold).fg(Color::Cyan),
                Cell::new("Path / Key")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Magenta),
                Cell::new("Detail")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::DarkGrey),
            ]);
            for (name, path) in &missing {
                table.add_row(vec![
                    Cell::new("missing").fg(Color::Red).add_attribute(Attribute::Bold),
                    Cell::new(name).fg(Color::Cyan),
                    Cell::new(path).fg(Color::Magenta),
                    Cell::new("Fix: xun bookmark gc --dry-run / xun bookmark set")
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                ]);
            }
            for (name, path, last_visit) in &stale {
                table.add_row(vec![
                    Cell::new("stale").fg(Color::Yellow),
                    Cell::new(name).fg(Color::Cyan),
                    Cell::new(path).fg(Color::Magenta),
                    Cell::new(format!("last visited: {}", format_age(*last_visit)))
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                ]);
            }
            for (path_key, names) in &duplicates {
                table.add_row(vec![
                    Cell::new("duplicate").fg(Color::Yellow),
                    Cell::new(names.join(",")).fg(Color::Cyan),
                    Cell::new(path_key).fg(Color::Magenta),
                    Cell::new("Fix: xun bookmark dedup -m path")
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                ]);
            }

            if table.row_count() == 0 {
                ui_println!("Check OK: no issues found.");
                return Ok(());
            }
            print_table(&table);
            Ok(())
        }
    }
}
