use comfy_table::{Attribute, Cell, Color, Table};

use crate::model::{ListFormat, parse_list_format};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, prefer_table_output, print_table};

use super::state::ToolStatus;

pub(super) fn resolve_format(raw: &str) -> Result<ListFormat, CliError> {
    let mut format = parse_list_format(raw).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", raw),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }
    Ok(format)
}

pub(super) fn render_detect(format: ListFormat, enabled: bool, url: &str) -> CliResult {
    if format == ListFormat::Json {
        let obj = serde_json::json!({
            "enabled": enabled,
            "url": if enabled { url.to_string() } else { String::new() }
        });
        out_println!("{}", obj);
        return Ok(());
    }
    if format == ListFormat::Tsv {
        if enabled {
            out_println!("enabled\t{}", url);
        } else {
            out_println!("disabled\t");
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Status")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Address")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
    ]);
    table.add_row(vec![
        Cell::new(if enabled { "ENABLED" } else { "DISABLED" }).fg(if enabled {
            Color::Green
        } else {
            Color::DarkGrey
        }),
        Cell::new(if enabled { url } else { "-" }).fg(Color::DarkGrey),
    ]);
    print_table(&table);
    Ok(())
}

pub(super) fn render_status(format: ListFormat, rows: &[ToolStatus]) -> CliResult {
    if format == ListFormat::Json {
        let rows_json: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "tool": r.tool,
                    "status": if r.enabled { "ON" } else { "OFF" },
                    "address": r.address,
                    "note": r.note
                })
            })
            .collect();
        out_println!("{}", serde_json::Value::Array(rows_json));
        return Ok(());
    }
    if format == ListFormat::Tsv {
        for r in rows {
            out_println!(
                "{}\t{}\t{}\t{}",
                r.tool,
                if r.enabled { "ON" } else { "OFF" },
                r.address,
                r.note
            );
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Tool")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Status")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Address")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
        Cell::new("Note")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
    ]);

    for r in rows {
        table.add_row(vec![
            Cell::new(r.tool),
            Cell::new(if r.enabled { "ON" } else { "OFF" }).fg(if r.enabled {
                Color::Green
            } else {
                Color::DarkGrey
            }),
            Cell::new(r.address.clone())
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim),
            Cell::new(r.note.clone())
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim),
        ]);
    }

    print_table(&table);
    Ok(())
}

pub(super) fn render_proxy_tests(results: Vec<(String, Result<u64, String>)>) {
    let mut t = Table::new();
    apply_pretty_table_style(&mut t);
    t.set_header(vec![
        Cell::new("Target")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Latency")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Detail")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
    ]);
    for (label, result) in results {
        match result {
            Ok(ms) => {
                t.add_row(vec![
                    Cell::new(label),
                    Cell::new(format!("{}ms", ms)).fg(Color::Green),
                    Cell::new("ok")
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                ]);
            }
            Err(e) => {
                t.add_row(vec![
                    Cell::new(label).fg(Color::Red),
                    Cell::new("-").fg(Color::Red),
                    Cell::new(e).fg(Color::Red),
                ]);
            }
        }
    }
    print_table(&t);
}
