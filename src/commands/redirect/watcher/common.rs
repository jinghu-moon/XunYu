use std::path::{Path, PathBuf};

use super::super::engine::{RedirectOptions, RedirectResult};
use super::ignore::IgnoreSet;
use super::render::render_batch;
use super::sweep::sweep_after_move;

pub(super) fn handle_results(
    source_abs: &Path,
    dest_dirs: &[PathBuf],
    ignore: &IgnoreSet,
    results: &[RedirectResult],
    opts: &RedirectOptions,
    max_sweep_dirs_per_batch: usize,
    sweep_max_depth: usize,
) -> bool {
    if results.is_empty() {
        return false;
    }

    render_batch(results, opts);
    if !opts.copy && !opts.dry_run {
        sweep_after_move(
            source_abs,
            dest_dirs,
            ignore,
            results,
            max_sweep_dirs_per_batch,
            sweep_max_depth,
        );
    }
    true
}
