// commands/cstat/mod.rs
//
// Entry point for `xun cstat` — code statistics and project cleanup scanner.

pub(crate) mod render;

#[cfg(feature = "tui")]
pub(crate) mod tui;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use ignore::WalkBuilder;
use rayon::prelude::*;

use crate::cli::CstatCmd;
use crate::cstat::lang::{TMP_EXTENSIONS, TMP_PREFIXES, rules_for_ext};
use crate::cstat::report::{Issues, LangStat, Report, accumulate, finalize};
use crate::cstat::scanner::scan_bytes;
use crate::output::{CliError, CliResult, can_interact};

pub(crate) fn cmd_cstat(args: CstatCmd) -> CliResult {
    // 1. Collect files
    let files = collect_files(&args)?;

    if files.is_empty() {
        ui_println!("No supported files found in '{}'.", args.path);
        return Ok(());
    }

    // 2. Parallel scan
    let lang_map: Mutex<HashMap<String, LangStat>> = Mutex::new(HashMap::new());
    let issues: Mutex<Issues> = Mutex::new(Issues::default());
    let hash_map: Mutex<HashMap<[u8; 32], Vec<String>>> = Mutex::new(HashMap::new());

    let large_threshold = args.large;
    let wants_empty = args.empty || args.all;
    let wants_dup = args.dup || args.all;
    let wants_tmp = args.tmp || args.all;

    files.par_iter().for_each(|path| {
        let path_str = path.to_string_lossy().into_owned();

        // Temporary file check (no I/O needed)
        if wants_tmp && is_tmp_file(path) {
            issues.lock().unwrap().tmp.push(path_str.clone());
            return;
        }

        // Read file
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(_) => return,
        };

        // Empty file check
        if wants_empty && content.is_empty() {
            issues.lock().unwrap().empty.push(path_str.clone());
        }

        // Duplicate detection (blake3)
        if wants_dup && !content.is_empty() {
            let mut hasher = blake3::Hasher::new();
            hasher.update(&content);
            let hash = *hasher.finalize().as_bytes();
            hash_map
                .lock()
                .unwrap()
                .entry(hash)
                .or_default()
                .push(path_str.clone());
        }

        // Determine language
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let Some(rules) = rules_for_ext(ext) else {
            return;
        };

        // Scan content
        let stat = scan_bytes(&content, &rules);

        // Large file check
        if let Some(threshold) = large_threshold {
            if stat.total_lines() > threshold as u32 {
                issues
                    .lock()
                    .unwrap()
                    .large
                    .push((path_str.clone(), stat.total_lines()));
            }
        }

        // Accumulate
        let mut map = lang_map.lock().unwrap();
        accumulate(&mut map, rules.name, &stat);
    });

    // 3. Post-process
    let mut issues = issues.into_inner().unwrap();

    if wants_dup {
        let hm = hash_map.into_inner().unwrap();
        issues.dup = hm.into_values().filter(|v| v.len() > 1).collect();
        issues.dup.sort();
    }
    issues.large.sort_by(|a, b| b.1.cmp(&a.1));

    let stats = finalize(lang_map.into_inner().unwrap());
    let report = Report { stats, issues };

    // 4. Output
    render_output(&args, report)
}

// ─── Output routing ──────────────────────────────────────────────────────────

fn render_output(args: &CstatCmd, report: Report) -> CliResult {
    let fmt = args.format.to_ascii_lowercase();

    // --output: write JSON to file
    if let Some(ref out_path) = args.output {
        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| CliError::new(1, format!("JSON error: {}", e)))?;
        std::fs::write(out_path, &json)
            .map_err(|e| CliError::new(1, format!("Write error: {}", e)))?;
        ui_println!("Report written to: {}", out_path);

        // If only --output without explicit format, we're done
        if fmt == "auto" {
            return Ok(());
        }
    }

    match fmt.as_str() {
        "json" => {
            render::render_json(&report);
            Ok(())
        }
        "table" => {
            render::render_stats(&report.stats);
            render_issues_if_needed(args, &report.issues);
            Ok(())
        }
        "auto" => {
            // TUI if interactive + has issues to show, otherwise table
            #[cfg(feature = "tui")]
            {
                let has_issues = !report.issues.is_empty();
                if can_interact() && has_issues {
                    return tui::run_cstat_tui(report, &args.path);
                }
            }
            render::render_stats(&report.stats);
            render_issues_if_needed(args, &report.issues);
            Ok(())
        }
        other => Err(CliError::with_details(
            2,
            format!("Unknown format: '{}'", other),
            &["Fix: Use --format auto, table, or json."],
        )),
    }
}

fn render_issues_if_needed(args: &CstatCmd, issues: &crate::cstat::report::Issues) {
    let show = args.empty || args.all || args.large.is_some() || args.dup || args.tmp;
    if show {
        render::render_issues(issues);
    }
}

// ─── File collection ─────────────────────────────────────────────────────────

fn collect_files(args: &CstatCmd) -> CliResult<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(&args.path);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .ignore(true)
        .follow_links(false);

    if let Some(d) = args.depth {
        builder.max_depth(Some(d));
    }

    let exts: Vec<String> = args
        .ext
        .iter()
        .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
        .collect();

    let files: Vec<PathBuf> = builder
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map_or(false, |t| t.is_file()))
        .map(|e| e.into_path())
        .filter(|p| {
            if exts.is_empty() {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map_or(false, |e| rules_for_ext(e).is_some())
            } else {
                p.extension().and_then(|e| e.to_str()).map_or(false, |e| {
                    let lower = e.to_ascii_lowercase();
                    exts.iter().any(|f| f == &lower)
                })
            }
        })
        .collect();

    Ok(files)
}

// ─── Temp file detection ─────────────────────────────────────────────────────

fn is_tmp_file(path: &PathBuf) -> bool {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        if TMP_EXTENSIONS.iter().any(|&t| t == ext_lower) {
            return true;
        }
    }

    TMP_PREFIXES.iter().any(|&p| filename.starts_with(p))
}
