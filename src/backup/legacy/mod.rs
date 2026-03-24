pub(crate) mod baseline;
pub(crate) mod checksum;
pub(crate) mod config;
pub(crate) mod diff;
pub(crate) mod find;
pub(crate) mod list;
pub(crate) mod meta;
pub(crate) mod report;
pub(crate) mod retention;
pub(crate) mod scan;
pub(crate) mod time_fmt;
pub(crate) mod util;
pub(crate) mod verify;
pub(crate) mod version;
pub(crate) mod zip;

use std::path::Path;

pub(crate) fn bench_read_baseline_len(prev: &Path) -> usize {
    baseline::read_baseline(prev).len()
}

pub(crate) fn bench_scan_and_diff_count(current_root: &Path, prev: &Path) -> usize {
    let current = scan::scan_files(current_root, &[], &[], &[]);
    let mut baseline = baseline::read_baseline(prev);
    diff::compute_diff(&current, &mut baseline, false).len()
}
