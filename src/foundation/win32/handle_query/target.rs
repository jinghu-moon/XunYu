use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::device_map::{dos_to_nt_paths, normalize_path_like};

#[derive(Debug, Clone)]
pub(super) struct TargetPath {
    pub(super) is_dir: bool,
    pub(super) dos_path: String,
    pub(super) nt_paths: Vec<String>,
}

pub(super) fn build_targets(
    paths: &[&Path],
    device_map: &HashMap<String, Vec<String>>,
) -> Vec<TargetPath> {
    paths
        .iter()
        .map(|path| {
            let abs = absolute_path(path);
            let dos_path = normalize_path_like(&abs.to_string_lossy());
            let nt_paths = dos_to_nt_paths(&dos_path, device_map);
            TargetPath {
                is_dir: path.is_dir(),
                dos_path,
                nt_paths,
            }
        })
        .collect()
}

pub(super) fn path_matches_target(
    target: &TargetPath,
    handle_nt: &str,
    handle_dos: Option<&str>,
) -> bool {
    let nt_match = target.nt_paths.iter().any(|p| {
        if target.is_dir {
            is_same_or_child(handle_nt, p)
        } else {
            path_eq(handle_nt, p)
        }
    });
    if nt_match {
        return true;
    }

    if let Some(handle_dos) = handle_dos {
        if target.is_dir {
            is_same_or_child(handle_dos, &target.dos_path)
        } else {
            path_eq(handle_dos, &target.dos_path)
        }
    } else {
        false
    }
}

pub(super) fn path_eq(a: &str, b: &str) -> bool {
    a == b
}

pub(super) fn is_same_or_child(path: &str, parent: &str) -> bool {
    path_eq(path, parent)
        || path
            .strip_prefix(parent)
            .is_some_and(|rest| rest.starts_with('\\'))
}

fn absolute_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(path)
}
