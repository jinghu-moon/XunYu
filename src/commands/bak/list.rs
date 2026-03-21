use std::fs;
use std::path::Path;
use std::time::SystemTime;

use comfy_table::{Attribute, Cell, Color, Table};

use crate::output::{CliResult, apply_pretty_table_style, print_table};

use super::config::BakConfig;
use super::time_fmt::fmt_unix_ts;
use super::util::dir_size;

pub(crate) fn cmd_bak_list(root: &Path, cfg: &BakConfig) -> CliResult {
    let backups_root = root.join(&cfg.storage.backups_dir);
    let _ = fs::create_dir_all(&backups_root);

    #[derive(Clone)]
    struct Item {
        name: String,
        is_zip: bool,
        mtime: u64,
        size: u64,
    }

    let mut items: Vec<Item> = Vec::new();
    if let Ok(rd) = fs::read_dir(&backups_root) {
        for e in rd.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.starts_with(&cfg.naming.prefix) {
                continue;
            }
            let meta = e.metadata().ok();
            let mtime = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let (is_zip, size) = if path.is_dir() {
                (false, dir_size(&path))
            } else if path.extension().is_some_and(|e| e == "zip") {
                (true, meta.as_ref().map(|m| m.len()).unwrap_or(0))
            } else {
                continue;
            };
            items.push(Item {
                name,
                is_zip,
                mtime,
                size,
            });
        }
    }
    items.sort_by(|a, b| a.mtime.cmp(&b.mtime).then_with(|| a.name.cmp(&b.name)));

    if items.is_empty() {
        ui_println!("No backups found: {}", backups_root.display());
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
            Cell::new(fmt_unix_ts(it.mtime)).fg(Color::Magenta),
            Cell::new(it.size).fg(Color::Green),
        ]);
    }
    print_table(&table);
    Ok(())
}
