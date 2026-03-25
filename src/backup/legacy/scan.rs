use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rayon::prelude::*;

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
};

use crate::backup::artifact::entry::{
    file_attributes, metadata_created_time_ns, system_time_to_unix_ns,
};
use crate::backup::common::hash::compute_file_content_hash;
use crate::util::{matches_patterns, normalize_glob_path};

use super::hash_cache::{
    HASH_CACHE_FILE, HashCacheEntry, cache_hit, load_hash_cache, save_hash_cache,
    update_cache_entry,
};
use super::util::norm;

pub(crate) struct ScannedFile {
    pub(crate) path: PathBuf,
    pub(crate) size: u64,
    pub(crate) modified: SystemTime,
    pub(crate) modified_ns: u64,
    pub(crate) created_time_ns: Option<u64>,
    pub(crate) win_attributes: u32,
    pub(crate) file_id: Option<String>,
    pub(crate) content_hash: Option<[u8; 32]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScanHashStats {
    pub(crate) total_files: u64,
    pub(crate) hash_checked_files: u64,
    pub(crate) hash_cache_hits: u64,
    pub(crate) hash_computed_files: u64,
    pub(crate) hash_failed_files: u64,
}

pub(crate) struct ScanWithHashResult {
    pub(crate) files: HashMap<String, ScannedFile>,
    pub(crate) stats: ScanHashStats,
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
                    local.insert(norm(inc), build_scanned_file(full, meta));
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

#[allow(dead_code)]
pub(crate) fn scan_files_with_hash(
    root: &Path,
    includes: &[String],
    exclude_patterns: &[String],
    include_patterns: &[String],
) -> HashMap<String, ScannedFile> {
    scan_files_with_hash_details(root, includes, exclude_patterns, include_patterns).files
}

pub(crate) fn scan_files_with_hash_details(
    root: &Path,
    includes: &[String],
    exclude_patterns: &[String],
    include_patterns: &[String],
) -> ScanWithHashResult {
    let mut files = scan_files(root, includes, exclude_patterns, include_patterns);
    let mut cache = load_hash_cache(root);
    let mut stats = ScanHashStats {
        total_files: files.len() as u64,
        hash_checked_files: files.len() as u64,
        ..ScanHashStats::default()
    };
    for scanned in files.values_mut() {
        let rel = scanned
            .path
            .strip_prefix(root)
            .unwrap_or(&scanned.path)
            .to_string_lossy()
            .replace('/', "\\");
        if let Some(hash) = cache_hit(
            &cache,
            &rel,
            scanned.size,
            scanned.modified_ns,
            scanned.created_time_ns,
            scanned.win_attributes,
            scanned.file_id.as_deref(),
        ) {
            stats.hash_cache_hits += 1;
            scanned.content_hash = Some(hash);
            update_cache_entry(
                &mut cache,
                rel,
                HashCacheEntry {
                    size: scanned.size,
                    mtime_ns: scanned.modified_ns,
                    created_time_ns: scanned.created_time_ns,
                    win_attributes: scanned.win_attributes,
                    file_id: scanned.file_id.clone(),
                    content_hash: hash,
                },
            );
            continue;
        }

        match compute_file_content_hash(&scanned.path) {
            Ok(content_hash) => {
                stats.hash_computed_files += 1;
                scanned.content_hash = Some(content_hash);
                update_cache_entry(
                    &mut cache,
                    rel,
                    HashCacheEntry {
                        size: scanned.size,
                        mtime_ns: scanned.modified_ns,
                        created_time_ns: scanned.created_time_ns,
                        win_attributes: scanned.win_attributes,
                        file_id: scanned.file_id.clone(),
                        content_hash,
                    },
                );
            }
            Err(_) => {
                stats.hash_failed_files += 1;
                scanned.content_hash = None;
            }
        }
    }
    cache.files.retain(|rel, _| files.contains_key(rel));
    save_hash_cache(root, &cache);
    ScanWithHashResult { files, stats }
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
            if path.file_name().and_then(|name| name.to_str()) == Some(HASH_CACHE_FILE) {
                continue;
            }
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let rel = path.strip_prefix(root).unwrap_or(&path);
            files.insert(rel_key(rel), build_scanned_file(path, meta));
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
        if path.file_name().and_then(|name| name.to_str()) == Some(HASH_CACHE_FILE) {
            continue;
        }
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
        files.insert(rel_norm.replace('/', "\\"), build_scanned_file(path, meta));
    }
}

fn build_scanned_file(path: PathBuf, meta: fs::Metadata) -> ScannedFile {
    let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    ScannedFile {
        file_id: collect_file_id(&path),
        path,
        size: meta.len(),
        modified,
        modified_ns: system_time_to_unix_ns(modified),
        created_time_ns: metadata_created_time_ns(&meta),
        win_attributes: file_attributes(&meta),
        content_hash: None,
    }
}

fn file_id_collection_enabled() -> bool {
    file_id_collection_enabled_with(|name| std::env::var_os(name))
}

fn file_id_collection_enabled_with<F>(mut get_env: F) -> bool
where
    F: FnMut(&str) -> Option<OsString>,
{
    !["XUN_BACKUP_DISABLE_FILE_ID", "XUN_DISABLE_FILE_ID"]
        .into_iter()
        .any(|name| get_env(name).is_some())
}

#[cfg(windows)]
fn collect_file_id(path: &Path) -> Option<String> {
    if !file_id_collection_enabled() {
        return None;
    }
    let file = fs::File::open(path).ok()?;
    let handle = file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }

    let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
    let ok = unsafe { GetFileInformationByHandle(handle, &mut info) };
    if ok == 0 {
        return None;
    }

    let index = ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64);
    Some(format!("{:08x}:{:016x}", info.dwVolumeSerialNumber, index))
}

#[cfg(not(windows))]
fn collect_file_id(_path: &Path) -> Option<String> {
    None
}

fn rel_key(rel: &Path) -> String {
    let value = rel.to_string_lossy();
    if value.contains('/') {
        value.replace('/', "\\")
    } else {
        value.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use tempfile::tempdir;

    use super::{
        collect_file_id, file_id_collection_enabled_with, scan_files, scan_files_with_hash,
        scan_files_with_hash_details,
    };
    use crate::backup::legacy::hash_cache::HASH_CACHE_FILE;

    #[test]
    fn scan_files_populates_extended_metadata_fields() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "alpha").unwrap();

        let files = scan_files(dir.path(), &[], &[], &[]);
        let file = files.get("a.txt").unwrap();
        assert_eq!(file.size, 5);
        assert!(file.modified_ns > 0);
        assert_eq!(file.win_attributes, 32u32);
        #[cfg(windows)]
        assert!(file.file_id.is_some());
        #[cfg(not(windows))]
        assert!(file.file_id.is_none());
        assert!(file.content_hash.is_none());
    }

    #[test]
    fn collect_file_id_returns_none_when_file_is_missing() {
        let dir = tempdir().unwrap();
        assert!(collect_file_id(&dir.path().join("missing.txt")).is_none());
    }

    #[test]
    fn file_id_collection_can_be_disabled_via_env_flag() {
        assert!(!file_id_collection_enabled_with(|name| {
            if name == "XUN_BACKUP_DISABLE_FILE_ID" {
                Some(OsString::from("1"))
            } else {
                None
            }
        }));
        assert!(file_id_collection_enabled_with(|_name| None));
    }

    #[test]
    fn scan_files_with_hash_computes_blake3_for_each_file() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "alpha").unwrap();

        let files = scan_files_with_hash(dir.path(), &[], &[], &[]);
        let file = files.get("a.txt").unwrap();
        assert_eq!(file.content_hash, Some(*blake3::hash(b"alpha").as_bytes()));
    }

    #[test]
    fn scan_files_skips_hash_cache_file() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join(HASH_CACHE_FILE), "{}").unwrap();
        std::fs::write(dir.path().join("a.txt"), "alpha").unwrap();

        let files = scan_files(dir.path(), &[], &[], &[]);
        assert!(files.contains_key("a.txt"));
        assert!(!files.contains_key(HASH_CACHE_FILE));
    }

    #[test]
    fn scan_files_with_hash_details_reports_cache_hits() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "alpha").unwrap();

        let first = scan_files_with_hash_details(dir.path(), &[], &[], &[]);
        assert_eq!(first.stats.total_files, 1);
        assert_eq!(first.stats.hash_checked_files, 1);
        assert_eq!(first.stats.hash_cache_hits, 0);
        assert_eq!(first.stats.hash_computed_files, 1);
        assert_eq!(first.stats.hash_failed_files, 0);

        let second = scan_files_with_hash_details(dir.path(), &[], &[], &[]);
        assert_eq!(second.stats.total_files, 1);
        assert_eq!(second.stats.hash_checked_files, 1);
        assert_eq!(second.stats.hash_cache_hits, 1);
        assert_eq!(second.stats.hash_computed_files, 0);
        assert_eq!(second.stats.hash_failed_files, 0);
    }

    #[test]
    fn scan_files_with_hash_reports_same_hash_for_same_content_different_paths() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("nested")).unwrap();
        std::fs::write(dir.path().join("a.txt"), "alpha").unwrap();
        std::fs::write(dir.path().join("nested").join("b.txt"), "alpha").unwrap();

        let files = scan_files_with_hash(dir.path(), &[], &[], &[]);
        assert_eq!(
            files["a.txt"].content_hash,
            files["nested\\b.txt"].content_hash
        );
    }

    #[test]
    fn scan_files_with_hash_handles_unicode_spaces_and_deep_paths() {
        let dir = tempdir().unwrap();
        let deep = dir
            .path()
            .join("中文目录")
            .join("path with spaces")
            .join("deep")
            .join("level4");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(dir.path().join("中文目录").join("说明.txt"), "alpha").unwrap();
        std::fs::write(deep.join("leaf.txt"), "beta").unwrap();

        let files = scan_files_with_hash(dir.path(), &[], &[], &[]);
        assert!(files.contains_key("中文目录\\说明.txt"));
        assert!(files.contains_key("中文目录\\path with spaces\\deep\\level4\\leaf.txt"));
        assert!(files["中文目录\\说明.txt"].content_hash.is_some());
        assert!(
            files["中文目录\\path with spaces\\deep\\level4\\leaf.txt"]
                .content_hash
                .is_some()
        );
    }
}
