// batch_rename/collect.rs
//
// File collection using walkdir + dunce.

use std::path::{Path, PathBuf};
use globset::{Glob, GlobMatcher};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::output::{CliError, CliResult};

pub fn collect_files(
    path: &str,
    exts: &[String],
    recursive: bool,
) -> CliResult<Vec<PathBuf>> {
    collect_files_filtered(path, exts, recursive, None, None)
}

pub fn collect_files_filtered(
    path: &str,
    exts: &[String],
    recursive: bool,
    filter: Option<&str>,
    exclude: Option<&str>,
) -> CliResult<Vec<PathBuf>> {
    let depth = if recursive { None } else { Some(1) };
    collect_files_depth(path, exts, depth, filter, exclude)
}

/// Core collector with explicit max depth.
/// `depth=None` means unlimited recursion; `depth=Some(n)` limits to n levels.
/// When depth > 1, uses rayon to walk top-level subdirectories in parallel.
pub fn collect_files_depth(
    path: &str,
    exts: &[String],
    depth: Option<usize>,
    filter: Option<&str>,
    exclude: Option<&str>,
) -> CliResult<Vec<PathBuf>> {
    let root = dunce::canonicalize(path)
        .map_err(|e| CliError::new(1, format!("Cannot access directory '{}': {}", path, e)))?;

    let max_depth = depth.unwrap_or(usize::MAX);

    let filter_matcher: Option<GlobMatcher> = filter
        .map(|pat| Glob::new(pat).map(|g| g.compile_matcher()))
        .transpose()
        .map_err(|e| CliError::new(1, format!("Invalid filter glob: {}", e)))?;

    let exclude_matcher: Option<GlobMatcher> = exclude
        .map(|pat| Glob::new(pat).map(|g| g.compile_matcher()))
        .transpose()
        .map_err(|e| CliError::new(1, format!("Invalid exclude glob: {}", e)))?;

    // For depth=1 (non-recursive), single-threaded walk is optimal.
    // For deeper walks, collect top-level subdirs and walk in parallel.
    let mut files: Vec<PathBuf> = if max_depth <= 1 {
        walk_single(&root, max_depth, exts, &filter_matcher, &exclude_matcher)
    } else {
        // Enumerate immediate children: files in root + subdirs walked in parallel
        let mut top_files = walk_single(&root, 1, exts, &filter_matcher, &exclude_matcher);

        let subdirs: Vec<PathBuf> = std::fs::read_dir(&root)
            .map(|rd| rd.filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .collect())
            .unwrap_or_default();

        let sub_files: Vec<PathBuf> = subdirs
            .par_iter()
            .flat_map(|sub| {
                let sub_depth = max_depth.saturating_sub(1);
                walk_single(sub, sub_depth, exts, &filter_matcher, &exclude_matcher)
            })
            .collect();

        top_files.extend(sub_files);
        top_files
    };

    // Restore stable sort by full path (equivalent to WalkDir sort_by_file_name across all dirs)
    files.sort_unstable();
    Ok(files)
}

/// Single-threaded WalkDir walk with filtering.
fn walk_single(
    root: &Path,
    max_depth: usize,
    exts: &[String],
    filter_matcher: &Option<GlobMatcher>,
    exclude_matcher: &Option<GlobMatcher>,
) -> Vec<PathBuf> {
    WalkDir::new(root)
        .min_depth(1)
        .max_depth(max_depth)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| ext_matches(p, exts))
        .filter(|p| glob_filter(p, filter_matcher))
        .filter(|p| !glob_exclude(p, exclude_matcher))
        .collect()
}

fn ext_matches(path: &Path, exts: &[String]) -> bool {
    if exts.is_empty() {
        return true;
    }
    let Some(file_ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    exts.iter()
        .any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(file_ext))
}

/// Returns true if the file matches the filter glob (or no filter is set).
fn glob_filter(path: &Path, matcher: &Option<GlobMatcher>) -> bool {
    match matcher {
        None => true,
        Some(m) => path.file_name()
            .map(|n| m.is_match(n))
            .unwrap_or(false),
    }
}

/// Returns true if the file matches the exclude glob (meaning it should be excluded).
fn glob_exclude(path: &Path, matcher: &Option<GlobMatcher>) -> bool {
    match matcher {
        None => false,
        Some(m) => path.file_name()
            .map(|n| m.is_match(n))
            .unwrap_or(false),
    }
}

// ─── Sorting ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub enum SortBy {
    /// Natural sort by file name (default).
    Name,
    /// Sort by modification time ascending.
    Mtime,
    /// Sort by creation time ascending (Windows ctime).
    Ctime,
}

/// Sort a file list in-place according to `by`.
/// Files whose metadata cannot be read fall back to name ordering.
pub fn sort_files_by(files: &mut [PathBuf], by: SortBy) {
    use crate::batch_rename::natural_sort::natural_cmp;
    match by {
        SortBy::Name => {
            files.sort_by(|a, b| {
                let an = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let bn = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
                natural_cmp(an, bn)
            });
        }
        SortBy::Mtime => {
            files.sort_by(|a, b| {
                let at = a.metadata().and_then(|m| m.modified()).ok();
                let bt = b.metadata().and_then(|m| m.modified()).ok();
                match (at, bt) {
                    (Some(at), Some(bt)) => at.cmp(&bt),
                    _ => {
                        let an = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        let bn = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        natural_cmp(an, bn)
                    }
                }
            });
        }
        SortBy::Ctime => {
            files.sort_by(|a, b| {
                let at = a.metadata().and_then(|m| m.created()).ok();
                let bt = b.metadata().and_then(|m| m.created()).ok();
                match (at, bt) {
                    (Some(at), Some(bt)) => at.cmp(&bt),
                    _ => {
                        let an = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        let bn = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        natural_cmp(an, bn)
                    }
                }
            });
        }
    }
}

/// Collect directories (non-recursively or up to `depth`) matching filter/exclude globs.
/// When `include_files` is false, returns only directories; when true, returns both.
pub fn collect_dirs_depth(
    path: &str,
    depth: Option<usize>,
    filter: Option<&str>,
    exclude: Option<&str>,
) -> CliResult<Vec<PathBuf>> {
    let root = dunce::canonicalize(path)
        .map_err(|e| CliError::new(1, format!("Cannot access directory '{}': {}", path, e)))?;

    let max_depth = depth.unwrap_or(1);

    let filter_matcher: Option<GlobMatcher> = filter
        .map(|pat| Glob::new(pat).map(|g| g.compile_matcher()))
        .transpose()
        .map_err(|e| CliError::new(1, format!("Invalid filter glob: {}", e)))?;

    let exclude_matcher: Option<GlobMatcher> = exclude
        .map(|pat| Glob::new(pat).map(|g| g.compile_matcher()))
        .transpose()
        .map_err(|e| CliError::new(1, format!("Invalid exclude glob: {}", e)))?;

    let mut dirs: Vec<PathBuf> = WalkDir::new(&root)
        .min_depth(1)
        .max_depth(max_depth)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .map(|e| e.into_path())
        .filter(|p| glob_filter(p, &filter_matcher))
        .filter(|p| !glob_exclude(p, &exclude_matcher))
        .collect();

    dirs.sort_unstable();
    Ok(dirs)
}
