use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rayon::prelude::*;

use crate::util::{matches_patterns, normalize_glob_path};

use super::util::norm;

pub(crate) fn scan_files(
    root: &Path,
    includes: &[String],
    exclude_patterns: &[String],
    include_patterns: &[String],
) -> HashMap<String, PathBuf> {
    let files = Mutex::new(HashMap::new());

    if includes.is_empty() {
        walk_parallel(root, root, exclude_patterns, include_patterns, &files);
    } else {
        // 顶层 include 路径并行处理
        includes.par_iter().for_each(|inc| {
            let full = root.join(inc);
            if full.is_file() {
                let mut guard = files.lock().unwrap();
                guard.insert(norm(inc), full);
            } else if full.is_dir() {
                walk_parallel(&full, root, exclude_patterns, include_patterns, &files);
            }
        });
    }

    files.into_inner().unwrap()
}

fn walk_parallel(
    dir: &Path,
    root: &Path,
    exclude_patterns: &[String],
    include_patterns: &[String],
    files: &Mutex<HashMap<String, PathBuf>>,
) {
    let Ok(rd) = fs::read_dir(dir) else { return };

    // 收集当前目录条目，分离子目录与文件
    let mut subdirs: Vec<PathBuf> = Vec::new();
    let mut file_entries: Vec<(String, PathBuf)> = Vec::new();

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
            // include 模式命中，保留
        } else if matches_patterns(&rel_norm, &name_lower, exclude_patterns, is_dir) {
            continue;
        }

        if is_dir {
            subdirs.push(path);
        } else {
            file_entries.push((rel_norm.replace('/', "\\"), path));
        }
    }

    // 批量写入文件条目
    {
        let mut guard = files.lock().unwrap();
        for (key, path) in file_entries {
            guard.insert(key, path);
        }
    }

    // 子目录并行递归
    subdirs.par_iter().for_each(|sub| {
        walk_parallel(sub, root, exclude_patterns, include_patterns, files);
    });
}
