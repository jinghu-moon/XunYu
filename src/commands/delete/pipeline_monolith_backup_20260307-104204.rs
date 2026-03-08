use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;
use regex::Regex;

#[cfg(feature = "protect")]
use crate::output::emit_warning;
use crate::output::{CliResult, can_interact};

use super::deleter;
use super::file_info;
use super::paths::volume_root;
use super::progress;
use super::scanner;
use super::types::{DeleteOptions, DeleteRecord};
use super::usn_scan;
use super::winapi;

pub(super) fn run_cli_pipeline(
    root: &Path,
    target_names: &HashSet<String>,
    match_all: bool,
    exclude_dirs: &HashSet<String>,
    patterns: &[Regex],
    opts: &DeleteOptions,
) -> CliResult<Vec<DeleteRecord>> {
    ui_println!("Scanning: {}", root.display());

    let start = std::time::Instant::now();
    let progress = std::sync::Arc::new(progress::Progress::default());
    let done = std::sync::Arc::new(AtomicBool::new(false));
    let show_progress = can_interact();
    let progress_thread = if show_progress {
        let progress = progress.clone();
        let done = done.clone();
        Some(std::thread::spawn(move || {
            use std::io::Write;
            while !done.load(Ordering::Relaxed) {
                eprint!(
                    "\rIn progress... found={} processed={} ok={} fail={}   ",
                    progress.scanned(),
                    progress.processed(),
                    progress.succeeded(),
                    progress.failed()
                );
                let _ = std::io::stderr().flush();
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            eprint!(
                "\rIn progress... found={} processed={} ok={} fail={}   \n",
                progress.scanned(),
                progress.processed(),
                progress.succeeded(),
                progress.failed()
            );
            let _ = std::io::stderr().flush();
        }))
    } else {
        None
    };

    let files = smart_scan(
        root,
        target_names,
        match_all,
        exclude_dirs,
        patterns,
        &progress,
    );
    if files.is_empty() {
        done.store(true, Ordering::Relaxed);
        if let Some(handle) = progress_thread {
            let _ = handle.join();
        }
        ui_println!(
            "Completed: {} ms (no matching files).",
            start.elapsed().as_millis()
        );
        ui_println!("No matching files found in {}", root.display());
        return Ok(Vec::new());
    }

    let results = delete_paths(files, opts, Some(progress.as_ref()));
    done.store(true, Ordering::Relaxed);
    if let Some(handle) = progress_thread {
        let _ = handle.join();
    }
    ui_println!("Completed: {} ms.", start.elapsed().as_millis());

    Ok(results)
}

fn smart_scan(
    root: &Path,
    target_names: &HashSet<String>,
    match_all: bool,
    exclude_dirs: &HashSet<String>,
    patterns: &[Regex],
    progress: &std::sync::Arc<progress::Progress>,
) -> Vec<PathBuf> {
    if !match_all && target_names.is_empty() {
        return Vec::new();
    }

    let volume = volume_root(root);
    let use_usn = winapi::is_elevated() && usn_scan::is_ntfs(&volume);
    if use_usn {
        let mut results =
            usn_scan::scan_volume(&volume, target_names, match_all, exclude_dirs, progress);
        results.retain(|p| p.starts_with(root));
        if !patterns.is_empty() {
            results.retain(|p| !scanner::matches_any(p.to_string_lossy().as_ref(), patterns));
        }
        return results;
    }

    let (tx, rx) = crossbeam_channel::unbounded::<PathBuf>();
    scanner::scan_tree(
        root.to_path_buf(),
        target_names,
        match_all,
        exclude_dirs,
        patterns,
        &tx,
        progress,
    );
    drop(tx);
    rx.into_iter().collect()
}

pub(super) fn delete_paths(
    paths: Vec<PathBuf>,
    opts: &DeleteOptions,
    progress: Option<&progress::Progress>,
) -> Vec<DeleteRecord> {
    if paths.is_empty() {
        return Vec::new();
    }
    let snapshot = winapi::handle_snapshot();
    paths
        .into_par_iter()
        .filter_map(|path| {
            if crate::windows::ctrlc::is_cancelled() {
                return None;
            }

            let info = if opts.collect_info {
                file_info::collect(&path)
            } else {
                None
            };

            #[cfg(feature = "protect")]
            if let Err(msg) = crate::protect::check_protection(
                &path,
                "delete",
                opts.force,
                opts.reason.as_deref(),
            ) {
                emit_warning(format!("Protection check failed: {msg}"), &[]);
                return Some(DeleteRecord::new(path, deleter::Outcome::Error(5), info));
            }

            let path_str = path.to_string_lossy();
            let mut outcome = if opts.dry_run {
                deleter::Outcome::WhatIf
            } else {
                deleter::try_delete_from_level(path_str.as_ref(), opts.level, snapshot)
            };

            if matches!(outcome, deleter::Outcome::Error(_)) && opts.on_reboot && !opts.dry_run {
                outcome = deleter::try_delete_from_level(path_str.as_ref(), 6, snapshot);
            }

            if let Some(p) = progress {
                p.inc_processed();
                if outcome.is_success() {
                    p.inc_succeeded();
                } else if outcome.is_error() {
                    p.inc_failed();
                }
            }

            Some(DeleteRecord::new(path, outcome, info))
        })
        .collect()
}
