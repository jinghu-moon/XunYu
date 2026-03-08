use comfy_table::{Attribute, Cell, Color, Table};

use crate::model::{ListFormat, parse_list_format};
use crate::output::{
    CliError, CliResult, apply_pretty_table_style, prefer_table_output, print_table,
};

use super::deleter;
use super::file_info;
use super::types::DeleteRecord;

pub(super) fn render_results(results: &[DeleteRecord], format_raw: &str) -> CliResult {
    let mut format = parse_list_format(format_raw).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {format_raw}."),
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

    if format == ListFormat::Tsv {
        for r in results {
            let (code, _) = outcome_error(&r.outcome);
            let code_s = code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".to_string());
            let (sha, kind, size) = info_fields(r.info.as_ref());
            out_println!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                r.path.display(),
                outcome_text(&r.outcome),
                r.outcome.is_success(),
                code_s,
                sha,
                kind,
                size
            );
        }
        return Ok(());
    }

    if format == ListFormat::Json {
        let arr: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                let (code, err) = outcome_error(&r.outcome);
                let (sha, kind, size) = info_fields(r.info.as_ref());
                serde_json::json!({
                    "path": r.path.to_string_lossy().to_string(),
                    "result": outcome_text(&r.outcome),
                    "success": r.outcome.is_success(),
                    "deferred": r.outcome.is_deferred(),
                    "error_code": code,
                    "error": err,
                    "sha256": sha,
                    "kind": kind,
                    "size": size,
                    "timestamp_ms": r.ts_ms,
                })
            })
            .collect();
        out_println!("{}", serde_json::Value::Array(arr));
        return Ok(());
    }

    let show_info = results.iter().any(|r| r.info.is_some());
    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    if show_info {
        table.set_header(vec![
            Cell::new("Path")
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new("Result")
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
            Cell::new("Kind")
                .add_attribute(Attribute::Bold)
                .fg(Color::Magenta),
            Cell::new("SHA256")
                .add_attribute(Attribute::Bold)
                .fg(Color::Green),
            Cell::new("Size")
                .add_attribute(Attribute::Bold)
                .fg(Color::Blue),
        ]);
    } else {
        table.set_header(vec![
            Cell::new("Path")
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new("Result")
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
        ]);
    }

    for r in results {
        let result_cell = Cell::new(outcome_text(&r.outcome)).fg(if r.outcome.is_success() {
            Color::Green
        } else {
            Color::Red
        });
        if show_info {
            let (sha, kind, size) = info_fields(r.info.as_ref());
            table.add_row(vec![
                Cell::new(r.path.display()),
                result_cell,
                Cell::new(kind),
                Cell::new(trunc_sha(&sha)),
                Cell::new(size),
            ]);
        } else {
            table.add_row(vec![Cell::new(r.path.display()), result_cell]);
        }
    }

    print_table(&table);
    Ok(())
}

pub(super) fn print_summary(results: &[DeleteRecord]) {
    let ok = results.iter().filter(|r| r.outcome.is_success()).count();
    let fail = results.iter().filter(|r| r.outcome.is_error()).count();
    ui_println!("Summary: ok={} fail={}", ok, fail);
}

fn outcome_text(outcome: &deleter::Outcome) -> String {
    match outcome {
        deleter::Outcome::Error(code) => format!("Failed: {}", deleter::Outcome::error_desc(*code)),
        o => o.label().to_string(),
    }
}

fn outcome_error(outcome: &deleter::Outcome) -> (Option<u32>, Option<String>) {
    match outcome {
        deleter::Outcome::Error(code) => (Some(*code), Some(deleter::Outcome::error_desc(*code))),
        _ => (None, None),
    }
}

fn info_fields(info: Option<&file_info::FileInfo>) -> (String, String, String) {
    match info {
        Some(i) => (i.sha256.clone(), i.kind.to_string(), i.size.to_string()),
        None => ("-".to_string(), "-".to_string(), "-".to_string()),
    }
}

fn trunc_sha(sha: &str) -> String {
    if sha.len() > 12 {
        sha[..12].to_string()
    } else {
        sha.to_string()
    }
}

pub(super) fn write_csv(results: &[DeleteRecord], path: &str) -> CliResult {
    use std::io::Write;
    let mut file = std::fs::File::create(path)
        .map_err(|e| CliError::new(1, format!("Failed to create log file: {e}")))?;

    writeln!(
        file,
        "FilePath,Result,IsSuccess,ErrorCode,SHA256,FileType,SizeBytes,TimestampMs"
    )
    .map_err(|e| CliError::new(1, format!("Failed to write log file: {e}")))?;

    for r in results {
        let (code, _) = outcome_error(&r.outcome);
        let (sha, kind, size) = info_fields(r.info.as_ref());
        let row = [
            csv_escape(r.path.to_string_lossy().as_ref()),
            csv_escape(&outcome_text(&r.outcome)),
            r.outcome.is_success().to_string(),
            code.map(|c| c.to_string())
                .unwrap_or_else(|| "-".to_string()),
            csv_escape(&sha),
            csv_escape(&kind),
            size,
            r.ts_ms.to_string(),
        ];
        writeln!(file, "{}", row.join(","))
            .map_err(|e| CliError::new(1, format!("Failed to write log file: {e}")))?;
    }
    Ok(())
}

fn csv_escape(raw: &str) -> String {
    if raw.contains(',') || raw.contains('"') || raw.contains('\n') || raw.contains('\r') {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw.to_string()
    }
}
