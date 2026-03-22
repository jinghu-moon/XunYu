//! `xun backup find` — 按标签/时间过滤备份

use std::fs;
use std::path::Path;

use comfy_table::{Attribute, Cell, Color, Table};
use serde::Serialize;

use crate::output::{CliResult, apply_pretty_table_style, print_table};

use super::config::BackupConfig;
use super::meta::read_meta;
use super::time_fmt::fmt_unix_ts;

pub(crate) fn cmd_backup_find(
    root: &Path,
    cfg: &BackupConfig,
    tag: Option<&str>,
    since: Option<u64>,
    until: Option<u64>,
    json: bool,
) -> CliResult {
    let backups_root = root.join(&cfg.storage.backups_dir);

    #[derive(Serialize)]
    struct BackupFindItem {
        name: String,
        ts: u64,
        time_display: String,
        incremental: bool,
        desc: String,
        tags: Vec<String>,
        stats: super::meta::BackupStats,
    }

    let mut results: Vec<BackupFindItem> = Vec::new();

    if let Ok(rd) = fs::read_dir(&backups_root) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.starts_with(&cfg.naming.prefix) {
                continue;
            }
            // 跳过 .zip/.meta.json 伴随文件，只处理目录和 zip
            if name.ends_with(".meta.json") {
                continue;
            }

            let backup_path = e.path();
            // 目录备份：读 .bak-meta.json
            let meta = if backup_path.is_dir() {
                read_meta(&backup_path)
            } else if name.ends_with(".zip") {
                // zip 备份：尝试读旁边的 .meta.json
                let stem = name.strip_suffix(".zip").unwrap_or(&name);
                read_meta(&backups_root.join(format!("{stem}.meta.json")))
            } else {
                continue;
            };

            let Some(m) = meta else { continue };

            // 时间过滤
            if let Some(s) = since
                && m.ts < s
            {
                continue;
            }
            if let Some(u) = until
                && m.ts > u
            {
                continue;
            }

            // 标签过滤
            if let Some(t) = tag
                && !m.tags.iter().any(|tag| tag == t)
            {
                continue;
            }

            let display_name = if name.ends_with(".zip") {
                name.strip_suffix(".zip").unwrap_or(&name).to_string()
            } else {
                name
            };
            results.push(BackupFindItem {
                name: display_name,
                ts: m.ts,
                time_display: fmt_unix_ts(m.ts),
                incremental: m.incremental,
                desc: m.desc,
                tags: m.tags,
                stats: m.stats,
            });
        }
    }

    results.sort_by(|a, b| a.ts.cmp(&b.ts).then_with(|| a.name.cmp(&b.name)));

    if results.is_empty() {
        if json {
            out_println!("[]");
            return Ok(());
        }
        ui_println!("No backups match the filter.");
        return Ok(());
    }

    if json {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Name")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Time")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
        Cell::new("Type")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Desc")
            .add_attribute(Attribute::Bold)
            .fg(Color::White),
    ]);
    for item in results {
        table.add_row(vec![
            Cell::new(&item.name).fg(Color::Cyan),
            Cell::new(item.time_display).fg(Color::Magenta),
            Cell::new(if item.incremental { "incr" } else { "full" }).fg(Color::Yellow),
            Cell::new(&item.desc).fg(Color::White),
        ]);
    }
    print_table(&table);
    Ok(())
}
