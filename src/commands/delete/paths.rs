use std::path::{Component, Path, PathBuf};

use super::types::PathKind;
use super::winapi;

pub(super) fn absolute_path(raw: &str) -> PathBuf {
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        return p;
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(p)
}

pub(super) fn classify_path(path: &Path) -> Option<PathKind> {
    if let Ok(meta) = std::fs::metadata(path) {
        return if meta.is_dir() {
            Some(PathKind::Dir)
        } else {
            Some(PathKind::File)
        };
    }
    let attrs = winapi::get_attrs(path.to_string_lossy().as_ref());
    if attrs == 0xFFFF_FFFF {
        return None;
    }
    if winapi::is_dir_attr(attrs) {
        Some(PathKind::Dir)
    } else {
        Some(PathKind::File)
    }
}

pub(super) fn volume_root(path: &Path) -> String {
    for comp in path.components() {
        if let Component::Prefix(p) = comp {
            let mut s = p.as_os_str().to_string_lossy().into_owned();
            if !s.ends_with('\\') && !s.ends_with('/') {
                s.push('\\');
            }
            return s;
        }
    }
    path.to_string_lossy().to_string()
}
