use std::path::Path;

use comfy_table::{Attribute, Cell, Color, Table};

use crate::cli::CheckCmd;
use crate::model::ListFormat;
use crate::output::CliResult;
use crate::output::{apply_pretty_table_style, format_age, print_table};
use crate::store::{db_path, load, now_secs};
use crate::util::normalize_path;

use super::report::resolve_output_format;

pub(crate) fn cmd_check(args: CheckCmd) -> CliResult {
    let file = db_path();
    let db = load(&file);
    if db.is_empty() {
        ui_println!("No bookmarks found.");
        return Ok(());
    }

    let format = resolve_output_format(&args.format)?;

    let now = now_secs();
    let stale_secs = args.days.saturating_mul(24 * 60 * 60);
    let stale_before = now.saturating_sub(stale_secs);

    let mut missing: Vec<(String, String)> = Vec::new();
    let mut stale: Vec<(String, String, u64)> = Vec::new(); // (name, path, last_visited)
    let mut by_path: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (name, e) in &db {
        let key = normalize_path(&e.path);
        by_path.entry(key).or_default().push(name.clone());

        if !Path::new(&e.path).exists() {
            missing.push((name.clone(), e.path.clone()));
        }
        if e.last_visited > 0 && e.last_visited < stale_before {
            stale.push((name.clone(), e.path.clone(), e.last_visited));
        }
    }

    let mut duplicates: Vec<(String, Vec<String>)> =
        by_path.into_iter().filter(|(_, v)| v.len() > 1).collect();
    duplicates.sort_by(|a, b| a.0.cmp(&b.0));

    if format == ListFormat::Json {
        let mut items: Vec<serde_json::Value> = Vec::new();
        for (name, path) in &missing {
            items.push(serde_json::json!({
                "kind": "missing",
                "name": name,
                "path": path,
                "fix": "Run `xun delete -bm <name>` or update the bookmark with `xun set <name> <path>`."
            }));
        }
        for (name, path, last_visit) in &stale {
            items.push(serde_json::json!({
                "kind": "stale",
                "name": name,
                "path": path,
                "age_secs": now.saturating_sub(*last_visit),
                "fix": "Run `xun touch <name>` (or use `xun z <pattern>`)."
            }));
        }
        for (path_key, names) in &duplicates {
            items.push(serde_json::json!({
                "kind": "duplicate",
                "path_key": path_key,
                "names": names,
                "fix": "Run `xun dedup -m path` to remove duplicates."
            }));
        }
        out_println!("{}", serde_json::Value::Array(items));
        return Ok(());
    }

    if format == ListFormat::Tsv {
        for (name, path) in &missing {
            out_println!("missing\t{name}\t{path}");
        }
        for (name, path, last_visit) in &stale {
            out_println!("stale\t{name}\t{path}\t{}", now.saturating_sub(*last_visit));
        }
        for (path_key, names) in &duplicates {
            out_println!("duplicate\t{}\t{}", path_key, names.join(","));
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Kind")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Name")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Path / Key")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
        Cell::new("Detail")
            .add_attribute(Attribute::Bold)
            .fg(Color::DarkGrey),
    ]);

    for (name, path) in &missing {
        table.add_row(vec![
            Cell::new("missing")
                .fg(Color::Red)
                .add_attribute(Attribute::Bold),
            Cell::new(name).fg(Color::Cyan),
            Cell::new(path).fg(Color::Magenta),
            Cell::new("Fix: xun delete -bm <name> or xun set <name> <path>")
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
            Cell::new(names.join(","))
                .fg(Color::Cyan)
                .add_attribute(Attribute::Dim),
            Cell::new(path_key).fg(Color::Magenta),
            Cell::new("Fix: xun dedup -m path")
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
