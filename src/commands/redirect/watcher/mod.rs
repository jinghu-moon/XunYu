mod batch;
mod common;
mod dir_watch;
mod file_ready;
mod ignore;
mod options;
mod reload;
mod render;
mod retry;
mod status;
mod sweep;

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use crate::config::RedirectProfile;
use crate::output::{CliError, CliResult};

use super::engine;
use super::engine::RedirectOptions;
use super::matcher;
use super::watch_core;

use batch::{apply_results_and_maybe_exit, now_unix_ts};
use dir_watch::{DirectoryWatcher, WatchSignal};
use file_ready::{FileReady, file_ready};
use ignore::{build_ignore_set, resolve_dest_dirs, should_ignore};
use options::WatchOptions;
use reload::maybe_reload_profile;
use retry::RetryQueue;
use status::{WatchStatus, WatchStatusWriter};

pub(crate) use status::{read_watch_status, render_watch_status};

pub(crate) fn watch_loop(
    tx: &str,
    source: &Path,
    profile_name: &str,
    mut profile: RedirectProfile,
    opts: &RedirectOptions,
) -> CliResult {
    let source_abs = source
        .canonicalize()
        .unwrap_or_else(|_| source.to_path_buf());

    let watch_opts = WatchOptions::from_env();
    let mut ignore = build_ignore_set(&source_abs, &profile);
    let mut dest_dirs = resolve_dest_dirs(&source_abs, &profile);

    let cfg_path = crate::config::config_path();
    let mut last_cfg_mtime = std::fs::metadata(&cfg_path)
        .ok()
        .and_then(|m| m.modified().ok());

    ui_println!("redirect watch: watching {}", source_abs.display());
    ui_println!(
        "  Profile: {} ({} rules, on_conflict={})",
        profile_name,
        profile.rules.len(),
        profile.on_conflict
    );
    ui_println!(
        "  Buffer: {}KB | Debounce: {}ms | Settle: {}ms | Retry: {}ms",
        watch_opts.buffer_len / 1024,
        watch_opts.debounce_ms,
        watch_opts.settle_ms,
        watch_opts.retry_ms
    );
    ui_println!("  Press Ctrl+C to stop.");

    let (event_tx, event_rx) = mpsc::channel::<WatchSignal>();

    let watcher = match DirectoryWatcher::new(&source_abs, watch_opts.buffer_len) {
        Ok(w) => w,
        Err(e) => {
            return Err(CliError::new(
                1,
                format!("redirect watch: failed to start watcher: {e}"),
            ));
        }
    };
    std::thread::spawn(move || watcher.run(event_tx));

    let start = Instant::now();
    let mut debouncer = watch_core::Debouncer::new(watch_opts.debounce_ms);
    let mut retry = RetryQueue::default();
    let mut batch_count: u64 = 0;
    let mut last_scan = Instant::now();
    let mut events_processed: u64 = 0;
    let errors: u64 = 0;
    let started_ts = now_unix_ts();
    let mut last_scan_ts = started_ts;

    let mut status_writer =
        WatchStatusWriter::new(&source_abs, tx, profile_name, watch_opts.buffer_len);
    status_writer.flush(&WatchStatus {
        pid: std::process::id(),
        tx: tx.to_string(),
        profile: profile_name.to_string(),
        source: source_abs.to_string_lossy().to_string(),
        started_ts,
        last_scan_ts,
        batches: batch_count,
        events_processed,
        retry_queue: Vec::new(),
        errors,
    });

    loop {
        loop {
            match event_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(sig) => match sig {
                    WatchSignal::Overflow => {
                        ui_println!("redirect watch: overflow, running full scan");
                        let results = engine::run_redirect(tx, &source_abs, &profile, opts);
                        if apply_results_and_maybe_exit(
                            &source_abs,
                            &dest_dirs,
                            &ignore,
                            &results,
                            opts,
                            &watch_opts,
                            &mut batch_count,
                            &mut last_scan_ts,
                        ) {
                            return Ok(());
                        }
                    }
                    WatchSignal::Paths(paths) => {
                        let now_ms = start.elapsed().as_millis() as u64;
                        events_processed = events_processed.saturating_add(paths.len() as u64);
                        for p in paths {
                            debouncer.push(now_ms, &p.to_string_lossy());
                        }
                    }
                },
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }

        maybe_reload_profile(
            &cfg_path,
            &mut last_cfg_mtime,
            &source_abs,
            profile_name,
            &mut profile,
            &mut ignore,
            &mut dest_dirs,
        );

        let now_ms = start.elapsed().as_millis() as u64;
        let due: Vec<PathBuf> = debouncer
            .flush_due(now_ms, watch_opts.max_paths_per_batch)
            .into_iter()
            .map(PathBuf::from)
            .collect();

        if !due.is_empty() {
            let unmatched_is_skip =
                matches!(profile.unmatched, crate::config::RedirectUnmatched::Skip);
            let mut ready = Vec::new();
            for p in due {
                if should_ignore(&source_abs, &dest_dirs, &ignore, &p) {
                    continue;
                }
                if unmatched_is_skip
                    && let Some(name) = p.file_name().and_then(|s| s.to_str())
                    && !matcher::any_rule_matches_name_only(name, &profile.rules)
                {
                    continue;
                }
                if !p.exists() {
                    continue;
                }
                if !p.is_file() {
                    continue;
                }
                match file_ready(&p, watch_opts.settle_ms) {
                    Ok(FileReady::Ready) => ready.push(p),
                    Ok(FileReady::NotReady(reason)) => retry.push(p, reason),
                    Err(e) => retry.push(p, format!("io_error:{}", e.kind())),
                }
            }

            if !ready.is_empty() {
                let results =
                    engine::run_redirect_on_paths(tx, &source_abs, &profile, opts, &ready);
                if apply_results_and_maybe_exit(
                    &source_abs,
                    &dest_dirs,
                    &ignore,
                    &results,
                    opts,
                    &watch_opts,
                    &mut batch_count,
                    &mut last_scan_ts,
                ) {
                    return Ok(());
                }
            }
        }

        if let Some(due) = retry.pop_due(watch_opts.retry_ms, watch_opts.max_retry_paths_per_batch)
        {
            let unmatched_is_skip =
                matches!(profile.unmatched, crate::config::RedirectUnmatched::Skip);
            let mut ready = Vec::new();
            for item in due {
                if should_ignore(&source_abs, &dest_dirs, &ignore, &item.path) {
                    continue;
                }
                if unmatched_is_skip
                    && let Some(name) = item.path.file_name().and_then(|s| s.to_str())
                    && !matcher::any_rule_matches_name_only(name, &profile.rules)
                {
                    continue;
                }
                if !item.path.exists() {
                    continue;
                }
                match file_ready(&item.path, watch_opts.settle_ms) {
                    Ok(FileReady::Ready) => ready.push(item.path),
                    Ok(FileReady::NotReady(reason)) => retry.push(item.path, reason),
                    Err(_) => retry.push(item.path, item.reason),
                }
            }
            if !ready.is_empty() {
                let results =
                    engine::run_redirect_on_paths(tx, &source_abs, &profile, opts, &ready);
                if apply_results_and_maybe_exit(
                    &source_abs,
                    &dest_dirs,
                    &ignore,
                    &results,
                    opts,
                    &watch_opts,
                    &mut batch_count,
                    &mut last_scan_ts,
                ) {
                    return Ok(());
                }
            }
        }

        if last_scan.elapsed().as_millis() as u64 >= watch_opts.scan_recheck_ms {
            last_scan = Instant::now();
            // If the watcher thread missed events without triggering overflow, do a lightweight top-level scan.
            let results = engine::run_redirect(tx, &source_abs, &profile, opts);
            if apply_results_and_maybe_exit(
                &source_abs,
                &dest_dirs,
                &ignore,
                &results,
                opts,
                &watch_opts,
                &mut batch_count,
                &mut last_scan_ts,
            ) {
                return Ok(());
            }
        }

        status_writer.maybe_flush(&WatchStatus {
            pid: std::process::id(),
            tx: tx.to_string(),
            profile: profile_name.to_string(),
            source: source_abs.to_string_lossy().to_string(),
            started_ts,
            last_scan_ts,
            batches: batch_count,
            events_processed,
            retry_queue: retry.sample_paths(16),
            errors,
        });

        std::thread::sleep(Duration::from_millis(20));
    }
}
