// batch_rename/collect.rs
//
// File collection using walkdir + dunce.

use std::path::PathBuf;
use walkdir::WalkDir;

use crate::output::{CliError, CliResult};

pub(crate) fn collect_files(
    path: &str,
    exts: &[String],
    recursive: bool,
) -> CliResult<Vec<PathBuf>> {
    let root = dunce::canonicalize(path)
        .map_err(|e| CliError::new(1, format!("Cannot access directory '{}': {}", path, e)))?;

    let max_depth = if recursive { usize::MAX } else { 1 };

    let mut files: Vec<PathBuf> = WalkDir::new(&root)
        .min_depth(1)
        .max_depth(max_depth)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| ext_matches(p, exts))
        .collect();

    files.sort();
    Ok(files)
}

fn ext_matches(path: &PathBuf, exts: &[String]) -> bool {
    if exts.is_empty() {
        return true;
    }
    let Some(file_ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    exts.iter()
        .any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(file_ext))
}
