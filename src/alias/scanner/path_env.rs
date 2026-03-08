use std::collections::HashSet;
use std::path::Path;
use std::time::UNIX_EPOCH;

use super::{AppEntry, Source, auto_alias, cache, is_utility_exe};

pub(crate) fn scan_path_env(no_cache: bool) -> Vec<AppEntry> {
    let fingerprint = path_env_fingerprint();
    if !no_cache {
        if let Some(v) = cache::load_source("path", 24 * 3600, Some(&fingerprint)) {
            return v;
        }
    }
    let list = scan_path_env_inner();
    if !no_cache {
        cache::store_source("path", &list, Some(&fingerprint));
    }
    list
}

fn scan_path_env_inner() -> Vec<AppEntry> {
    let mut list = Vec::new();
    let mut seen = HashSet::<String>::new();
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in path_var.split(';') {
        let dir = dir.trim();
        if dir.is_empty() {
            continue;
        }
        let path = Path::new(dir);
        if !path.is_dir() {
            continue;
        }
        if is_xun_shims_dir(path) {
            continue;
        }
        let Ok(read) = std::fs::read_dir(path) else {
            continue;
        };
        for item in read.flatten() {
            let file = item.path();
            if !file.is_file() {
                continue;
            }
            let ext_ok = file
                .extension()
                .and_then(|v| v.to_str())
                .map(|v| v.eq_ignore_ascii_case("exe"))
                .unwrap_or(false);
            if !ext_ok {
                continue;
            }
            let exe_name = file
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if is_utility_exe(&exe_name) {
                continue;
            }
            if !seen.insert(exe_name.clone()) {
                continue;
            }
            let display_name = file
                .file_stem()
                .and_then(|v| v.to_str())
                .unwrap_or("app")
                .to_string();
            list.push(AppEntry {
                name: auto_alias(&display_name),
                display_name,
                exe_path: file.to_string_lossy().to_string(),
                source: Source::PathEnv,
            });
        }
    }
    list
}

fn is_xun_shims_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
        return false;
    };
    if !name.eq_ignore_ascii_case("shims") {
        return false;
    }
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|v| v.to_str())
        .map(|v| v.eq_ignore_ascii_case("xun"))
        .unwrap_or(false)
}

fn path_env_fingerprint() -> String {
    let mut out = String::new();
    let path_var = std::env::var("PATH").unwrap_or_default();
    for raw_dir in path_var.split(';') {
        let dir = raw_dir.trim();
        if dir.is_empty() {
            continue;
        }
        out.push_str(&dir.to_ascii_lowercase());
        out.push('=');
        let secs = Path::new(dir)
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        out.push_str(&secs.to_string());
        out.push(';');
    }
    out
}
