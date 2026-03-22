use std::fs;
use std::path::Path;

use comfy_table::{Attribute, Cell, Color, Table};
use serde::Serialize;

use crate::output::{CliResult, apply_pretty_table_style, print_table};

use super::config::BackupConfig;
use super::meta::collect_backup_records;
use super::time_fmt::fmt_unix_ts;

#[derive(Clone, Serialize)]
struct BackupListItem {
    name: String,
    is_zip: bool,
    mtime_unix: u64,
    mtime_display: String,
    size_bytes: u64,
}

pub(crate) fn cmd_backup_list(root: &Path, cfg: &BackupConfig, json: bool) -> CliResult {
    let backups_root = root.join(&cfg.storage.backups_dir);
    let _ = fs::create_dir_all(&backups_root);

    let items: Vec<BackupListItem> = collect_backup_records(&backups_root, &cfg.naming.prefix)
        .into_iter()
        .map(|record| BackupListItem {
            name: record.entry_name,
            is_zip: record.is_zip,
            mtime_unix: record.mtime,
            mtime_display: fmt_unix_ts(record.mtime),
            size_bytes: record.size_bytes,
        })
        .collect();

    if items.is_empty() {
        if json {
            out_println!("[]");
            return Ok(());
        }
        ui_println!("No backups found: {}", backups_root.display());
        return Ok(());
    }

    if json {
        out_println!("{}", serde_json::to_string_pretty(&items).unwrap_or_default());
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Name")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Type")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Mtime")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
        Cell::new("Size")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
    ]);
    for it in items {
        table.add_row(vec![
            Cell::new(it.name).fg(Color::Cyan),
            Cell::new(if it.is_zip { "zip" } else { "dir" }).fg(Color::Yellow),
            Cell::new(it.mtime_display).fg(Color::Magenta),
            Cell::new(it.size_bytes).fg(Color::Green),
        ]);
    }
    print_table(&table);
    Ok(())
}
