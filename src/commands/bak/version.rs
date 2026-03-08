use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct VersionInfo {
    pub(crate) next_version: u32,
    pub(crate) prev_path: Option<PathBuf>,
    pub(crate) prev_name: Option<String>,
}

/// Parse version number from backup name: "{prefix}{digits}-..." → Some(digits)
pub(crate) fn parse_version(name: &str, prefix: &str) -> Option<u32> {
    let rest = name.strip_prefix(prefix)?;
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let digit_str = &rest[..end];
    if rest[end..].starts_with('-') {
        digit_str.parse().ok()
    } else {
        None
    }
}

pub(crate) fn scan_versions(backups_root: &Path, prefix: &str) -> VersionInfo {
    let _ = fs::create_dir_all(backups_root);

    let mut items: Vec<(u32, PathBuf, String)> = Vec::new();
    if let Ok(rd) = fs::read_dir(backups_root) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if let Some(n) = parse_version(&name, prefix) {
                items.push((n, e.path(), name));
            }
        }
    }
    items.sort_by_key(|i| i.0);

    let next_version = items.last().map_or(1, |i| i.0 + 1);
    let prev = items.last().map(|i| (i.1.clone(), i.2.clone()));
    VersionInfo {
        next_version,
        prev_path: prev.as_ref().map(|p| p.0.clone()),
        prev_name: prev.map(|p| p.1),
    }
}
