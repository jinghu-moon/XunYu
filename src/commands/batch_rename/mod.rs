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

pub(crate) fn cmd_brn(args: BrnCmd) -> CliResult {
    // Handle `xun brn undo`
    if args.path == "undo" {
        return cmd_brn_undo();
    }

    // Resolve rename mode
    let mode = resolve_mode(&args)?;

    // Collect files
    let files = collect_files(&args.path, &args.ext, args.recursive)?;
    if files.is_empty() {
        ui_println!("No matching files found in '{}'.", args.path);
        return Ok(());
    }

    // Compute rename operations
    let ops = compute_ops(&files, &mode)?;

    // Filter out no-ops (from == to)
    let ops: Vec<_> = ops.into_iter().filter(|o| o.from != o.to).collect();
    if ops.is_empty() {
        ui_println!("All files already match the target pattern. Nothing to rename.");
        return Ok(());
    }

    // Conflict detection
    let conflicts = detect_conflicts(&ops);
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
        apply_renames(&ops)
    } else {
        #[cfg(feature = "tui")]
        {
            if can_interact() {
                return tui::run_brn_tui(ops);
            }
        }
        preview_table(&ops, false);
        Ok(())
    }
}

pub(crate) fn cmd_brn_undo() -> CliResult {
    run_undo()
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
        return Ok(RenameMode::Seq {
            start: args.start,
            pad: args.pad,
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

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("From").fg(Color::Cyan),
        Cell::new("").fg(Color::DarkGrey),
        Cell::new("To").fg(Color::Cyan),
    ]);

    for op in ops {
        let from_name = filename_str(&op.from);
        let to_name = filename_str(&op.to);
        table.add_row(vec![
            Cell::new(&from_name).fg(Color::Red),
            Cell::new("→"),
            Cell::new(&to_name).fg(Color::Green),
        ]);
    }

    print_table(&table);

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

fn apply_renames(ops: &[crate::batch_rename::types::RenameOp]) -> CliResult {
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
        write_undo(&records)?;
    }

    ui_println!("\n{} renamed, {} failed.", success, errors);
    if !records.is_empty() {
        ui_println!("Undo: xun brn undo");
    }

    if errors > 0 {
        Err(CliError::new(1, format!("{} rename(s) failed.", errors)))
    } else {
        Ok(())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn filename_str(path: &std::path::PathBuf) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_owned()
}
