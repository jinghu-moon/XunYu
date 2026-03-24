use std::fs;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::backup::common::hash::{decode_hash_hex, encode_hash_hex};

pub(crate) const HASH_MANIFEST_FILE: &str = ".bak-manifest.json";
pub(crate) const HASH_MANIFEST_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BackupSnapshotManifest {
    pub(crate) version: u32,
    pub(crate) snapshot_id: String,
    pub(crate) created_at_ns: u64,
    pub(crate) source_root: String,
    pub(crate) file_count: u64,
    pub(crate) total_raw_bytes: u64,
    pub(crate) entries: Vec<BackupSnapshotEntry>,
    pub(crate) removed: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BackupSnapshotEntry {
    pub(crate) path: String,
    #[serde(
        serialize_with = "serialize_hash32",
        deserialize_with = "deserialize_hash32"
    )]
    pub(crate) content_hash: [u8; 32],
    pub(crate) size: u64,
    pub(crate) mtime_ns: u64,
    pub(crate) created_time_ns: Option<u64>,
    pub(crate) win_attributes: u32,
    pub(crate) file_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum HashManifestError {
    #[error("manifest file not found: {0}")]
    NotFound(String),
    #[error("unsupported backup source: {0}")]
    UnsupportedSource(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("manifest parse error: {0}")]
    Parse(String),
}

impl BackupSnapshotManifest {
    pub(crate) fn new(
        source_root: String,
        created_at_ns: u64,
        entries: Vec<BackupSnapshotEntry>,
        removed: Vec<String>,
    ) -> Self {
        let total_raw_bytes = entries.iter().map(|entry| entry.size).sum();
        Self {
            version: HASH_MANIFEST_VERSION,
            snapshot_id: Ulid::new().to_string(),
            created_at_ns,
            source_root,
            file_count: entries.len() as u64,
            total_raw_bytes,
            entries,
            removed,
        }
    }
}

pub(crate) fn serialize_backup_snapshot_manifest(
    manifest: &BackupSnapshotManifest,
) -> Result<Vec<u8>, HashManifestError> {
    serde_json::to_vec_pretty(manifest).map_err(|err| HashManifestError::Parse(err.to_string()))
}

pub(crate) fn write_backup_snapshot_manifest(
    backup_root: &Path,
    manifest: &BackupSnapshotManifest,
) -> Result<(), HashManifestError> {
    let bytes = serialize_backup_snapshot_manifest(manifest)?;
    fs::write(backup_root.join(HASH_MANIFEST_FILE), bytes)
        .map_err(|err| HashManifestError::Io(err.to_string()))
}

pub(crate) fn read_backup_snapshot_manifest(
    backup_path: &Path,
) -> Result<BackupSnapshotManifest, HashManifestError> {
    if backup_path.is_dir() {
        return read_backup_snapshot_manifest_from_dir(backup_path);
    }
    if backup_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        return read_backup_snapshot_manifest_from_zip(backup_path);
    }
    Err(HashManifestError::UnsupportedSource(
        backup_path.display().to_string(),
    ))
}

fn read_backup_snapshot_manifest_from_dir(
    backup_root: &Path,
) -> Result<BackupSnapshotManifest, HashManifestError> {
    let manifest_path = backup_root.join(HASH_MANIFEST_FILE);
    let bytes = fs::read(&manifest_path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            HashManifestError::NotFound(manifest_path.display().to_string())
        } else {
            HashManifestError::Io(err.to_string())
        }
    })?;
    serde_json::from_slice(&bytes).map_err(|err| HashManifestError::Parse(err.to_string()))
}

fn read_backup_snapshot_manifest_from_zip(
    zip_path: &Path,
) -> Result<BackupSnapshotManifest, HashManifestError> {
    let file = fs::File::open(zip_path).map_err(|err| HashManifestError::Io(err.to_string()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|err| HashManifestError::Parse(err.to_string()))?;
    let mut entry = archive.by_name(HASH_MANIFEST_FILE).map_err(|_| {
        HashManifestError::NotFound(format!("{}::{}", zip_path.display(), HASH_MANIFEST_FILE))
    })?;
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|err| HashManifestError::Io(err.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|err| HashManifestError::Parse(err.to_string()))
}

fn serialize_hash32<S>(value: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&encode_hash_hex(value))
}

fn deserialize_hash32<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    decode_hash_hex(&value).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;
    use ulid::Ulid;

    use super::{
        BackupSnapshotEntry, BackupSnapshotManifest, HASH_MANIFEST_FILE, HASH_MANIFEST_VERSION,
        read_backup_snapshot_manifest, serialize_backup_snapshot_manifest,
        write_backup_snapshot_manifest,
    };

    fn sample_manifest() -> BackupSnapshotManifest {
        BackupSnapshotManifest {
            version: HASH_MANIFEST_VERSION,
            snapshot_id: "01JQ7J3N4QCG7N2T1DJ7C9ZK4Q".to_string(),
            created_at_ns: 1_774_320_000_000_000_000,
            source_root: "D:\\project".to_string(),
            file_count: 1,
            total_raw_bytes: 1234,
            entries: vec![BackupSnapshotEntry {
                path: "src/main.rs".to_string(),
                content_hash: [0xab; 32],
                size: 1234,
                mtime_ns: 1_774_319_900_000_000_000,
                created_time_ns: Some(1_774_319_000_000_000_000),
                win_attributes: 32,
                file_id: Some("0000000000000000000000000000abcd".to_string()),
            }],
            removed: Vec::new(),
        }
    }

    #[test]
    fn backup_snapshot_manifest_new_generates_ulid_and_totals() {
        let manifest = BackupSnapshotManifest::new(
            "D:\\project".to_string(),
            1_774_320_000_000_000_000,
            vec![
                BackupSnapshotEntry {
                    path: "a.txt".to_string(),
                    content_hash: [0x11; 32],
                    size: 3,
                    mtime_ns: 1,
                    created_time_ns: None,
                    win_attributes: 32,
                    file_id: None,
                },
                BackupSnapshotEntry {
                    path: "b.txt".to_string(),
                    content_hash: [0x22; 32],
                    size: 5,
                    mtime_ns: 2,
                    created_time_ns: Some(3),
                    win_attributes: 1,
                    file_id: Some("file-2".to_string()),
                },
            ],
            vec!["gone.txt".to_string()],
        );

        assert_eq!(manifest.version, HASH_MANIFEST_VERSION);
        assert_eq!(manifest.file_count, 2);
        assert_eq!(manifest.total_raw_bytes, 8);
        assert_eq!(manifest.removed, vec!["gone.txt".to_string()]);
        assert!(Ulid::from_string(&manifest.snapshot_id).is_ok());
    }

    #[test]
    fn snapshot_manifest_json_roundtrips() {
        let manifest = sample_manifest();
        let json = serialize_backup_snapshot_manifest(&manifest).unwrap();
        let roundtrip: BackupSnapshotManifest = serde_json::from_slice(&json).unwrap();
        assert_eq!(roundtrip, manifest);
    }

    #[test]
    fn snapshot_manifest_version_is_two() {
        assert_eq!(sample_manifest().version, 2);
    }

    #[test]
    fn snapshot_manifest_removed_serializes_as_empty_array() {
        let manifest = sample_manifest();
        let value: serde_json::Value =
            serde_json::from_slice(&serialize_backup_snapshot_manifest(&manifest).unwrap()).unwrap();
        assert_eq!(value["removed"], serde_json::json!([]));
    }

    #[test]
    fn snapshot_manifest_content_hash_serializes_as_lowercase_hex() {
        let manifest = sample_manifest();
        let value: serde_json::Value =
            serde_json::from_slice(&serialize_backup_snapshot_manifest(&manifest).unwrap()).unwrap();
        let hash = value["entries"][0]["content_hash"].as_str().unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()));
    }

    #[test]
    fn snapshot_manifest_uses_relative_forward_slash_paths() {
        let manifest = sample_manifest();
        let value: serde_json::Value =
            serde_json::from_slice(&serialize_backup_snapshot_manifest(&manifest).unwrap()).unwrap();
        let path = value["entries"][0]["path"].as_str().unwrap();
        assert_eq!(path, "src/main.rs");
        assert!(!path.contains('\\'));
    }

    #[test]
    fn snapshot_manifest_preserves_unix_epoch_nanosecond_mtime() {
        let manifest = sample_manifest();
        let json = serialize_backup_snapshot_manifest(&manifest).unwrap();
        let roundtrip: BackupSnapshotManifest = serde_json::from_slice(&json).unwrap();
        assert_eq!(
            roundtrip.entries[0].mtime_ns,
            1_774_319_900_000_000_000
        );
    }

    #[test]
    fn write_and_read_snapshot_manifest_from_dir() {
        let dir = tempdir().unwrap();
        let manifest = sample_manifest();
        write_backup_snapshot_manifest(dir.path(), &manifest).unwrap();

        let loaded = read_backup_snapshot_manifest(dir.path()).unwrap();
        assert_eq!(loaded, manifest);
    }

    #[test]
    fn read_snapshot_manifest_from_zip() {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("backup.zip");
        let manifest = sample_manifest();
        let bytes = serialize_backup_snapshot_manifest(&manifest).unwrap();

        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file(HASH_MANIFEST_FILE, options).unwrap();
        writer.write_all(&bytes).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        std::fs::write(&zip_path, bytes).unwrap();

        let loaded = read_backup_snapshot_manifest(&zip_path).unwrap();
        assert_eq!(loaded, manifest);
    }
}
