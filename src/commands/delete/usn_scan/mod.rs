mod common;
mod fs;
mod path;
mod scan;

use std::collections::HashSet;
use std::path::PathBuf;

use super::progress::Progress;

pub(crate) fn scan_volume(
    volume_root: &str,
    target_names: &HashSet<String>,
    match_all: bool,
    exclude_dirs: &HashSet<String>,
    progress: &Progress,
) -> Vec<PathBuf> {
    scan::scan_volume(volume_root, target_names, match_all, exclude_dirs, progress)
}

pub(crate) fn is_ntfs(root: &str) -> bool {
    fs::is_ntfs(root)
}
