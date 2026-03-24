//! blake3 完整性校验：生成/验证 .bak-manifest.json

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

use crate::backup::artifact::entry::{file_attributes, metadata_created_time_ns, system_time_to_unix_ns};
use crate::backup::common::hash::compute_file_content_hash;
use crate::backup::legacy::hash_manifest::{
    BackupSnapshotEntry, BackupSnapshotManifest, HashManifestError, read_backup_snapshot_manifest,
    write_backup_snapshot_manifest,
};

pub(crate) enum VerifyResult {
    Ok,
    /// 损坏的文件列表
    Corrupted(Vec<String>),
    /// manifest 不存在
    NoManifest,
}

pub(crate) fn file_blake3(path: &Path) -> Option<[u8; 32]> {
    compute_file_content_hash(path).ok()
}

/// 将文件哈希写入 backup_path/.bak-manifest.json
#[allow(dead_code)]
pub(crate) fn write_manifest(backup_path: &Path, files: &HashMap<String, [u8; 32]>) {
    let entries: Vec<BackupSnapshotEntry> = files
        .iter()
        .filter_map(|(rel, hash)| {
            let file_path = backup_path.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            let metadata = fs::metadata(&file_path).ok()?;
            Some(BackupSnapshotEntry {
                path: rel.clone(),
                content_hash: *hash,
                size: metadata.len(),
                mtime_ns: metadata
                    .modified()
                    .ok()
                    .map(system_time_to_unix_ns)
                    .unwrap_or(0),
                created_time_ns: metadata_created_time_ns(&metadata),
                win_attributes: file_attributes(&metadata),
                file_id: None,
            })
        })
        .collect();

    let created_at_ns = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    let manifest = BackupSnapshotManifest::new(
        backup_path.display().to_string(),
        created_at_ns,
        entries,
        Vec::new(),
    );
    let _ = write_backup_snapshot_manifest(backup_path, &manifest);
}

/// 校验 backup_path 中所有文件是否与 manifest 一致
pub(crate) fn verify_manifest(backup_path: &Path) -> VerifyResult {
    let manifest = match read_backup_snapshot_manifest(backup_path) {
        Ok(manifest) => manifest,
        Err(HashManifestError::NotFound(_)) => return VerifyResult::NoManifest,
        Err(_) => return VerifyResult::NoManifest,
    };

    let mut corrupted: Vec<String> = Vec::new();
    if backup_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        let Ok(file) = fs::File::open(backup_path) else {
            return VerifyResult::NoManifest;
        };
        let Ok(mut archive) = zip::ZipArchive::new(file) else {
            return VerifyResult::NoManifest;
        };
        for entry in &manifest.entries {
            match file_blake3_from_zip_entry(&mut archive, &entry.path) {
                Some(hash) if hash == entry.content_hash => {}
                _ => corrupted.push(entry.path.clone()),
            }
        }
        for extra in collect_zip_extra_files(&mut archive, &manifest) {
            corrupted.push(extra);
        }
    } else {
        for entry in &manifest.entries {
            let file_path = backup_path.join(entry.path.replace('/', std::path::MAIN_SEPARATOR_STR));
            match file_blake3(&file_path) {
                Some(hash) if hash == entry.content_hash => {}
                _ => corrupted.push(entry.path.clone()),
            }
        }
        for extra in collect_dir_extra_files(backup_path, &manifest) {
            corrupted.push(extra);
        }
    }

    corrupted.sort();
    corrupted.dedup();

    if corrupted.is_empty() {
        VerifyResult::Ok
    } else {
        VerifyResult::Corrupted(corrupted)
    }
}

fn file_blake3_from_zip_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    path: &str,
) -> Option<[u8; 32]> {
    let mut entry = archive.by_name(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = entry.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(*hasher.finalize().as_bytes())
}

fn collect_dir_extra_files(
    backup_root: &Path,
    manifest: &BackupSnapshotManifest,
) -> Vec<String> {
    let expected: std::collections::HashSet<&str> =
        manifest.entries.iter().map(|entry| entry.path.as_str()).collect();
    let mut extras = Vec::new();
    let mut stack = vec![backup_root.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(rd) = fs::read_dir(&current) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = path
                .strip_prefix(backup_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if is_internal_backup_file(&rel) {
                continue;
            }
            if !expected.contains(rel.as_str()) {
                extras.push(rel);
            }
        }
    }
    extras
}

fn collect_zip_extra_files<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    manifest: &BackupSnapshotManifest,
) -> Vec<String> {
    let expected: std::collections::HashSet<&str> =
        manifest.entries.iter().map(|entry| entry.path.as_str()).collect();
    let mut extras = Vec::new();
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let path = entry.name().replace('\\', "/");
        if is_internal_backup_file(&path) {
            continue;
        }
        if !expected.contains(path.as_str()) {
            extras.push(path);
        }
    }
    extras
}

fn is_internal_backup_file(rel: &str) -> bool {
    matches!(
        rel.rsplit('/').next().unwrap_or(rel),
        ".bak-manifest.json" | ".bak-meta.json"
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::tempdir;

    use super::{VerifyResult, file_blake3, verify_manifest, write_manifest};

    #[test]
    fn verify_manifest_roundtrip_ok() {
        let dir = tempdir().unwrap();
        let backup_root = dir.path();
        std::fs::write(backup_root.join("a.txt"), b"alpha").unwrap();
        std::fs::write(backup_root.join("b.txt"), b"beta").unwrap();

        let mut files = HashMap::new();
        files.insert(
            "a.txt".to_string(),
            file_blake3(&backup_root.join("a.txt")).unwrap(),
        );
        files.insert(
            "b.txt".to_string(),
            file_blake3(&backup_root.join("b.txt")).unwrap(),
        );
        write_manifest(backup_root, &files);

        assert!(matches!(verify_manifest(backup_root), VerifyResult::Ok));
    }

    #[test]
    fn verify_manifest_detects_corrupted_files() {
        let dir = tempdir().unwrap();
        let backup_root = dir.path();
        std::fs::write(backup_root.join("a.txt"), b"alpha").unwrap();

        let mut files = HashMap::new();
        files.insert(
            "a.txt".to_string(),
            file_blake3(&backup_root.join("a.txt")).unwrap(),
        );
        write_manifest(backup_root, &files);
        std::fs::write(backup_root.join("a.txt"), b"changed").unwrap();

        match verify_manifest(backup_root) {
            VerifyResult::Corrupted(files) => assert_eq!(files, vec!["a.txt".to_string()]),
            _other => panic!("expected corrupted manifest result, got unexpected variant"),
        }
    }

    #[test]
    fn verify_manifest_detects_files_missing_from_manifest() {
        let dir = tempdir().unwrap();
        let backup_root = dir.path();
        std::fs::write(backup_root.join("a.txt"), b"alpha").unwrap();
        std::fs::write(backup_root.join("b.txt"), b"beta").unwrap();

        let mut files = HashMap::new();
        files.insert(
            "a.txt".to_string(),
            file_blake3(&backup_root.join("a.txt")).unwrap(),
        );
        write_manifest(backup_root, &files);

        match verify_manifest(backup_root) {
            VerifyResult::Corrupted(files) => assert_eq!(files, vec!["b.txt".to_string()]),
            _other => panic!("expected corrupted result for untracked file"),
        }
    }
}
