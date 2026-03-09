use std::collections::HashMap;
use std::fs;

use crate::model::ListFormat;
use crate::output::{apply_pretty_table_style, prefer_table_output, print_table};

use comfy_table::{Attribute, Cell, Color, Table};
use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct TxSummary {
    pub(crate) tx: String,
    pub(crate) first_ts: u64,
    pub(crate) last_ts: u64,
    pub(crate) total: usize,
    pub(crate) ok: usize,
    pub(crate) fail: usize,
    pub(crate) actions: Vec<String>,
}

fn audit_path() -> std::path::PathBuf {
    crate::security::audit::audit_file_path()
}

fn parse_tx_from_params(params: &str) -> Option<String> {
    let idx = params.find("tx=")?;
    let rest = &params[(idx + 3)..];
    let end = rest.find(' ').unwrap_or(rest.len());
    let tx = rest[..end].trim();
    if tx.is_empty() {
        None
    } else {
        Some(tx.to_string())
    }
}

fn extract_tx(v: &Value) -> Option<String> {
    let params = v.get("params_json").or_else(|| v.get("params"))?;
    match params {
        Value::Object(map) => map.get("tx").and_then(Value::as_str).map(|s| s.to_string()),
        Value::String(s) => parse_tx_from_params(s),
        _ => None,
    }
}

pub(crate) fn query_tx_summaries(filter_tx: Option<&str>, last: Option<usize>) -> Vec<TxSummary> {
    let path = audit_path();
    let text = fs::read_to_string(&path).unwrap_or_default();

    let mut map: HashMap<String, TxSummary> = HashMap::new();
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let action = v.get("action").and_then(Value::as_str).unwrap_or("");
        if !action.starts_with("redirect_") {
            continue;
        }
        let Some(tx) = extract_tx(&v) else {
            continue;
        };
        if let Some(filter) = filter_tx
            && tx != filter
        {
            continue;
        }
        let ts = v.get("timestamp").and_then(Value::as_u64).unwrap_or(0);
        let result = v.get("result").and_then(Value::as_str).unwrap_or("");

        let entry = map.entry(tx.clone()).or_insert_with(|| TxSummary {
            tx,
            first_ts: ts,
            last_ts: ts,
            total: 0,
            ok: 0,
            fail: 0,
            actions: Vec::new(),
        });
        entry.total += 1;
        if result == "success" {
            entry.ok += 1;
        } else {
            entry.fail += 1;
        }
        if ts > 0 {
            entry.first_ts = entry.first_ts.min(ts);
            entry.last_ts = entry.last_ts.max(ts);
        }
        if !entry.actions.contains(&action.to_string()) {
            entry.actions.push(action.to_string());
        }
    }

    let mut out: Vec<TxSummary> = map.into_values().collect();
    out.sort_by_key(|x| std::cmp::Reverse(x.last_ts));
    if let Some(n) = last {
        out.truncate(n);
    }
    out
}

pub(crate) fn render_tx_summaries(items: &[TxSummary], mut format: ListFormat) {
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }

    match format {
        ListFormat::Tsv => {
            for it in items {
                out_println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    it.tx,
                    it.last_ts,
                    it.total,
                    it.ok,
                    it.fail,
                    it.actions.join(",")
                );
            }
        }
        ListFormat::Json => {
            let arr: Vec<serde_json::Value> = items
                .iter()
                .map(|it| {
                    serde_json::json!({
                        "tx": it.tx,
                        "first_ts": it.first_ts,
                        "last_ts": it.last_ts,
                        "total": it.total,
                        "ok": it.ok,
                        "fail": it.fail,
                        "actions": it.actions,
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(arr));
        }
        ListFormat::Table => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            table.set_header(vec![
                Cell::new("TX")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
                Cell::new("Last")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Yellow),
                Cell::new("Total")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Magenta),
                Cell::new("OK")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Green),
                Cell::new("Fail")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Red),
                Cell::new("Actions")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::DarkGrey),
            ]);
            for it in items {
                table.add_row(vec![
                    Cell::new(&it.tx).fg(Color::Cyan),
                    Cell::new(it.last_ts).fg(Color::Yellow),
                    Cell::new(it.total).fg(Color::Magenta),
                    Cell::new(it.ok).fg(Color::Green),
                    Cell::new(it.fail).fg(Color::Red),
                    Cell::new(it.actions.join(","))
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                ]);
            }
            print_table(&table);
        }
        ListFormat::Auto => unreachable!(),
    }
}
