use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::super::engine::{RedirectOptions, RedirectResult};
use super::common;
use super::ignore::IgnoreSet;
use super::options::WatchOptions;

pub(super) fn now_unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_results_and_maybe_exit(
    source_abs: &Path,
    dest_dirs: &[PathBuf],
    ignore: &IgnoreSet,
    results: &[RedirectResult],
    opts: &RedirectOptions,
    watch_opts: &WatchOptions,
    batch_count: &mut u64,
    last_scan_ts: &mut u64,
) -> bool {
    if !common::handle_results(
        source_abs,
        dest_dirs,
        ignore,
        results,
        opts,
        watch_opts.max_sweep_dirs_per_batch,
        watch_opts.sweep_max_depth,
    ) {
        return false;
    }

    *batch_count += 1;
    *last_scan_ts = now_unix_ts();
    watch_opts.should_exit(*batch_count)
}
