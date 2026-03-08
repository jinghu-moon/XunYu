use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use regex::Regex;

use crate::output::{CliResult, can_interact};

use super::progress;
use super::types::{DeleteOptions, DeleteRecord};

mod execute;
mod scan;

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

    let files = scan::smart_scan(
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

pub(super) fn delete_paths(
    paths: Vec<std::path::PathBuf>,
    opts: &DeleteOptions,
    progress: Option<&progress::Progress>,
) -> Vec<DeleteRecord> {
    execute::delete_paths(paths, opts, progress)
}
