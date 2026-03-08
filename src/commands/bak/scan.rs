use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::util::{matches_patterns, normalize_glob_path};

use super::util::norm;

pub(crate) fn scan_files(
    root: &Path,
    includes: &[String],
    exclude_patterns: &[String],
    include_patterns: &[String],
) -> HashMap<String, PathBuf> {
    let mut files = HashMap::new();

    fn walk(
        dir: &Path,
        root: &Path,
        exclude_patterns: &[String],
        include_patterns: &[String],
        files: &mut HashMap<String, PathBuf>,
    ) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let ft = match e.file_type() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let path = e.path();
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let rel_norm = normalize_glob_path(&rel.to_string_lossy());
            let name = e.file_name().to_string_lossy().into_owned();
            let name_lower = name.to_lowercase();
            let is_dir = ft.is_dir();

            if !include_patterns.is_empty()
                && matches_patterns(&rel_norm, &name_lower, include_patterns, is_dir)
            {
                // keep
            } else if matches_patterns(&rel_norm, &name_lower, exclude_patterns, is_dir) {
                continue;
            }

            if is_dir {
                walk(&path, root, exclude_patterns, include_patterns, files);
            } else {
                files.insert(rel_norm.replace('/', "\\"), path);
            }
        }
    }

    if includes.is_empty() {
        walk(root, root, exclude_patterns, include_patterns, &mut files);
    }
    for inc in includes {
        let full = root.join(inc);
        if full.is_file() {
            files.insert(norm(inc), full);
        } else if full.is_dir() {
            walk(&full, root, exclude_patterns, include_patterns, &mut files);
        }
    }
    files
}
