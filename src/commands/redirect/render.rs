use std::collections::HashMap;
use std::path::Path;

use comfy_table::{Attribute, Cell, Color, Table};

use crate::config::RedirectProfile;
use crate::model::ListFormat;
use crate::output::{apply_pretty_table_style, print_table};
use crate::runtime;

use super::{debug_tools, engine, undo};

fn count_results(results: &[engine::RedirectResult]) -> (usize, usize, usize) {
    let mut would_apply = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    for r in results {
        if r.result == "dry_run" {
            would_apply += 1;
        } else if r.result == "skipped" {
            skipped += 1;
        } else if r.result == "failed" {
            failed += 1;
        }
    }
    (would_apply, skipped, failed)
}

pub(crate) fn render_simulate_results(
    profile: &RedirectProfile,
    names: &[String],
    format: ListFormat,
) {
    let mut rows: Vec<(String, String, String)> = Vec::new();
    for raw in names {
        let out = debug_tools::explain_one(profile, raw);
        let rule = out.matched_rule.unwrap_or_else(|| "(none)".to_string());
        let dest = out.rendered_dest_file.unwrap_or_default();
        rows.push((raw.clone(), rule, dest));
    }

    match format {
        ListFormat::Tsv => {
            for (name, rule, dest) in rows {
                out_println!("{name}\t{rule}\t{dest}");
            }
        }
        ListFormat::Json => {
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(name, rule, dest)| {
                    serde_json::json!({
                        "name": name,
                        "rule": rule,
                        "dest": dest,
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(arr));
        }
        ListFormat::Table => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            table.set_header(vec![
                Cell::new("Name")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Yellow),
                Cell::new("Rule")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
                Cell::new("Dest")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Magenta),
            ]);
            for (name, rule, dest) in rows {
                table.add_row(vec![
                    Cell::new(name).fg(Color::Yellow),
                    Cell::new(rule).fg(Color::Cyan),
                    Cell::new(dest)
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                ]);
            }
            print_table(&table);
        }
        ListFormat::Auto => unreachable!(),
    }
}

pub(crate) fn render_stats(profile: &RedirectProfile, results: &[engine::RedirectResult]) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut unmatched_skipped: usize = 0;
    for r in results {
        if r.rule.is_empty() {
            if r.result == "skipped" && r.reason == "unmatched" {
                unmatched_skipped += 1;
            }
            continue;
        }
        *counts.entry(r.rule.clone()).or_insert(0) += 1;
    }

    ui_println!("Rules coverage:");
    for rule in &profile.rules {
        let n = counts.get(&rule.name).copied().unwrap_or(0);
        if n == 0 {
            ui_println!("  {}: 0 ⚠️ unused rule", rule.name);
        } else {
            ui_println!("  {}: {n}", rule.name);
        }
    }
    let unmatched_n = counts.get("(unmatched)").copied().unwrap_or(0);
    if unmatched_n > 0 {
        ui_println!("  (unmatched): {unmatched_n}");
    }
    if unmatched_skipped > 0 {
        ui_println!("  (unmatched skipped): {unmatched_skipped}");
    }
}

pub(crate) fn render_dry_run_summary(results: &[engine::RedirectResult], copy: bool) {
    let (would_apply, skipped, failed) = count_results(results);

    let verb = if copy { "copied" } else { "moved" };
    ui_println!(
        "Dry run complete: {would_apply} file(s) would be {verb}, {skipped} skipped, {failed} failed."
    );
    ui_println!("Run without --dry-run to execute.");
}

pub(crate) fn render_preview_summary(results: &[engine::RedirectResult], copy: bool) {
    let (would_apply, skipped, failed) = count_results(results);

    let verb = if copy { "copy" } else { "move" };
    ui_println!("Preview: {would_apply} file(s) would {verb}, {skipped} skipped, {failed} failed.");
    ui_println!(
        "Note: This preview is based on the current scan; the filesystem may change before execution."
    );
}

pub(crate) fn render_results(tx: &str, results: &[engine::RedirectResult], format: ListFormat) {
    match format {
        ListFormat::Tsv => {
            for r in results {
                out_println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    r.action,
                    r.src,
                    r.dst,
                    r.rule,
                    r.result,
                    r.reason
                );
            }
        }
        ListFormat::Json => {
            let arr: Vec<serde_json::Value> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "action": r.action,
                        "src": r.src,
                        "dst": r.dst,
                        "rule": r.rule,
                        "result": r.result,
                        "reason": r.reason,
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(arr));
        }
        ListFormat::Table => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            let mut header = vec![
                Cell::new("Action")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
                Cell::new("File")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Yellow),
                Cell::new("Dest")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Magenta),
                Cell::new("Result")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Green),
            ];
            if runtime::is_verbose() {
                header.push(
                    Cell::new("Reason")
                        .add_attribute(Attribute::Bold)
                        .fg(Color::DarkGrey),
                );
            }
            table.set_header(header);
            for r in results {
                let mut row = vec![
                    Cell::new(&r.action).fg(Color::Cyan),
                    Cell::new(
                        Path::new(&r.src)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or(&r.src),
                    )
                    .fg(Color::Yellow),
                    Cell::new(Path::new(&r.dst).to_string_lossy())
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                    Cell::new(&r.result).fg(if r.result == "success" {
                        Color::Green
                    } else if r.result == "skipped" {
                        Color::DarkGrey
                    } else if r.result == "dry_run" {
                        Color::Yellow
                    } else {
                        Color::Red
                    }),
                ];
                if runtime::is_verbose() {
                    row.push(
                        Cell::new(&r.reason)
                            .fg(Color::DarkGrey)
                            .add_attribute(Attribute::Dim),
                    );
                }
                table.add_row(row);
            }
            print_table(&table);
            ui_println!("tx={}", tx);
        }
        ListFormat::Auto => unreachable!(),
    }
}

pub(crate) fn render_undo_results(tx: &str, results: &[undo::UndoResult], format: ListFormat) {
    match format {
        ListFormat::Tsv => {
            for r in results {
                out_println!(
                    "{}\t{}\t{}\t{}\t{}",
                    r.action,
                    r.src,
                    r.dst,
                    r.result,
                    r.reason
                );
            }
        }
        ListFormat::Json => {
            let arr: Vec<serde_json::Value> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "action": r.action,
                        "src": r.src,
                        "dst": r.dst,
                        "result": r.result,
                        "reason": r.reason,
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(arr));
        }
        ListFormat::Table => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            table.set_header(vec![
                Cell::new("Action")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
                Cell::new("From")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Yellow),
                Cell::new("To")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Magenta),
                Cell::new("Result")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Green),
            ]);
            for r in results {
                table.add_row(vec![
                    Cell::new(&r.action).fg(Color::Cyan),
                    Cell::new(Path::new(&r.src).to_string_lossy())
                        .fg(Color::Yellow)
                        .add_attribute(Attribute::Dim),
                    Cell::new(Path::new(&r.dst).to_string_lossy())
                        .fg(Color::Magenta)
                        .add_attribute(Attribute::Dim),
                    Cell::new(&r.result).fg(if r.result == "success" {
                        Color::Green
                    } else if r.result == "skipped" {
                        Color::DarkGrey
                    } else if r.result == "dry_run" {
                        Color::Yellow
                    } else {
                        Color::Red
                    }),
                ]);
            }
            print_table(&table);
            ui_println!("tx={}", tx);
        }
        ListFormat::Auto => unreachable!(),
    }
}
