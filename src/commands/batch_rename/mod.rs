// commands/batch_rename.rs
//
// Entry point for `xun brn` — batch file renamer.

#[cfg(feature = "tui")]
mod tui;

use comfy_table::{Cell, Color, Table};

use crate::batch_rename::collect::{SortBy, collect_dirs_depth, collect_files_depth, collect_files, sort_files_by};
use crate::batch_rename::compute::{RenameMode, ReplacePair, compute_ops, compute_ops_chain};
use crate::batch_rename::conflict::{ConflictInfo, ConflictKind, detect_conflicts};
use crate::batch_rename::output_format::{ops_to_csv, ops_to_json};
use crate::batch_rename::types::CaseStyle;
use crate::batch_rename::undo::{UndoRecord, push_undo, run_redo_steps, run_undo_steps};
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
    // Handle `xun brn --undo [N]`
    if let Some(steps) = args.undo {
        return run_undo_steps(&args.path, steps.max(1));
    }

    // Handle `xun brn --redo [N]`
    if let Some(steps) = args.redo {
        return run_redo_steps(&args.path, steps.max(1));
    }

    let t_total = std::time::Instant::now();

    // Resolve rename steps (multi-step pipeline)
    let t0 = std::time::Instant::now();
    let steps = resolve_steps(&args)?;
    t_print!("resolve_steps: {:.2}ms (n={})", t0.elapsed().as_secs_f64() * 1000.0, steps.len());

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

    // Collect files (with optional filter/exclude/depth)
    let t0 = std::time::Instant::now();
    let depth = args.depth.map(|d| if d == 0 { 1 } else { d })
        .or(if args.recursive { None } else { Some(1) });
    let mut files = if args.filter.is_some() || args.exclude.is_some() || args.depth.is_some() {
        collect_files_depth(
            &scan_root,
            &args.ext,
            depth,
            args.filter.as_deref(),
            args.exclude.as_deref(),
        )?
    } else {
        collect_files(&scan_root, &args.ext, args.recursive)?
    };
    // Optionally include directories
    if args.include_dirs {
        let dirs = collect_dirs_depth(
            &scan_root,
            depth,
            args.filter.as_deref(),
            args.exclude.as_deref(),
        )?;
        files.extend(dirs);
        files.sort_unstable();
    }
    // Apply sort order (affects sequence numbering)
    if let Some(ref sort_str) = args.sort_by {
        let by = parse_sort_by(sort_str)?;
        sort_files_by(&mut files, by);
    }
    t_print!("collect_files: {:.2}ms (n={})", t0.elapsed().as_secs_f64() * 1000.0, files.len());
    if files.is_empty() {
        ui_println!("No matching files found in '{}'.", scan_root);
        return Ok(());
    }

    // Compute rename operations (single step or multi-step chain)
    let t0 = std::time::Instant::now();
    let ops = if steps.len() == 1 {
        compute_ops(&files, &steps[0])?
    } else {
        compute_ops_chain(&files, &steps)?
    };
    t_print!("compute_ops: {:.2}ms (n={})", t0.elapsed().as_secs_f64() * 1000.0, ops.len());

    // Filter out no-ops (from == to)
    let t0 = std::time::Instant::now();
    let (effective_ops, noop_ops): (Vec<_>, Vec<_>) = ops.into_iter().partition(|o| o.from != o.to);
    let ops = effective_ops;
    t_print!("filter_noop: {:.2}ms (effective={} skipped={})", t0.elapsed().as_secs_f64() * 1000.0, ops.len(), noop_ops.len());

    // Warn about strip_prefix no-ops when strip_prefix is the only step
    if steps.len() == 1 && matches!(steps[0], RenameMode::StripPrefix(_)) && !noop_ops.is_empty() {
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
        print_conflict_table(&conflicts);
        return Err(CliError::new(1, "Resolve conflicts before renaming."));
    }

    // Parse output format
    let fmt = args.output_format.as_deref().unwrap_or("table");
    match fmt {
        "json" | "csv" | "table" => {}
        other => {
            return Err(CliError::with_details(
                2,
                format!("Unknown output format: '{}'", other),
                &["Fix: Use table, json, or csv."],
            ));
        }
    }

    // Route: TUI / apply / preview
    if args.apply {
        if !args.yes && can_interact() {
            preview_ops(&ops, true, fmt);
            if !confirm_apply(ops.len())? {
                ui_println!("Cancelled.");
                return Ok(());
            }
        }
        let t0 = std::time::Instant::now();
        let r = apply_renames(&ops, std::path::Path::new(&scan_root), fmt);
        t_print!("apply_renames: {:.2}ms", t0.elapsed().as_secs_f64() * 1000.0);
        t_print!("cmd_brn total: {:.2}ms", t_total.elapsed().as_secs_f64() * 1000.0);
        r
    } else {
        #[cfg(feature = "tui")]
        {
            if can_interact() && fmt == "table" {
                t_print!("cmd_brn total (pre-tui): {:.2}ms", t_total.elapsed().as_secs_f64() * 1000.0);
                return tui::run_brn_tui(ops, std::path::PathBuf::from(&scan_root));
            }
        }
        let t0 = std::time::Instant::now();
        preview_ops(&ops, false, fmt);
        t_print!("preview_table: {:.2}ms", t0.elapsed().as_secs_f64() * 1000.0);
        t_print!("cmd_brn total: {:.2}ms", t_total.elapsed().as_secs_f64() * 1000.0);
        Ok(())
    }
}

// ─── Step resolution ────────────────────────────────────────────────────────

/// Build the rename step pipeline from CLI args.
/// Fixed order: trim → strip_brackets → strip_prefix → strip_suffix → remove_chars
/// → replace/from-to → regex → case → ext_case → rename_ext → add_ext
/// → insert_at → slice → insert_date → normalize_seq → normalize_unicode
/// → template → prefix → suffix → seq
fn resolve_steps(args: &BrnCmd) -> CliResult<Vec<RenameMode>> {
    let mut steps: Vec<RenameMode> = Vec::new();

    // 1. Trim
    if args.trim {
        steps.push(RenameMode::Trim { chars: args.trim_chars.clone() });
    }

    // 2. Strip brackets
    if let Some(ref spec) = args.strip_brackets {
        let (mut round, mut square, mut curly) = (false, false, false);
        for part in spec.split(',') {
            match part.trim() {
                "round" | "()" => round = true,
                "square" | "[]" => square = true,
                "curly" | "{}" => curly = true,
                "all" => { round = true; square = true; curly = true; }
                other => return Err(CliError::with_details(
                    2,
                    format!("Unknown bracket type: '{}'", other),
                    &["Fix: Use round, square, curly, or all (comma-separated)."],
                )),
            }
        }
        steps.push(RenameMode::StripBrackets { round, square, curly });
    }

    // 3. Strip prefix
    if let Some(ref s) = args.strip_prefix {
        steps.push(RenameMode::StripPrefix(s.clone()));
    }

    // 4. Strip suffix
    if let Some(ref s) = args.strip_suffix {
        steps.push(RenameMode::StripSuffix(s.clone()));
    }

    // 5. Remove chars
    if let Some(ref chars) = args.remove_chars {
        steps.push(RenameMode::RemoveChars { chars: chars.clone() });
    }

    // 6. Literal replace (--from / --to)
    if let Some(ref from) = args.from {
        let to = args.to.as_deref().unwrap_or("");
        steps.push(RenameMode::Replace(vec![ReplacePair {
            from: from.clone(),
            to: to.to_owned(),
        }]));
    } else if args.to.is_some() {
        return Err(CliError::with_details(
            2,
            "--to requires --from.",
            &["Fix: Specify --from <text> --to <replacement>."],
        ));
    }

    // 7. Regex (with optional flags)
    if let Some(ref pattern) = args.regex {
        let flags = args.regex_flags.as_deref().unwrap_or("");
        // Prepend inline flags if specified: (?i), (?m), etc.
        let full_pattern = if flags.is_empty() {
            pattern.clone()
        } else {
            format!("(?{}){}", flags, pattern)
        };
        let replace = args.replace.as_deref().unwrap_or("");
        steps.push(RenameMode::Regex {
            pattern: full_pattern,
            replace: replace.to_owned(),
        });
    } else if args.replace.is_some() {
        return Err(CliError::with_details(
            2,
            "--replace requires --regex.",
            &["Fix: Specify --regex <pattern> --replace <replacement>."],
        ));
    } else if args.regex_flags.is_some() {
        return Err(CliError::with_details(
            2,
            "--regex-flags requires --regex.",
            &["Fix: Specify --regex <pattern> --regex-flags <flags>."],
        ));
    }

    // 8. Case
    if let Some(ref case_str) = args.case {
        let style = parse_case_style(case_str)?;
        steps.push(RenameMode::Case(style));
    }

    // 9. Ext case
    if let Some(ref ec) = args.ext_case {
        let style = parse_case_style(ec)?;
        steps.push(RenameMode::ExtCase(style));
    }

    // 10. Rename ext (format: old:new)
    if let Some(ref spec) = args.rename_ext {
        let (from_ext, to_ext) = parse_colon_pair(spec, "--rename-ext", "old:new (e.g. jpeg:jpg)")?;
        steps.push(RenameMode::RenameExt { from: from_ext, to: to_ext });
    }

    // 11. Add ext
    if let Some(ref ext) = args.add_ext {
        steps.push(RenameMode::AddExt { ext: ext.clone() });
    }

    // 12. Insert at (format: pos:text)
    if let Some(ref spec) = args.insert_at {
        let colon = spec.find(':').ok_or_else(|| CliError::with_details(
            2,
            "--insert-at requires format pos:text (e.g. 3:_)",
            &["Fix: Use --insert-at <position>:<text>."],
        ))?;
        let pos: usize = spec[..colon].parse().map_err(|_| CliError::with_details(
            2,
            format!("--insert-at: invalid position '{}'", &spec[..colon]),
            &["Fix: Position must be a non-negative integer."],
        ))?;
        let insert = spec[colon + 1..].to_owned();
        steps.push(RenameMode::InsertAt { pos, insert });
    }

    // 13. Slice (format: start:end, both optional)
    if let Some(ref spec) = args.slice {
        let colon = spec.find(':').ok_or_else(|| CliError::with_details(
            2,
            "--slice requires format start:end (e.g. 0:8 or -4:)",
            &["Fix: Use --slice <start>:<end> with optional negative indices."],
        ))?;
        let parse_idx = |s: &str| -> CliResult<Option<i64>> {
            if s.is_empty() { return Ok(None); }
            s.parse::<i64>().map(Some).map_err(|_| CliError::with_details(
                2,
                format!("--slice: invalid index '{}'", s),
                &["Fix: Use integers (can be negative for from-end indexing)."],
            ))
        };
        let start = parse_idx(&spec[..colon])?;
        let end = parse_idx(&spec[colon + 1..])?;
        steps.push(RenameMode::Slice { start, end });
    }

    // 14. Insert date (format: prefix|suffix:fmt)
    if let Some(ref spec) = args.insert_date {
        let colon = spec.find(':');
        let (pos_str, fmt) = match colon {
            Some(i) => (&spec[..i], spec[i + 1..].to_owned()),
            None => (spec.as_str(), "%Y%m%d".to_owned()),
        };
        let prefix = match pos_str {
            "prefix" => true,
            "suffix" | "" => false,
            other => return Err(CliError::with_details(
                2,
                format!("--insert-date: unknown position '{}'", other),
                &["Fix: Use prefix:<fmt> or suffix:<fmt> (e.g. prefix:%Y%m%d)."],
            )),
        };
        steps.push(RenameMode::InsertDate { fmt, use_ctime: args.ctime, prefix });
    } else if args.ctime {
        return Err(CliError::with_details(
            2,
            "--ctime requires --insert-date.",
            &["Fix: Specify --insert-date prefix|suffix:<fmt> --ctime."],
        ));
    }

    // 15. Normalize seq
    if let Some(pad) = args.normalize_seq {
        steps.push(RenameMode::NormalizeSeq { pad });
    }

    // 16. Normalize unicode
    if let Some(ref form) = args.normalize_unicode {
        let form_lc = form.to_ascii_lowercase();
        match form_lc.as_str() {
            "nfc" | "nfd" | "nfkc" | "nfkd" => {}
            other => return Err(CliError::with_details(
                2,
                format!("--normalize-unicode: unknown form '{}'", other),
                &["Fix: Use nfc, nfd, nfkc, or nfkd."],
            )),
        }
        steps.push(RenameMode::NormalizeUnicode { form: form_lc });
    }

    // 17. Template
    if let Some(ref tpl) = args.template {
        steps.push(RenameMode::Template {
            tpl: tpl.clone(),
            start: args.template_start,
            pad: args.template_pad,
        });
    }

    // 18. Prefix
    if let Some(ref p) = args.prefix {
        steps.push(RenameMode::Prefix(p.clone()));
    }

    // 19. Suffix
    if let Some(ref s) = args.suffix {
        steps.push(RenameMode::Suffix(s.clone()));
    }

    // 20. Seq
    if args.seq {
        steps.push(RenameMode::SeqExt {
            start: args.start,
            pad: args.pad,
            prefix: false,
            only: false,
        });
    }

    if steps.is_empty() {
        return Err(CliError::with_details(
            2,
            "No rename step specified.",
            &["Fix: Use --trim, --strip-prefix, --strip-suffix, --strip-brackets, --remove-chars, --from, --regex, --case, --ext-case, --rename-ext, --add-ext, --insert-at, --slice, --insert-date, --normalize-seq, --normalize-unicode, --template, --prefix, --suffix, or --seq."],
        ));
    }

    Ok(steps)
}

/// Parse a "key:value" pair, returning (key, value) as owned Strings.
fn parse_colon_pair(spec: &str, flag: &str, example: &str) -> CliResult<(String, String)> {
    let colon = spec.find(':').ok_or_else(|| CliError::with_details(
        2,
        format!("{} requires format {} (got '{}')", flag, example, spec),
        &[] as &[&str],
    ))?;
    Ok((spec[..colon].to_owned(), spec[colon + 1..].to_owned()))
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

fn parse_sort_by(s: &str) -> CliResult<SortBy> {
    match s.to_ascii_lowercase().as_str() {
        "name" => Ok(SortBy::Name),
        "mtime" => Ok(SortBy::Mtime),
        "ctime" => Ok(SortBy::Ctime),
        _ => Err(CliError::with_details(
            2,
            format!("Unknown sort key: '{}'", s),
            &["Fix: Use name, mtime, or ctime."],
        )),
    }
}

// ─── Preview ─────────────────────────────────────────────────────────────────

fn preview_ops(ops: &[crate::batch_rename::types::RenameOp], will_apply: bool, fmt: &str) {
    match fmt {
        "json" => {
            println!("{}", ops_to_json(ops, 0));
            return;
        }
        "csv" => {
            println!("{}", ops_to_csv(ops));
            return;
        }
        _ => {} // table (default)
    }

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

// ─── Conflict display ───────────────────────────────────────────────────────

fn print_conflict_table(conflicts: &[ConflictInfo]) {
    use comfy_table::modifiers::UTF8_ROUND_CORNERS;
    ui_println!("\nConflicts detected ({}):\n", conflicts.len());

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Type").fg(Color::Cyan),
        Cell::new("Source(s)").fg(Color::Cyan),
        Cell::new("Target").fg(Color::Cyan),
        Cell::new("Reason").fg(Color::Cyan),
    ]);

    for c in conflicts {
        let (kind_label, reason) = match c.kind {
            ConflictKind::WouldOverwrite => ("overwrite", "target already exists on disk"),
            ConflictKind::DuplicateTarget => ("duplicate", "multiple files map to same name"),
        };
        let sources_str = c.sources
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect::<Vec<_>>()
            .join("\n");
        let target_str = c.target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        table.add_row(vec![
            Cell::new(kind_label).fg(Color::Yellow),
            Cell::new(&sources_str).fg(Color::Red),
            Cell::new(target_str).fg(Color::Red),
            Cell::new(reason),
        ]);
    }

    print_table(&table);
    ui_println!("\nFix conflicts, then re-run.");
    let _ = UTF8_ROUND_CORNERS; // suppress unused import warning
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

fn apply_renames(ops: &[crate::batch_rename::types::RenameOp], scan_root: &std::path::Path, fmt: &str) -> CliResult {
    let mut records: Vec<UndoRecord> = Vec::new();
    let mut success = 0usize;
    let mut errors = 0usize;
    let mut failed_ops: Vec<String> = Vec::new();
    let mut done_ops: Vec<(&crate::batch_rename::types::RenameOp, bool)> = Vec::new();

    for op in ops {
        match std::fs::rename(&op.from, &op.to) {
            Ok(()) => {
                success += 1;
                records.push(UndoRecord {
                    from: op.to.to_string_lossy().into_owned(),
                    to: op.from.to_string_lossy().into_owned(),
                });
                done_ops.push((op, true));
                if fmt != "json" {
                    ui_println!("  OK  {} -> {}", op.from.display(), op.to.display());
                }
            }
            Err(e) => {
                errors += 1;
                failed_ops.push(format!("{}: {}", op.from.display(), e));
                done_ops.push((op, false));
                if fmt != "json" {
                    ui_println!("  ERR {} -> {}: {}", op.from.display(), op.to.display(), e);
                }
            }
        }
    }

    if !records.is_empty() {
        push_undo(scan_root, &records)?;
    }

    if fmt == "json" {
        // Structured JSON output for dashboard
        let ops_json: Vec<serde_json::Value> = done_ops.iter().map(|(op, ok)| {
            serde_json::json!({
                "from": op.from.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                "to":   op.to.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                "ok":   ok,
            })
        }).collect();
        let result = serde_json::json!({
            "total":   ops.len(),
            "success": success,
            "failed":  errors,
            "ops":     ops_json,
        });
        println!("{}", result);
    } else {
        ui_println!("\n{} renamed, {} failed.", success, errors);
        if !records.is_empty() {
            ui_println!("Undo: xun brn {} --undo", scan_root.display());
        }
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
