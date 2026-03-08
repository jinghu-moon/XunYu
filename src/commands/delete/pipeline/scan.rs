use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use regex::Regex;

use crate::commands::delete::paths::volume_root;
use crate::commands::delete::progress::Progress;
use crate::commands::delete::{scanner, usn_scan, winapi};

pub(super) fn smart_scan(
    root: &Path,
    target_names: &HashSet<String>,
    match_all: bool,
    exclude_dirs: &HashSet<String>,
    patterns: &[Regex],
    progress: &Arc<Progress>,
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
