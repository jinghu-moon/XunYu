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
            if name.ends_with(".meta.json") {
                continue;
            }
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{parse_version, scan_versions};

    #[test]
    fn parse_version_accepts_backup_names_and_rejects_non_backup_names() {
        assert_eq!(parse_version("v12-demo", "v"), Some(12));
        assert_eq!(parse_version("backup12-demo", "backup"), Some(12));
        assert_eq!(parse_version("v-demo", "v"), None);
        assert_eq!(parse_version("v12demo", "v"), None);
        assert_eq!(parse_version("demo-v12", "v"), None);
    }

    #[test]
    fn scan_versions_ignores_zip_meta_companion_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("v1-first.zip"), b"zip").unwrap();
        std::fs::write(root.join("v1-first.meta.json"), b"{}").unwrap();
        std::fs::write(root.join("v2-second.zip"), b"zip").unwrap();
        std::fs::write(root.join("v2-second.meta.json"), b"{}").unwrap();

        let info = scan_versions(root, "v");
        assert_eq!(info.next_version, 3);
        assert_eq!(info.prev_name.as_deref(), Some("v2-second.zip"));
        assert!(
            info.prev_path
                .as_ref()
                .is_some_and(|p| p.file_name().and_then(|n| n.to_str()) == Some("v2-second.zip"))
        );
    }
}
