// commands/batch_rename.rs
//
// Entry point for `xun brn` — batch file renamer.

#[cfg(feature = "tui")]
mod tui;

use comfy_table::{Cell, Color, Table};

use crate::batch_rename::collect::collect_files;
use crate::batch_rename::compute::{RenameMode, compute_ops};
use crate::batch_rename::conflict::detect_conflicts;
use crate::batch_rename::types::CaseStyle;
use crate::batch_rename::undo::{UndoRecord, run_undo, write_undo};
use crate::cli::BrnCmd;
use crate::output::{CliError, CliResult, apply_pretty_table_style, can_interact, print_table};
use crate::path_guard::{PathPolicy, validate_paths};

/// 当 `XUN_BRN_TIMING=1` 时输出各环节耗时到 stderr
#[inline]
fn timing_enabled() -> bool {
    std::env::var("XUN_BRN_TIMING").as_deref() == Ok("1")
}

macro_rules! t_print {
    ($($arg:tt)*) => {
        if timing_enabled() {
            eprintln!("[timing] {}", format_args!($($arg)*));
        }
    }
}

pub(crate) fn cmd_brn(args: BrnCmd) -> CliResult {
    // Handle `xun brn --undo`
    if args.undo {
        return cmd_brn_undo(&args.path);
    }

    let t_total = std::time::Instant::now();

    // Resolve rename mode
    let t0 = std::time::Instant::now();
    let mode = resolve_mode(&args)?;
    t_print!("resolve_mode: {:.2}ms", t0.elapsed().as_secs_f64() * 1000.0);

    let t0 = std::time::Instant::now();
    let mut policy = if args.apply {
        PathPolicy::for_write()
    } else {
        PathPolicy::for_read()
    };
    policy.allow_relative = true;
    let validation = validate_paths(vec![args.path.clone()], &policy);
    t_print!("validate_path: {:.2}ms", t0.elapsed().as_secs_f64() * 1000.0);
    if !validation.issues.is_empty() {
        let details: Vec<String> = validation
            .issues
            .iter()
            .map(|issue| format!("Invalid path: {} ({})", issue.raw, issue.detail))
            .collect();
        return Err(CliError::with_details(
            2,
            "Invalid input path.".to_string(),
            &details,
        ));
    }
    let scan_root = validation
        .ok
        .first()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| args.path.clone());

    // Collect files
    let t0 = std::time::Instant::now();
    let files = collect_files(&scan_root, &args.ext, args.recursive)?;
    t_print!("collect_files: {:.2}ms (n={})", t0.elapsed().as_secs_f64() * 1000.0, files.len());
    if files.is_empty() {
        ui_println!("No matching files found in '{}'.", scan_root);
        return Ok(());
    }

    // Compute rename operations
    let t0 = std::time::Instant::now();
    let ops = compute_ops(&files, &mode)?;
    t_print!("compute_ops: {:.2}ms (n={})", t0.elapsed().as_secs_f64() * 1000.0, ops.len());

    // Filter out no-ops (from == to)
    let t0 = std::time::Instant::now();
    let (effective_ops, noop_ops): (Vec<_>, Vec<_>) = ops.into_iter().partition(|o| o.from != o.to);
    let ops = effective_ops;
    t_print!("filter_noop: {:.2}ms (effective={} skipped={})", t0.elapsed().as_secs_f64() * 1000.0, ops.len(), noop_ops.len());

    // Warn about strip_prefix no-ops
    if matches!(mode, RenameMode::StripPrefix(_)) && !noop_ops.is_empty() {
        ui_println!("Warning: {} file(s) did not have the prefix and were skipped:", noop_ops.len());
        for op in &noop_ops {
            ui_println!("  {}", op.from.file_name().and_then(|n| n.to_str()).unwrap_or("?"));
        }
    }

    if ops.is_empty() {
        ui_println!("All files already match the target pattern. Nothing to rename.");
        return Ok(());
    }

    // Conflict detection
    let t0 = std::time::Instant::now();
    let conflicts = detect_conflicts(&ops, args.apply);
    t_print!("detect_conflicts: {:.2}ms (conflicts={})", t0.elapsed().as_secs_f64() * 1000.0, conflicts.len());
    if !conflicts.is_empty() {
        ui_println!("Conflicts detected:");
        for c in &conflicts {
            ui_println!("  {}", c);
        }
        return Err(CliError::new(1, "Resolve conflicts before renaming."));
    }

    // Route: TUI / apply / preview
    if args.apply {
        if !args.yes && can_interact() {
            preview_table(&ops, true);
            if !confirm_apply(ops.len())? {
                ui_println!("Cancelled.");
                return Ok(());
            }
        }
        let t0 = std::time::Instant::now();
        let r = apply_renames(&ops, std::path::Path::new(&scan_root));
        t_print!("apply_renames: {:.2}ms", t0.elapsed().as_secs_f64() * 1000.0);
        t_print!("cmd_brn total: {:.2}ms", t_total.elapsed().as_secs_f64() * 1000.0);
        r
    } else {
        #[cfg(feature = "tui")]
        {
            if can_interact() {
                t_print!("cmd_brn total (pre-tui): {:.2}ms", t_total.elapsed().as_secs_f64() * 1000.0);
                return tui::run_brn_tui(ops, std::path::PathBuf::from(&scan_root));
            }
        }
        let t0 = std::time::Instant::now();
        preview_table(&ops, false);
        t_print!("preview_table: {:.2}ms", t0.elapsed().as_secs_f64() * 1000.0);
        t_print!("cmd_brn total: {:.2}ms", t_total.elapsed().as_secs_f64() * 1000.0);
        Ok(())
    }
}

pub(crate) fn cmd_brn_undo(dir: &str) -> CliResult {
    run_undo(dir)
}

// ─── Mode resolution ─────────────────────────────────────────────────────────

fn resolve_mode(args: &BrnCmd) -> CliResult<RenameMode> {
    let modes: Vec<&str> = [
        args.regex.is_some().then_some("regex"),
        args.case.is_some().then_some("case"),
        args.prefix.is_some().then_some("prefix"),
        args.suffix.is_some().then_some("suffix"),
        args.strip_prefix.is_some().then_some("strip-prefix"),
        args.seq.then_some("seq"),
    ]
    .into_iter()
    .flatten()
    .collect();

    if modes.is_empty() {
        return Err(CliError::with_details(
            2,
            "No rename mode specified.",
            &["Fix: Use --regex, --case, --prefix, --suffix, --strip-prefix, or --seq."],
        ));
    }
    if modes.len() > 1 {
        return Err(CliError::new(
            2,
            format!(
                "Multiple modes active: {}. Use exactly one.",
                modes.join(", ")
            ),
        ));
    }

    if let Some(ref pattern) = args.regex {
        let replace = args.replace.as_deref().unwrap_or("");
        return Ok(RenameMode::Regex {
            pattern: pattern.clone(),
            replace: replace.to_owned(),
        });
    }
    if let Some(ref case_str) = args.case {
        let style = parse_case_style(case_str)?;
        return Ok(RenameMode::Case(style));
    }
    if let Some(ref p) = args.prefix {
        return Ok(RenameMode::Prefix(p.clone()));
    }
    if let Some(ref s) = args.suffix {
        return Ok(RenameMode::Suffix(s.clone()));
    }
    if let Some(ref s) = args.strip_prefix {
        return Ok(RenameMode::StripPrefix(s.clone()));
    }
    if args.seq {
        return Ok(RenameMode::SeqExt {
            start: args.start,
            pad: args.pad,
            prefix: false,
            only: false,
        });
    }

    unreachable!()
}

fn parse_case_style(s: &str) -> CliResult<CaseStyle> {
    match s.to_ascii_lowercase().as_str() {
        "kebab" => Ok(CaseStyle::Kebab),
        "snake" => Ok(CaseStyle::Snake),
        "pascal" => Ok(CaseStyle::Pascal),
        "upper" => Ok(CaseStyle::Upper),
        "lower" => Ok(CaseStyle::Lower),
        _ => Err(CliError::with_details(
            2,
            format!("Unknown case style: '{}'", s),
            &["Fix: Use kebab, snake, pascal, upper, or lower."],
        )),
    }
}

// ─── Preview ─────────────────────────────────────────────────────────────────

fn preview_table(ops: &[crate::batch_rename::types::RenameOp], will_apply: bool) {
    if will_apply {
        ui_println!("\nApplying {} rename(s):\n", ops.len());
    } else {
        ui_println!(
            "\nPreview ({} rename(s)) — dry-run, no changes made:\n",
            ops.len()
        );
    }

    const MAX_PREVIEW_ROWS: usize = 200;
    let display_ops = if ops.len() > MAX_PREVIEW_ROWS {
        &ops[..MAX_PREVIEW_ROWS]
    } else {
        ops
    };

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("From").fg(Color::Cyan),
        Cell::new("").fg(Color::DarkGrey),
        Cell::new("To").fg(Color::Cyan),
    ]);

    for op in display_ops {
        let from_name = filename_str(&op.from);
        let to_name = filename_str(&op.to);
        table.add_row(vec![
            Cell::new(&from_name).fg(Color::Red),
            Cell::new("→"),
            Cell::new(&to_name).fg(Color::Green),
        ]);
    }

    print_table(&table);

    if ops.len() > MAX_PREVIEW_ROWS {
        ui_println!(
            "... and {} more (showing first {} of {}). Use --apply -y to execute all.",
            ops.len() - MAX_PREVIEW_ROWS,
            MAX_PREVIEW_ROWS,
            ops.len()
        );
    }

    if !will_apply {
        ui_println!("\nRun with --apply to execute.");
    }
}

// ─── Confirmation ────────────────────────────────────────────────────────────

fn confirm_apply(count: usize) -> CliResult<bool> {
    use dialoguer::Confirm;
    let ans = Confirm::new()
        .with_prompt(format!("Rename {} file(s)?", count))
        .default(false)
        .interact()
        .map_err(|e| CliError::new(1, format!("Prompt error: {}", e)))?;
    Ok(ans)
}

// ─── Apply ───────────────────────────────────────────────────────────────────

fn apply_renames(ops: &[crate::batch_rename::types::RenameOp], scan_root: &std::path::Path) -> CliResult {
    let mut records: Vec<UndoRecord> = Vec::new();
    let mut success = 0usize;
    let mut errors = 0usize;

    for op in ops {
        match std::fs::rename(&op.from, &op.to) {
            Ok(()) => {
                success += 1;
                records.push(UndoRecord {
                    from: op.to.to_string_lossy().into_owned(),
                    to: op.from.to_string_lossy().into_owned(),
                });
                ui_println!("  OK  {} -> {}", op.from.display(), op.to.display());
            }
            Err(e) => {
                errors += 1;
                ui_println!("  ERR {} -> {}: {}", op.from.display(), op.to.display(), e);
            }
        }
    }

    if !records.is_empty() {
        write_undo(scan_root, &records)?;
    }

    ui_println!("\n{} renamed, {} failed.", success, errors);
    if !records.is_empty() {
        ui_println!("Undo: xun brn {} --undo", scan_root.display());
    }

    if errors > 0 {
        Err(CliError::new(1, format!("{} rename(s) failed.", errors)))
    } else {
        Ok(())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn filename_str(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_owned()
}
