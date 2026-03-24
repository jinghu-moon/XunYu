pub(crate) mod baseline;
pub(crate) mod checksum;
pub(crate) mod config;
pub(crate) mod diff;
pub(crate) mod find;
pub(crate) mod hash_cache;
pub(crate) mod hash_diff;
pub(crate) mod hash_manifest;
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BenchHashDiffStats {
    pub(crate) diff_entries: usize,
    pub(crate) total_files: usize,
    pub(crate) hash_checked_files: u64,
    pub(crate) hash_cache_hits: u64,
    pub(crate) hash_computed_files: u64,
    pub(crate) hash_failed_files: u64,
}

pub(crate) fn bench_scan_and_metadata_diff_count(
    current_root: &Path,
    prev: &Path,
    includes: &[String],
) -> usize {
    let current = scan::scan_files(current_root, includes, &[], &[]);
    let mut baseline = baseline::read_metadata_only_baseline(prev);
    diff::compute_diff(&current, &mut baseline, false).len()
}

pub(crate) fn bench_scan_and_hash_diff(
    current_root: &Path,
    prev: &Path,
    includes: &[String],
) -> BenchHashDiffStats {
    let scan_result = scan::scan_files_with_hash_details(current_root, includes, &[], &[]);
    let previous =
        hash_manifest::read_backup_snapshot_manifest(prev).expect("hash manifest should exist");
    let diff_entries = hash_diff::diff_against_hash_manifest(&scan_result.files, &previous).len();
    BenchHashDiffStats {
        diff_entries,
        total_files: scan_result.files.len(),
        hash_checked_files: scan_result.stats.hash_checked_files,
        hash_cache_hits: scan_result.stats.hash_cache_hits,
        hash_computed_files: scan_result.stats.hash_computed_files,
        hash_failed_files: scan_result.stats.hash_failed_files,
    }
}
