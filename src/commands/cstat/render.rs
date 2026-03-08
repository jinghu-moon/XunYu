// commands/cstat/render.rs
//
// comfy-table rendering for cstat CLI output.

use comfy_table::{Cell, CellAlignment, Color, Table};

use crate::cstat::report::{Issues, LangStat, Report, fmt_bytes};
use crate::output::{apply_pretty_table_style, print_table};

// ─── Stats table ─────────────────────────────────────────────────────────────

pub(crate) fn render_stats(stats: &[LangStat]) {
    if stats.is_empty() {
        ui_println!("No code statistics to display.");
        return;
    }

    let total_code: u32 = stats.iter().map(|s| s.code).sum();

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Language").fg(Color::Cyan),
        Cell::new("Files").fg(Color::Cyan),
        Cell::new("Code").fg(Color::Cyan),
        Cell::new("Comment").fg(Color::Cyan),
        Cell::new("Blank").fg(Color::Cyan),
        Cell::new("Total").fg(Color::Cyan),
        Cell::new("Bytes").fg(Color::Cyan),
    ]);

    for s in stats {
        let pct = if total_code > 0 {
            format!(" ({:.0}%)", s.code as f64 / total_code as f64 * 100.0)
        } else {
            String::new()
        };

        table.add_row(vec![
            Cell::new(&s.name),
            Cell::new(s.files).set_alignment(CellAlignment::Right),
            Cell::new(format!("{}{}", s.code, pct))
                .fg(Color::Green)
                .set_alignment(CellAlignment::Right),
            Cell::new(s.comment).set_alignment(CellAlignment::Right),
            Cell::new(s.blank).set_alignment(CellAlignment::Right),
            Cell::new(s.total_lines()).set_alignment(CellAlignment::Right),
            Cell::new(fmt_bytes(s.bytes)).set_alignment(CellAlignment::Right),
        ]);
    }

    // Totals row
    let t_files: u32 = stats.iter().map(|s| s.files).sum();
    let t_comment: u32 = stats.iter().map(|s| s.comment).sum();
    let t_blank: u32 = stats.iter().map(|s| s.blank).sum();
    let t_total: u32 = stats.iter().map(|s| s.total_lines()).sum();
    let t_bytes: u64 = stats.iter().map(|s| s.bytes).sum();

    table.add_row(vec![
        Cell::new("Total").fg(Color::White),
        Cell::new(t_files)
            .fg(Color::White)
            .set_alignment(CellAlignment::Right),
        Cell::new(total_code)
            .fg(Color::Green)
            .set_alignment(CellAlignment::Right),
        Cell::new(t_comment)
            .fg(Color::White)
            .set_alignment(CellAlignment::Right),
        Cell::new(t_blank)
            .fg(Color::White)
            .set_alignment(CellAlignment::Right),
        Cell::new(t_total)
            .fg(Color::White)
            .set_alignment(CellAlignment::Right),
        Cell::new(fmt_bytes(t_bytes))
            .fg(Color::White)
            .set_alignment(CellAlignment::Right),
    ]);

    print_table(&table);
}

// ─── Issues list ─────────────────────────────────────────────────────────────

pub(crate) fn render_issues(issues: &Issues) {
    if issues.is_empty() {
        return;
    }

    ui_println!("");

    if !issues.empty.is_empty() {
        ui_println!("Empty files ({}):", issues.empty.len());
        for p in &issues.empty {
            ui_println!("  {}", p);
        }
    }

    if !issues.large.is_empty() {
        ui_println!("Large files ({}):", issues.large.len());
        for (p, lines) in &issues.large {
            ui_println!("  {} ({} lines)", p, lines);
        }
    }

    if !issues.tmp.is_empty() {
        ui_println!("Temporary files ({}):", issues.tmp.len());
        for p in &issues.tmp {
            ui_println!("  {}", p);
        }
    }

    if !issues.dup.is_empty() {
        ui_println!("Duplicate groups ({}):", issues.dup.len());
        for group in &issues.dup {
            ui_println!("  ---");
            for p in group {
                ui_println!("    {}", p);
            }
        }
    }
}

// ─── JSON output ─────────────────────────────────────────────────────────────

pub(crate) fn render_json(report: &Report) {
    match serde_json::to_string_pretty(report) {
        Ok(json) => out_println!("{}", json),
        Err(e) => ui_println!("JSON serialization error: {}", e),
    }
}
