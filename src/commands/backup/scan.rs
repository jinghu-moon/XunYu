use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rayon::prelude::*;

use crate::util::{matches_patterns, normalize_glob_path};

use super::util::norm;

pub(crate) struct ScannedFile {
    pub(crate) path: PathBuf,
    pub(crate) size: u64,
    pub(crate) modified: SystemTime,
}

pub(crate) fn scan_files(
    root: &Path,
    includes: &[String],
    exclude_patterns: &[String],
    include_patterns: &[String],
) -> HashMap<String, ScannedFile> {
    let fast_path = exclude_patterns.is_empty() && include_patterns.is_empty();
    if includes.is_empty() {
        let mut files = HashMap::new();
        if fast_path {
            walk_fast(root, root, &mut files);
        } else {
            walk_filtered(root, root, exclude_patterns, include_patterns, &mut files);
        }
        return files;
    }

    let parts: Vec<HashMap<String, ScannedFile>> = includes
        .par_iter()
        .map(|inc| {
            let mut local = HashMap::new();
            let full = root.join(inc);
            if full.is_file() {
                if let Ok(meta) = fs::metadata(&full) {
                    local.insert(
                        norm(inc),
                        ScannedFile {
                            path: full,
                            size: meta.len(),
                            modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                        },
                    );
                }
            } else if full.is_dir() {
                if fast_path {
                    walk_fast(&full, root, &mut local);
                } else {
                    walk_filtered(&full, root, exclude_patterns, include_patterns, &mut local);
                }
            }
            local
        })
        .collect();

    let capacity = parts.iter().map(HashMap::len).sum();
    let mut files = HashMap::with_capacity(capacity);
    for part in parts {
        files.extend(part);
    }
    files
}

fn walk_fast(dir: &Path, root: &Path, files: &mut HashMap<String, ScannedFile>) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(rd) = fs::read_dir(&current) else {
            continue;
        };
        for entry in rd.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let rel = path.strip_prefix(root).unwrap_or(&path);
            files.insert(
                rel_key(rel),
                ScannedFile {
                    path,
                    size: meta.len(),
                    modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                },
            );
        }
    }
}

fn walk_filtered(
    dir: &Path,
    root: &Path,
    exclude_patterns: &[String],
    include_patterns: &[String],
    files: &mut HashMap<String, ScannedFile>,
) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };

    for entry in rd.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let rel_norm = normalize_glob_path(&rel.to_string_lossy());
        let name = entry.file_name().to_string_lossy().into_owned();
        let name_lower = name.to_lowercase();
        let is_dir = file_type.is_dir();

        if !include_patterns.is_empty()
            && matches_patterns(&rel_norm, &name_lower, include_patterns, is_dir)
        {
            // include 模式命中，保留
        } else if matches_patterns(&rel_norm, &name_lower, exclude_patterns, is_dir) {
            continue;
        }

        if is_dir {
            walk_filtered(&path, root, exclude_patterns, include_patterns, files);
            continue;
        }

        let Ok(meta) = entry.metadata() else {
            continue;
        };
        files.insert(
            rel_norm.replace('/', "\\"),
            ScannedFile {
                path,
                size: meta.len(),
                modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            },
        );
    }
}

fn rel_key(rel: &Path) -> String {
    let value = rel.to_string_lossy();
    if value.contains('/') {
        value.replace('/', "\\")
    } else {
        value.into_owned()
    }
}
