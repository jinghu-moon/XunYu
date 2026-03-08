mod common;
#[cfg(not(windows))]
mod dir_std;
#[cfg(windows)]
mod dir_windows;
mod parallel;
mod single;

use std::thread;

use crate::output::CliResult;

use super::filters::FindFilters;
#[cfg(windows)]
use super::mft;
use super::rules::CompiledRules;

pub(crate) use common::{ScanItem, ScanOutput};
use parallel::scan_parallel;
use single::scan_single_thread;

pub(crate) fn scan(
    base_dirs: &[String],
    rules: &CompiledRules,
    filters: &FindFilters,
    force_meta: bool,
) -> CliResult<Vec<ScanItem>> {
    let output = scan_internal(base_dirs, rules, filters, force_meta, false)?;
    Ok(output.items)
}

pub(crate) fn scan_count(
    base_dirs: &[String],
    rules: &CompiledRules,
    filters: &FindFilters,
    force_meta: bool,
) -> CliResult<usize> {
    let output = scan_internal(base_dirs, rules, filters, force_meta, true)?;
    Ok(output.count)
}

fn scan_internal(
    base_dirs: &[String],
    rules: &CompiledRules,
    filters: &FindFilters,
    force_meta: bool,
    count_only: bool,
) -> CliResult<ScanOutput> {
    #[cfg(windows)]
    {
        if let Some(output) = mft::try_scan_mft(base_dirs, rules, filters, force_meta, count_only)?
        {
            return Ok(output);
        }
    }
    let threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    if threads <= 1 {
        return scan_single_thread(base_dirs, rules, filters, force_meta, count_only);
    }
    scan_parallel(base_dirs, rules, filters, force_meta, count_only, threads)
}
