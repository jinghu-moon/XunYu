use std::io;
use std::path::{Path, PathBuf};

use crate::util::normalize_path;

use super::super::engine::RedirectResult;
use super::ignore::IgnoreSet;

pub(super) fn sweep_after_move(
    source_root: &Path,
    dest_dirs: &[PathBuf],
    ignore: &IgnoreSet,
    results: &[RedirectResult],
    max_dirs_total: usize,
    max_depth: usize,
) {
    if max_dirs_total == 0 || max_depth == 0 {
        return;
    }

    let mut removed_total = 0usize;
    for r in results {
        if removed_total >= max_dirs_total {
            break;
        }
        if (r.action != "move" && r.action != "dedup") || r.result != "success" {
            continue;
        }
        let src = PathBuf::from(&r.src);
        let Some(mut dir) = src.parent().map(|p| p.to_path_buf()) else {
            continue;
        };
        removed_total += sweep_empty_parents(
            source_root,
            dest_dirs,
            ignore.protect_prefixes(),
            &mut dir,
            max_depth,
            max_dirs_total.saturating_sub(removed_total),
        );
    }
}

fn sweep_empty_parents(
    source_root: &Path,
    dest_dirs: &[PathBuf],
    protect_prefixes: &[PathBuf],
    dir: &mut PathBuf,
    max_depth: usize,
    max_dirs: usize,
) -> usize {
    let source_key = normalize_path(&source_root.to_string_lossy());
    let dest_keys: Vec<String> = dest_dirs
        .iter()
        .map(|d| normalize_path(&d.to_string_lossy()))
        .collect();
    let protect_keys: Vec<String> = protect_prefixes
        .iter()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| normalize_path(&p.to_string_lossy()))
        .collect();

    let mut removed = 0usize;
    let mut depth = 0usize;

    while removed < max_dirs && depth < max_depth {
        depth += 1;

        if dir.as_os_str().is_empty() {
            break;
        }

        let dir_key = normalize_path(&dir.to_string_lossy());
        if dir_key == source_key {
            break;
        }
        if !is_key_under(&dir_key, &source_key) {
            break;
        }
        if is_sweep_boundary_key(&dir_key, &dest_keys, &protect_keys) {
            break;
        }

        let empty = match is_dir_empty(dir) {
            Ok(v) => v,
            Err(_) => break,
        };
        if !empty {
            break;
        }

        if std::fs::remove_dir(&*dir).is_err() {
            break;
        }
        removed += 1;

        let Some(parent) = dir.parent().map(|p| p.to_path_buf()) else {
            break;
        };
        *dir = parent;
    }

    removed
}

fn is_key_under(path_key: &str, root_key: &str) -> bool {
    if path_key == root_key {
        return true;
    }
    let mut prefix = root_key.to_string();
    prefix.push('/');
    path_key.starts_with(&prefix)
}

fn is_sweep_boundary_key(dir_key: &str, dest_keys: &[String], protect_keys: &[String]) -> bool {
    if dest_keys.iter().any(|d| d == dir_key) {
        return true;
    }
    for p in protect_keys {
        if dir_key == p {
            return true;
        }
        let mut prefix = p.clone();
        prefix.push('/');
        if dir_key.starts_with(&prefix) {
            return true;
        }
    }
    false
}

fn is_dir_empty(dir: &Path) -> io::Result<bool> {
    let mut it = std::fs::read_dir(dir)?;
    Ok(it.next().is_none())
}
