//! `xun backup find` — 按标签/时间过滤备份

use std::path::Path;

use chrono::{DateTime, Local, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use comfy_table::{Attribute, Cell, Color, Table};
use serde::Serialize;

use crate::output::{CliError, CliResult, apply_pretty_table_style, print_table};

use super::config::BackupConfig;
use super::meta::{BackupStats, collect_backup_records};
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
        stats: BackupStats,
        size_bytes: u64,
    }

    #[derive(Serialize)]
    struct BackupFindFilters {
        tag: Option<String>,
        since: Option<u64>,
        until: Option<u64>,
    }

    #[derive(Serialize)]
    struct BackupFindResponse {
        action: String,
        count: usize,
        filters: BackupFindFilters,
        items: Vec<BackupFindItem>,
    }

    let mut results: Vec<BackupFindItem> = Vec::new();

    for record in collect_backup_records(&backups_root, &cfg.naming.prefix) {
        let Some(m) = record.meta else { continue };

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

        results.push(BackupFindItem {
            name: record.display_name,
            ts: m.ts,
            time_display: fmt_unix_ts(m.ts),
            incremental: m.incremental,
            desc: m.desc,
            tags: m.tags,
            stats: m.stats,
            size_bytes: record.size_bytes,
        });
    }

    results.sort_by(|a, b| a.ts.cmp(&b.ts).then_with(|| a.name.cmp(&b.name)));

    if json {
        let response = BackupFindResponse {
            action: "find".to_string(),
            count: results.len(),
            filters: BackupFindFilters {
                tag: tag.map(str::to_string),
                since,
                until,
            },
            items: results,
        };
        out_println!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_default()
        );
        return Ok(());
    }

    if results.is_empty() {
        ui_println!("No backups match the filter.");
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

pub(crate) fn parse_time_filter_bound(
    raw: Option<&str>,
    upper_bound: bool,
) -> Result<Option<u64>, CliError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_details(
            2,
            format!(
                "Invalid {} filter: empty value",
                if upper_bound { "until" } else { "since" }
            ),
            &[r#"Fix: Use RFC3339, YYYY-MM-DD, or YYYY-MM-DD HH:MM:SS."#],
        ));
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(Some(dt.timestamp().max(0) as u64));
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return local_naive_to_unix(dt, upper_bound).map(Some);
    }

    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let dt = if upper_bound {
            date.and_hms_opt(23, 59, 59)
        } else {
            date.and_hms_opt(0, 0, 0)
        }
        .ok_or_else(|| invalid_time_filter(raw, upper_bound))?;
        return local_naive_to_unix(dt, upper_bound).map(Some);
    }

    Err(invalid_time_filter(raw, upper_bound))
}

fn local_naive_to_unix(naive: NaiveDateTime, upper_bound: bool) -> Result<u64, CliError> {
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc).timestamp().max(0) as u64),
        LocalResult::Ambiguous(a, b) => {
            let dt = if upper_bound { b } else { a };
            Ok(dt.with_timezone(&Utc).timestamp().max(0) as u64)
        }
        LocalResult::None => Err(CliError::with_details(
            2,
            "Invalid local time value.".to_string(),
            &[r#"Fix: Use RFC3339, YYYY-MM-DD, or YYYY-MM-DD HH:MM:SS."#],
        )),
    }
}

fn invalid_time_filter(raw: &str, upper_bound: bool) -> CliError {
    CliError::with_details(
        2,
        format!(
            "Invalid {} filter: {}",
            if upper_bound { "until" } else { "since" },
            raw
        ),
        &[r#"Fix: Use RFC3339, YYYY-MM-DD, or YYYY-MM-DD HH:MM:SS."#],
    )
}

#[cfg(test)]
mod tests {
    use super::parse_time_filter_bound;

    #[test]
    fn parse_time_filter_bound_date_spans_full_day() {
        let since = parse_time_filter_bound(Some("2026-03-22"), false)
            .unwrap()
            .expect("since should parse");
        let until = parse_time_filter_bound(Some("2026-03-22"), true)
            .unwrap()
            .expect("until should parse");
        assert_eq!(until.saturating_sub(since), 86_399);
    }

    #[test]
    fn parse_time_filter_bound_accepts_rfc3339() {
        let ts = parse_time_filter_bound(Some("2026-03-22T10:00:00Z"), false)
            .unwrap()
            .expect("rfc3339 should parse");
        assert_eq!(ts, 1_774_173_600);
    }

    #[test]
    fn parse_time_filter_bound_rejects_invalid_value() {
        let err = parse_time_filter_bound(Some("not-a-time"), false).unwrap_err();
        assert!(err.message.contains("Invalid since filter"));
    }
}
