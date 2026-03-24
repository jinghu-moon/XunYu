use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::backup::legacy::hash_manifest::read_backup_snapshot_manifest;

#[allow(dead_code)]
pub(crate) struct FileMeta {
    pub(crate) size: u64,
    pub(crate) modified: SystemTime,
    pub(crate) modified_ns: u64,
    pub(crate) created_time_ns: Option<u64>,
    pub(crate) win_attributes: u32,
    pub(crate) file_id: Option<String>,
    pub(crate) content_hash: Option<[u8; 32]>,
}

pub(crate) fn read_baseline(prev: &Path) -> HashMap<String, FileMeta> {
    if let Ok(manifest) = read_backup_snapshot_manifest(prev) {
        return manifest
            .entries
            .into_iter()
            .map(|entry| {
                (
                    entry.path.replace('/', "\\"),
                    FileMeta {
                        size: entry.size,
                        modified: system_time_from_unix_ns(entry.mtime_ns),
                        modified_ns: entry.mtime_ns,
                        created_time_ns: entry.created_time_ns,
                        win_attributes: entry.win_attributes,
                        file_id: entry.file_id,
                        content_hash: Some(entry.content_hash),
                    },
                )
            })
            .collect();
    }

    read_metadata_only_baseline(prev)
}

pub(crate) fn read_metadata_only_baseline(prev: &Path) -> HashMap<String, FileMeta> {
    let mut old = HashMap::new();
    if prev.extension().is_some_and(|e| e == "zip") && prev.is_file() {
        read_baseline_zip(prev, &mut old);
    } else if prev.is_dir() {
        read_baseline_dir(prev, prev, &mut old);
    }
    old
}

fn is_backup_internal_name(name: &str) -> bool {
    matches!(name, ".bak-meta.json" | ".bak-manifest.json")
}

fn read_baseline_zip(zip_path: &Path, old: &mut HashMap<String, FileMeta>) {
    let Ok(file) = fs::File::open(zip_path) else {
        return;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return;
    };
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('/', "\\");
        if name.is_empty() {
            continue;
        }
        if is_backup_internal_name(name.rsplit('\\').next().unwrap_or(&name)) {
            continue;
        }
        let modified = entry
            .last_modified()
            .map(zip_datetime_to_systime)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let modified_ns = modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|value| value.as_nanos() as u64)
            .unwrap_or(0);
        old.insert(
            name,
            FileMeta {
                size: entry.size(),
                modified,
                modified_ns,
                created_time_ns: None,
                win_attributes: 0,
                file_id: None,
                content_hash: None,
            },
        );
    }
}

fn read_baseline_dir(dir: &Path, base: &Path, old: &mut HashMap<String, FileMeta>) {
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
            let rel = path.strip_prefix(base).unwrap_or(&path);
            if is_backup_internal_name(rel.file_name().and_then(|s| s.to_str()).unwrap_or_default())
            {
                continue;
            }
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let modified_ns = modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|value| value.as_nanos() as u64)
                .unwrap_or(0);
            old.insert(
                rel_key(rel),
                FileMeta {
                    size: meta.len(),
                    modified,
                    modified_ns,
                    created_time_ns: None,
                    win_attributes: 0,
                    file_id: None,
                    content_hash: None,
                },
            );
        }
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

fn zip_datetime_to_systime(dt: zip::DateTime) -> SystemTime {
    fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
        let y = y - if m <= 2 { 1 } else { 0 };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = m + if m > 2 { -3 } else { 9 };
        let doy = (153 * m + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }

    let y = dt.year() as i64;
    let m = dt.month() as i64;
    let d = dt.day() as i64;
    let hh = dt.hour() as i64;
    let mm = dt.minute() as i64;
    let ss = dt.second() as i64;

    let days = days_from_civil(y, m, d);
    let secs = days.saturating_mul(86_400) + hh * 3_600 + mm * 60 + ss;
    if secs <= 0 {
        SystemTime::UNIX_EPOCH
    } else {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs as u64)
    }
}

fn system_time_from_unix_ns(unix_ns: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_nanos(unix_ns)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::backup::legacy::hash_manifest::{
        BackupSnapshotEntry, BackupSnapshotManifest, write_backup_snapshot_manifest,
    };

    use super::read_baseline;

    #[test]
    fn read_baseline_dir_skips_internal_backup_files() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "ok").unwrap();
        std::fs::write(dir.path().join(".bak-meta.json"), "{}").unwrap();
        std::fs::write(dir.path().join(".bak-manifest.json"), "{}").unwrap();

        let baseline = read_baseline(dir.path());
        assert!(baseline.contains_key("a.txt"));
        assert!(!baseline.contains_key(".bak-meta.json"));
        assert!(!baseline.contains_key(".bak-manifest.json"));
        assert_eq!(baseline.len(), 1);
    }

    #[test]
    fn read_baseline_prefers_hash_manifest_when_present() {
        let dir = tempdir().unwrap();
        let manifest = BackupSnapshotManifest {
            version: 2,
            snapshot_id: "01JQ7J3N4QCG7N2T1DJ7C9ZK4Q".to_string(),
            created_at_ns: 1,
            source_root: dir.path().display().to_string(),
            file_count: 1,
            total_raw_bytes: 5,
            entries: vec![BackupSnapshotEntry {
                path: "a.txt".to_string(),
                content_hash: [0xaa; 32],
                size: 5,
                mtime_ns: 123,
                created_time_ns: Some(456),
                win_attributes: 32,
                file_id: Some("file-1".to_string()),
            }],
            removed: Vec::new(),
        };
        write_backup_snapshot_manifest(dir.path(), &manifest).unwrap();

        let baseline = read_baseline(dir.path());
        let file = baseline.get("a.txt").unwrap();
        assert_eq!(file.size, 5);
        assert_eq!(file.modified_ns, 123);
        assert_eq!(file.created_time_ns, Some(456));
        assert_eq!(file.win_attributes, 32);
        assert_eq!(file.file_id.as_deref(), Some("file-1"));
        assert_eq!(file.content_hash, Some([0xaa; 32]));
    }
}
