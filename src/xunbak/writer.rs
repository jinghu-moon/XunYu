use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rayon::prelude::*;
use serde_json::json;
use ulid::Ulid;

use crate::xunbak::blob::write_blob_record;
use crate::xunbak::checkpoint::{
    CheckpointError, CheckpointPayload, compute_manifest_hash, write_checkpoint_record,
};
use crate::xunbak::codec::should_skip_compress;
use crate::xunbak::constants::FLAG_SPLIT;
use crate::xunbak::constants::{
    Codec, FOOTER_SIZE, HEADER_SIZE, RECORD_PREFIX_SIZE, XUNBAK_READER_VERSION,
    XUNBAK_WRITE_VERSION,
};
use crate::xunbak::footer::{Footer, FooterError};
use crate::xunbak::header::Header;
use crate::xunbak::manifest::{
    ManifestBody, ManifestCodec, ManifestEntry, ManifestError, ManifestPrefix, ManifestType,
    normalize_path, write_manifest_record,
};
use crate::xunbak::reader::{ContainerReader, ReaderError};

#[derive(Debug)]
pub struct ContainerWriter {
    path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupOptions {
    pub codec: Codec,
    pub zstd_level: i32,
    pub split_size: Option<u64>,
}

impl Default for BackupOptions {
    fn default() -> Self {
        Self {
            codec: Codec::ZSTD,
            zstd_level: 1,
            split_size: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupResult {
    pub container_path: PathBuf,
    pub file_count: usize,
    pub blob_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressEvent {
    pub processed_files: usize,
    pub total_files: usize,
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompressedBlob {
    pub path: String,
    pub header: crate::xunbak::blob::BlobHeader,
    pub record_len: u64,
    pub record_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateResult {
    pub container_path: PathBuf,
    pub added_blob_count: usize,
    pub file_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlobLocator {
    pub blob_id: [u8; 32],
    pub codec: Codec,
    pub blob_offset: u64,
    pub blob_len: u64,
    pub volume_index: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffKind {
    New,
    Modified,
    Unchanged,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffEntry {
    pub path: String,
    pub kind: DiffKind,
}

#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error(transparent)]
    Checkpoint(#[from] CheckpointError),
    #[error(transparent)]
    Footer(#[from] FooterError),
    #[error(transparent)]
    Reader(#[from] ReaderError),
    #[error("invalid split state for update")]
    InvalidSplitState,
    #[error("record too large for split size: record={record_len} split_size={split_size}")]
    RecordTooLargeForSplit { record_len: u64, split_size: u64 },
    #[error("split size mismatch: expected {expected}, got {actual}")]
    SplitSizeMismatch { expected: u64, actual: u64 },
}

impl ContainerWriter {
    pub fn create(path: &Path) -> Result<Self, WriterError> {
        let mut file = File::create(path).map_err(|err| WriterError::Io(err.to_string()))?;

        let header = Header {
            write_version: XUNBAK_WRITE_VERSION,
            min_reader_version: XUNBAK_READER_VERSION,
            flags: 0,
            created_at_unix: now_unix_secs(),
            split: None,
        };
        file.write_all(&header.to_bytes())
            .map_err(|err| WriterError::Io(err.to_string()))?;

        let manifest_prefix = ManifestPrefix {
            manifest_codec: ManifestCodec::JSON,
            manifest_type: ManifestType::FULL,
            manifest_version: 1,
        };
        let manifest_body = ManifestBody {
            snapshot_id: Ulid::new().to_string(),
            base_snapshot_id: None,
            created_at: now_unix_secs(),
            source_root: String::new(),
            snapshot_context: json!({}),
            file_count: 0,
            total_raw_bytes: 0,
            entries: vec![],
            removed: vec![],
        };
        let mut manifest_record = Vec::new();
        write_manifest_record(&mut manifest_record, manifest_prefix, &manifest_body)?;
        let manifest_payload = &manifest_record[crate::xunbak::constants::RECORD_PREFIX_SIZE..];
        let manifest_hash = compute_manifest_hash(manifest_payload);
        let manifest_offset = HEADER_SIZE as u64;
        let manifest_len = manifest_record.len() as u64;

        let checkpoint_offset = manifest_offset + manifest_len;
        let checkpoint_record_len = crate::xunbak::constants::RECORD_PREFIX_SIZE as u64
            + crate::xunbak::constants::CHECKPOINT_PAYLOAD_SIZE as u64;
        let total_container_bytes = checkpoint_offset + checkpoint_record_len + FOOTER_SIZE as u64;
        let checkpoint_payload = CheckpointPayload {
            snapshot_id: Ulid::from_string(&manifest_body.snapshot_id)
                .expect("generated ULID must parse")
                .to_bytes(),
            manifest_offset,
            manifest_len,
            manifest_hash,
            container_end: total_container_bytes,
            blob_count: 0,
            referenced_blob_bytes: 0,
            total_container_bytes,
            prev_checkpoint_offset: 0,
            total_volumes: 1,
        };
        let mut checkpoint_record = Vec::new();
        write_checkpoint_record(&mut checkpoint_record, &checkpoint_payload)?;

        let footer = Footer { checkpoint_offset };

        file.write_all(&manifest_record)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        file.write_all(&checkpoint_record)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        file.write_all(&footer.to_bytes())
            .map_err(|err| WriterError::Io(err.to_string()))?;

        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn backup(
        container_path: &Path,
        source_dir: &Path,
        options: &BackupOptions,
    ) -> Result<BackupResult, WriterError> {
        let mut noop = |_event: ProgressEvent| {};
        Self::backup_with_progress(container_path, source_dir, options, &mut noop)
    }

    pub fn backup_with_progress(
        container_path: &Path,
        source_dir: &Path,
        options: &BackupOptions,
        progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<BackupResult, WriterError> {
        if options.split_size.is_some() {
            return backup_split_with_progress(container_path, source_dir, options, progress);
        }
        let mut file =
            File::create(container_path).map_err(|err| WriterError::Io(err.to_string()))?;
        let header = Header {
            write_version: XUNBAK_WRITE_VERSION,
            min_reader_version: XUNBAK_READER_VERSION,
            flags: 0,
            created_at_unix: now_unix_secs(),
            split: None,
        };
        file.write_all(&header.to_bytes())
            .map_err(|err| WriterError::Io(err.to_string()))?;

        let mut scanned = Vec::new();
        collect_files(source_dir, source_dir, &mut scanned, Some(container_path))?;
        scanned.sort_by(|a, b| a.rel.cmp(&b.rel));
        let total_files = scanned.len();
        let total_bytes: u64 = scanned.iter().map(|item| item.size).sum();
        let started = Instant::now();

        let mut entries = Vec::with_capacity(scanned.len());
        let mut content_index: HashMap<[u8; 32], BlobLocator> = HashMap::new();
        let mut next_offset = HEADER_SIZE as u64;
        let mut total_raw_bytes = 0u64;
        let mut processed_files = 0usize;
        let mut processed_bytes = 0u64;

        for item in &scanned {
            total_raw_bytes += item.size;
            let content =
                std::fs::read(&item.path).map_err(|err| WriterError::Io(err.to_string()))?;
            let content_hash = *blake3::hash(&content).as_bytes();
            if let Some(locator) = content_index.get(&content_hash) {
                entries.push(ManifestEntry {
                    path: item.rel.clone(),
                    blob_id: locator.blob_id,
                    content_hash,
                    size: item.size,
                    mtime_ns: item.mtime_ns,
                    created_time_ns: item.created_time_ns,
                    win_attributes: item.win_attributes,
                    codec: locator.codec,
                    blob_offset: locator.blob_offset,
                    blob_len: locator.blob_len,
                    volume_index: locator.volume_index,
                    parts: None,
                    ext: None,
                });
                processed_files += 1;
                processed_bytes += item.size;
                progress(ProgressEvent {
                    processed_files,
                    total_files,
                    processed_bytes,
                    total_bytes,
                    elapsed_ms: started.elapsed().as_millis(),
                });
                continue;
            }
            let codec = effective_codec_for_path(item, options.codec);
            let write = write_blob_record(&mut file, &content, codec, options.zstd_level)
                .map_err(|err| WriterError::Io(err.to_string()))?;
            let blob_len = RECORD_PREFIX_SIZE as u64 + write.record_len;
            entries.push(ManifestEntry {
                path: item.rel.clone(),
                blob_id: write.header.blob_id,
                content_hash,
                size: item.size,
                mtime_ns: item.mtime_ns,
                created_time_ns: item.created_time_ns,
                win_attributes: item.win_attributes,
                codec: write.header.codec,
                blob_offset: next_offset,
                blob_len,
                volume_index: 0,
                parts: None,
                ext: None,
            });
            content_index.insert(
                content_hash,
                BlobLocator {
                    blob_id: write.header.blob_id,
                    codec: write.header.codec,
                    blob_offset: next_offset,
                    blob_len,
                    volume_index: 0,
                },
            );
            next_offset += blob_len;
            processed_files += 1;
            processed_bytes += item.size;
            progress(ProgressEvent {
                processed_files,
                total_files,
                processed_bytes,
                total_bytes,
                elapsed_ms: started.elapsed().as_millis(),
            });
        }

        let referenced_blob_bytes = sum_unique_blob_bytes(&entries);
        let blob_count = unique_blob_count(&entries) as u64;
        let manifest_prefix = ManifestPrefix {
            manifest_codec: ManifestCodec::JSON,
            manifest_type: ManifestType::FULL,
            manifest_version: 1,
        };
        let manifest_body = ManifestBody {
            snapshot_id: Ulid::new().to_string(),
            base_snapshot_id: None,
            created_at: now_unix_secs(),
            source_root: source_dir.to_string_lossy().into_owned(),
            snapshot_context: json!({
                "hostname": std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown-host".to_string()),
                "username": std::env::var("USERNAME").unwrap_or_else(|_| "unknown-user".to_string()),
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "xunyu_version": env!("CARGO_PKG_VERSION"),
                "command_mode": "backup",
                "compression_profile": options.codec.as_u8(),
            }),
            file_count: entries.len() as u64,
            total_raw_bytes,
            entries,
            removed: vec![],
        };
        let mut manifest_record = Vec::new();
        write_manifest_record(&mut manifest_record, manifest_prefix, &manifest_body)?;
        let manifest_payload = &manifest_record[RECORD_PREFIX_SIZE..];
        let manifest_hash = compute_manifest_hash(manifest_payload);
        let manifest_offset = next_offset;
        let manifest_len = manifest_record.len() as u64;
        let checkpoint_offset = manifest_offset + manifest_len;
        let checkpoint_record_len =
            RECORD_PREFIX_SIZE as u64 + crate::xunbak::constants::CHECKPOINT_PAYLOAD_SIZE as u64;
        let total_container_bytes = checkpoint_offset + checkpoint_record_len + FOOTER_SIZE as u64;
        let checkpoint_payload = CheckpointPayload {
            snapshot_id: Ulid::from_string(&manifest_body.snapshot_id)
                .expect("generated ULID must parse")
                .to_bytes(),
            manifest_offset,
            manifest_len,
            manifest_hash,
            container_end: total_container_bytes,
            blob_count,
            referenced_blob_bytes,
            total_container_bytes,
            prev_checkpoint_offset: 0,
            total_volumes: 1,
        };
        let mut checkpoint_record = Vec::new();
        write_checkpoint_record(&mut checkpoint_record, &checkpoint_payload)?;
        let footer = Footer { checkpoint_offset };

        file.write_all(&manifest_record)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        file.write_all(&checkpoint_record)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        file.write_all(&footer.to_bytes())
            .map_err(|err| WriterError::Io(err.to_string()))?;

        Ok(BackupResult {
            container_path: container_path.to_path_buf(),
            file_count: scanned.len(),
            blob_count: blob_count as usize,
        })
    }

    pub fn update(
        container_path: &Path,
        source_dir: &Path,
        options: &BackupOptions,
    ) -> Result<UpdateResult, WriterError> {
        let mut noop = |_event: ProgressEvent| {};
        Self::update_with_progress(container_path, source_dir, options, &mut noop)
    }

    pub fn update_with_progress(
        container_path: &Path,
        source_dir: &Path,
        options: &BackupOptions,
        progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<UpdateResult, WriterError> {
        let reader = ContainerReader::open(container_path)?;
        if reader.is_split {
            return update_split_with_progress(
                container_path,
                source_dir,
                options,
                progress,
                &reader,
            );
        }
        let baseline = reader.load_manifest()?;
        let mut content_index = build_content_hash_index(&baseline);
        let mut scanned = Vec::new();
        collect_files(source_dir, source_dir, &mut scanned, Some(container_path))?;
        scanned.sort_by(|a, b| a.rel.cmp(&b.rel));
        let _diff = diff_against_manifest(&scanned, &baseline);
        let total_files = scanned.len();
        let total_bytes: u64 = scanned.iter().map(|item| item.size).sum();
        let started = Instant::now();

        let old_footer_offset = reader.file_size - FOOTER_SIZE as u64;

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(container_path)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        file.set_len(old_footer_offset)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        file.seek(SeekFrom::Start(old_footer_offset))
            .map_err(|err| WriterError::Io(err.to_string()))?;

        let mut next_offset = old_footer_offset;
        let mut entries = Vec::with_capacity(scanned.len());
        let mut added_blob_count = 0usize;
        let mut total_raw_bytes = 0u64;
        let mut processed_files = 0usize;
        let mut processed_bytes = 0u64;

        for item in &scanned {
            total_raw_bytes += item.size;
            if let Some(old) = baseline.entries.iter().find(|entry| entry.path == item.rel)
                && old.size == item.size
                && old.mtime_ns == item.mtime_ns
            {
                entries.push(old.clone());
                processed_files += 1;
                processed_bytes += item.size;
                progress(ProgressEvent {
                    processed_files,
                    total_files,
                    processed_bytes,
                    total_bytes,
                    elapsed_ms: started.elapsed().as_millis(),
                });
                continue;
            }

            let content =
                std::fs::read(&item.path).map_err(|err| WriterError::Io(err.to_string()))?;
            let content_hash = *blake3::hash(&content).as_bytes();
            if let Some(locator) = content_index.get(&content_hash) {
                entries.push(ManifestEntry {
                    path: item.rel.clone(),
                    blob_id: locator.blob_id,
                    content_hash,
                    size: item.size,
                    mtime_ns: item.mtime_ns,
                    created_time_ns: item.created_time_ns,
                    win_attributes: item.win_attributes,
                    codec: locator.codec,
                    blob_offset: locator.blob_offset,
                    blob_len: locator.blob_len,
                    volume_index: locator.volume_index,
                    parts: None,
                    ext: None,
                });
                processed_files += 1;
                processed_bytes += item.size;
                progress(ProgressEvent {
                    processed_files,
                    total_files,
                    processed_bytes,
                    total_bytes,
                    elapsed_ms: started.elapsed().as_millis(),
                });
                continue;
            }

            let codec = effective_codec_for_path(item, options.codec);
            let write = write_blob_record(&mut file, &content, codec, options.zstd_level)
                .map_err(|err| WriterError::Io(err.to_string()))?;
            let blob_len = RECORD_PREFIX_SIZE as u64 + write.record_len;
            entries.push(ManifestEntry {
                path: item.rel.clone(),
                blob_id: write.header.blob_id,
                content_hash,
                size: item.size,
                mtime_ns: item.mtime_ns,
                created_time_ns: item.created_time_ns,
                win_attributes: item.win_attributes,
                codec: write.header.codec,
                blob_offset: next_offset,
                blob_len,
                volume_index: 0,
                parts: None,
                ext: None,
            });
            content_index.insert(
                content_hash,
                BlobLocator {
                    blob_id: write.header.blob_id,
                    codec: write.header.codec,
                    blob_offset: next_offset,
                    blob_len,
                    volume_index: 0,
                },
            );
            next_offset += blob_len;
            added_blob_count += 1;
            processed_files += 1;
            processed_bytes += item.size;
            progress(ProgressEvent {
                processed_files,
                total_files,
                processed_bytes,
                total_bytes,
                elapsed_ms: started.elapsed().as_millis(),
            });
        }

        let referenced_blob_bytes = sum_unique_blob_bytes(&entries);
        let entry_count = entries.len() as u64;
        let blob_count = unique_blob_count(&entries) as u64;
        let manifest_prefix = ManifestPrefix {
            manifest_codec: ManifestCodec::JSON,
            manifest_type: ManifestType::FULL,
            manifest_version: 1,
        };
        let manifest_body = ManifestBody {
            snapshot_id: Ulid::new().to_string(),
            base_snapshot_id: None,
            created_at: now_unix_secs(),
            source_root: source_dir.to_string_lossy().into_owned(),
            snapshot_context: json!({
                "hostname": std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown-host".to_string()),
                "username": std::env::var("USERNAME").unwrap_or_else(|_| "unknown-user".to_string()),
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "xunyu_version": env!("CARGO_PKG_VERSION"),
                "command_mode": "update",
                "compression_profile": options.codec.as_u8(),
            }),
            file_count: entry_count,
            total_raw_bytes,
            entries,
            removed: vec![],
        };
        let mut manifest_record = Vec::new();
        write_manifest_record(&mut manifest_record, manifest_prefix, &manifest_body)?;
        file.write_all(&manifest_record)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        file.sync_all()
            .map_err(|err| WriterError::Io(err.to_string()))?;

        let manifest_payload = &manifest_record[RECORD_PREFIX_SIZE..];
        let manifest_hash = compute_manifest_hash(manifest_payload);
        let manifest_offset = next_offset;
        let manifest_len = manifest_record.len() as u64;
        let checkpoint_offset = manifest_offset + manifest_len;
        let checkpoint_record_len =
            RECORD_PREFIX_SIZE as u64 + crate::xunbak::constants::CHECKPOINT_PAYLOAD_SIZE as u64;
        let total_container_bytes = checkpoint_offset + checkpoint_record_len + FOOTER_SIZE as u64;
        let checkpoint_payload = CheckpointPayload {
            snapshot_id: Ulid::from_string(&manifest_body.snapshot_id)
                .expect("generated ULID must parse")
                .to_bytes(),
            manifest_offset,
            manifest_len,
            manifest_hash,
            container_end: total_container_bytes,
            blob_count,
            referenced_blob_bytes,
            total_container_bytes,
            prev_checkpoint_offset: 0,
            total_volumes: 1,
        };
        let mut checkpoint_record = Vec::new();
        write_checkpoint_record(&mut checkpoint_record, &checkpoint_payload)?;
        file.write_all(&checkpoint_record)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        let footer = Footer { checkpoint_offset };
        file.write_all(&footer.to_bytes())
            .map_err(|err| WriterError::Io(err.to_string()))?;
        file.sync_all()
            .map_err(|err| WriterError::Io(err.to_string()))?;
        Ok(UpdateResult {
            container_path: container_path.to_path_buf(),
            added_blob_count,
            file_count: scanned.len(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScannedSourceFile {
    pub rel: String,
    pub path: PathBuf,
    pub size: u64,
    pub mtime_ns: u64,
    pub created_time_ns: u64,
    pub win_attributes: u32,
}

#[derive(Clone, Copy, Debug)]
struct ScannedManifestEntryView {
    size: u64,
    mtime_ns: u64,
}

fn collect_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<ScannedSourceFile>,
    output_base: Option<&Path>,
) -> Result<(), WriterError> {
    for entry in std::fs::read_dir(dir).map_err(|err| WriterError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| WriterError::Io(err.to_string()))?;
        let path = entry.path();
        if output_base.is_some_and(|base| is_output_artifact(&path, base)) {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|err| WriterError::Io(err.to_string()))?;
        if file_type.is_dir() {
            collect_files(root, &path, out, output_base)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|err| WriterError::Io(err.to_string()))?;
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        out.push(ScannedSourceFile {
            rel: normalize_path(&rel).map_err(WriterError::Manifest)?,
            path,
            size: metadata.len(),
            mtime_ns: metadata
                .modified()
                .ok()
                .and_then(system_time_to_unix_ns)
                .unwrap_or(0),
            created_time_ns: metadata
                .created()
                .ok()
                .and_then(system_time_to_unix_ns)
                .unwrap_or(0),
            win_attributes: file_attributes(&metadata),
        });
    }
    Ok(())
}

pub fn build_content_hash_index(manifest: &ManifestBody) -> HashMap<[u8; 32], BlobLocator> {
    manifest
        .entries
        .iter()
        .map(|entry| {
            (
                entry.content_hash,
                BlobLocator {
                    blob_id: entry.blob_id,
                    codec: entry.codec,
                    blob_offset: entry.blob_offset,
                    blob_len: entry.blob_len,
                    volume_index: entry.volume_index,
                },
            )
        })
        .collect()
}

pub fn diff_against_manifest(
    scan_result: &[ScannedSourceFile],
    manifest: &ManifestBody,
) -> Vec<DiffEntry> {
    let mut baseline: HashMap<&str, ScannedManifestEntryView> = manifest
        .entries
        .iter()
        .map(|entry| {
            (
                entry.path.as_str(),
                ScannedManifestEntryView {
                    size: entry.size,
                    mtime_ns: entry.mtime_ns,
                },
            )
        })
        .collect();

    let mut diff = Vec::new();
    for item in scan_result {
        match baseline.remove(item.rel.as_str()) {
            Some(old) if old.size == item.size && old.mtime_ns == item.mtime_ns => {
                diff.push(DiffEntry {
                    path: item.rel.clone(),
                    kind: DiffKind::Unchanged,
                });
            }
            Some(_) => diff.push(DiffEntry {
                path: item.rel.clone(),
                kind: DiffKind::Modified,
            }),
            None => diff.push(DiffEntry {
                path: item.rel.clone(),
                kind: DiffKind::New,
            }),
        }
    }
    for path in baseline.keys() {
        diff.push(DiffEntry {
            path: (*path).to_string(),
            kind: DiffKind::Deleted,
        });
    }
    diff.sort_by(|a, b| a.path.cmp(&b.path));
    diff
}

fn sum_unique_blob_bytes(entries: &[ManifestEntry]) -> u64 {
    let mut seen = HashMap::new();
    for entry in entries {
        seen.entry((entry.volume_index, entry.blob_offset, entry.blob_len))
            .or_insert(entry.blob_len);
    }
    seen.values().copied().sum()
}

fn unique_blob_count(entries: &[ManifestEntry]) -> usize {
    let mut seen = HashMap::new();
    for entry in entries {
        seen.entry((entry.volume_index, entry.blob_offset, entry.blob_len))
            .or_insert(());
    }
    seen.len()
}

fn system_time_to_unix_ns(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos() as u64)
}

fn effective_codec_for_path(file: &ScannedSourceFile, requested: Codec) -> Codec {
    if requested == Codec::NONE {
        return Codec::NONE;
    }
    let ext = file
        .path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if should_skip_compress(ext) {
        Codec::NONE
    } else {
        requested
    }
}

pub fn parallel_compress_pipeline(
    files: &[ScannedSourceFile],
    codec: Codec,
    level: i32,
    num_threads: usize,
) -> Result<Vec<CompressedBlob>, WriterError> {
    let work = || {
        files
            .par_iter()
            .map(|file| {
                let requested = effective_codec_for_path(file, codec);
                let content =
                    std::fs::read(&file.path).map_err(|err| WriterError::Io(err.to_string()))?;
                let mut record_bytes = Vec::new();
                let write = write_blob_record(&mut record_bytes, &content, requested, level)
                    .map_err(|err| WriterError::Io(err.to_string()))?;
                Ok(CompressedBlob {
                    path: file.rel.clone(),
                    header: write.header,
                    record_len: write.record_len,
                    record_bytes,
                })
            })
            .collect::<Result<Vec<_>, WriterError>>()
    };

    if num_threads <= 1 {
        files
            .iter()
            .map(|file| {
                let requested = effective_codec_for_path(file, codec);
                let content =
                    std::fs::read(&file.path).map_err(|err| WriterError::Io(err.to_string()))?;
                let mut record_bytes = Vec::new();
                let write = write_blob_record(&mut record_bytes, &content, requested, level)
                    .map_err(|err| WriterError::Io(err.to_string()))?;
                Ok(CompressedBlob {
                    path: file.rel.clone(),
                    header: write.header,
                    record_len: write.record_len,
                    record_bytes,
                })
            })
            .collect()
    } else {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .map_err(|err| WriterError::Io(err.to_string()))?
            .install(work)
    }
}

#[cfg(windows)]
fn file_attributes(metadata: &std::fs::Metadata) -> u32 {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes()
}

#[cfg(not(windows))]
fn file_attributes(_metadata: &std::fs::Metadata) -> u32 {
    0
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn is_output_artifact(path: &Path, base: &Path) -> bool {
    if path == base {
        return true;
    }
    let base_str = base.to_string_lossy();
    let path_str = path.to_string_lossy();
    path_str == format!("{base_str}.lock")
        || (path_str.starts_with(&format!("{base_str}."))
            && path_str.chars().rev().take(3).all(|ch| ch.is_ascii_digit()))
}

struct VolumeOutput {
    base_path: PathBuf,
    split_size: u64,
    set_id: u64,
    current_index: u16,
    current_len: u64,
    file: File,
    paths: Vec<PathBuf>,
}

impl VolumeOutput {
    fn new(base_path: &Path, split_size: u64) -> Result<Self, WriterError> {
        let set_id = generate_set_id(base_path);
        let first_path = split_volume_path(base_path, 0);
        let mut file = File::create(&first_path).map_err(|err| WriterError::Io(err.to_string()))?;
        let header = Header {
            write_version: XUNBAK_WRITE_VERSION,
            min_reader_version: XUNBAK_READER_VERSION,
            flags: FLAG_SPLIT,
            created_at_unix: now_unix_secs(),
            split: Some(crate::xunbak::header::SplitHeader {
                volume_index: 0,
                split_size,
                set_id,
            }),
        };
        file.write_all(&header.to_bytes())
            .map_err(|err| WriterError::Io(err.to_string()))?;
        Ok(Self {
            base_path: base_path.to_path_buf(),
            split_size,
            set_id,
            current_index: 0,
            current_len: HEADER_SIZE as u64,
            file,
            paths: vec![first_path],
        })
    }

    fn write_record(
        &mut self,
        record: &[u8],
        trailing_reserve: u64,
    ) -> Result<(u16, u64), WriterError> {
        let record_len = record.len() as u64;
        self.ensure_capacity(record_len, trailing_reserve)?;
        let offset = self.current_len;
        self.file
            .write_all(record)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        self.current_len += record_len;
        Ok((self.current_index, offset))
    }

    fn write_footer(&mut self, checkpoint_offset: u64) -> Result<(), WriterError> {
        let footer = Footer { checkpoint_offset };
        let footer_bytes = footer.to_bytes();
        self.file
            .write_all(&footer_bytes)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        self.current_len += footer_bytes.len() as u64;
        Ok(())
    }

    fn total_volumes(&self) -> u16 {
        self.paths.len() as u16
    }

    fn last_volume_path(&self) -> &Path {
        self.paths.last().expect("at least one volume")
    }

    fn ensure_capacity(
        &mut self,
        record_len: u64,
        trailing_reserve: u64,
    ) -> Result<(), WriterError> {
        if HEADER_SIZE as u64 + record_len + trailing_reserve > self.split_size {
            return Err(WriterError::RecordTooLargeForSplit {
                record_len,
                split_size: self.split_size,
            });
        }
        if self.current_len + record_len + trailing_reserve <= self.split_size {
            return Ok(());
        }
        self.rotate()
    }

    fn rotate(&mut self) -> Result<(), WriterError> {
        self.current_index = self.current_index.saturating_add(1);
        let path = split_volume_path(&self.base_path, self.current_index);
        let mut file = File::create(&path).map_err(|err| WriterError::Io(err.to_string()))?;
        let header = Header {
            write_version: XUNBAK_WRITE_VERSION,
            min_reader_version: XUNBAK_READER_VERSION,
            flags: FLAG_SPLIT,
            created_at_unix: now_unix_secs(),
            split: Some(crate::xunbak::header::SplitHeader {
                volume_index: self.current_index,
                split_size: self.split_size,
                set_id: self.set_id,
            }),
        };
        file.write_all(&header.to_bytes())
            .map_err(|err| WriterError::Io(err.to_string()))?;
        self.file = file;
        self.current_len = HEADER_SIZE as u64;
        self.paths.push(path);
        Ok(())
    }

    fn from_existing_split(
        base_path: &Path,
        split_size: u64,
        set_id: u64,
        existing_paths: &[PathBuf],
        last_volume_len_without_footer: u64,
    ) -> Result<Self, WriterError> {
        let last_path = existing_paths
            .last()
            .cloned()
            .ok_or_else(|| WriterError::Io("missing split volumes".to_string()))?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&last_path)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        file.set_len(last_volume_len_without_footer)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        file.seek(SeekFrom::Start(last_volume_len_without_footer))
            .map_err(|err| WriterError::Io(err.to_string()))?;
        Ok(Self {
            base_path: base_path.to_path_buf(),
            split_size,
            set_id,
            current_index: (existing_paths.len() - 1) as u16,
            current_len: last_volume_len_without_footer,
            file,
            paths: existing_paths.to_vec(),
        })
    }
}

fn split_volume_path(base_path: &Path, volume_index: u16) -> PathBuf {
    PathBuf::from(format!("{}.{:03}", base_path.display(), volume_index + 1))
}

fn split_base_path(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if raw.len() >= 4
        && raw.as_bytes()[raw.len() - 4] == b'.'
        && raw[raw.len() - 3..].chars().all(|ch| ch.is_ascii_digit())
    {
        PathBuf::from(raw[..raw.len() - 4].to_string())
    } else {
        path.to_path_buf()
    }
}

fn generate_set_id(base_path: &Path) -> u64 {
    let seed = format!("{}:{}", base_path.display(), now_unix_secs());
    let hash = blake3::hash(seed.as_bytes());
    u64::from_le_bytes(hash.as_bytes()[..8].try_into().expect("fixed length"))
}

fn backup_split_with_progress(
    container_path: &Path,
    source_dir: &Path,
    options: &BackupOptions,
    progress: &mut dyn FnMut(ProgressEvent),
) -> Result<BackupResult, WriterError> {
    let split_size = options.split_size.expect("checked by caller");
    cleanup_existing_split_outputs(container_path)?;
    let mut output = VolumeOutput::new(container_path, split_size)?;
    let mut scanned = Vec::new();
    collect_files(source_dir, source_dir, &mut scanned, Some(container_path))?;
    scanned.sort_by(|a, b| a.rel.cmp(&b.rel));
    let total_files = scanned.len();
    let total_bytes: u64 = scanned.iter().map(|item| item.size).sum();
    let started = Instant::now();

    let mut entries = Vec::with_capacity(scanned.len());
    let mut content_index: HashMap<[u8; 32], BlobLocator> = HashMap::new();
    let mut total_raw_bytes = 0u64;
    let mut processed_files = 0usize;
    let mut processed_bytes = 0u64;

    for item in &scanned {
        total_raw_bytes += item.size;
        let content = fs::read(&item.path).map_err(|err| WriterError::Io(err.to_string()))?;
        let content_hash = *blake3::hash(&content).as_bytes();
        if let Some(locator) = content_index.get(&content_hash) {
            entries.push(ManifestEntry {
                path: item.rel.clone(),
                blob_id: locator.blob_id,
                content_hash,
                size: item.size,
                mtime_ns: item.mtime_ns,
                created_time_ns: item.created_time_ns,
                win_attributes: item.win_attributes,
                codec: locator.codec,
                blob_offset: locator.blob_offset,
                blob_len: locator.blob_len,
                volume_index: locator.volume_index,
                parts: None,
                ext: None,
            });
            processed_files += 1;
            processed_bytes += item.size;
            progress(ProgressEvent {
                processed_files,
                total_files,
                processed_bytes,
                total_bytes,
                elapsed_ms: started.elapsed().as_millis(),
            });
            continue;
        }
        let codec = effective_codec_for_path(item, options.codec);
        let mut record_bytes = Vec::new();
        let write = write_blob_record(&mut record_bytes, &content, codec, options.zstd_level)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        let blob_len = record_bytes.len() as u64;
        let (volume_index, blob_offset) = output.write_record(&record_bytes, 0)?;
        entries.push(ManifestEntry {
            path: item.rel.clone(),
            blob_id: write.header.blob_id,
            content_hash,
            size: item.size,
            mtime_ns: item.mtime_ns,
            created_time_ns: item.created_time_ns,
            win_attributes: item.win_attributes,
            codec: write.header.codec,
            blob_offset,
            blob_len,
            volume_index,
            parts: None,
            ext: None,
        });
        content_index.insert(
            content_hash,
            BlobLocator {
                blob_id: write.header.blob_id,
                codec: write.header.codec,
                blob_offset,
                blob_len,
                volume_index,
            },
        );
        processed_files += 1;
        processed_bytes += item.size;
        progress(ProgressEvent {
            processed_files,
            total_files,
            processed_bytes,
            total_bytes,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let referenced_blob_bytes = sum_unique_blob_bytes(&entries);
    let blob_count = unique_blob_count(&entries) as u64;
    let manifest_prefix = ManifestPrefix {
        manifest_codec: ManifestCodec::JSON,
        manifest_type: ManifestType::FULL,
        manifest_version: 1,
    };
    let manifest_body = ManifestBody {
        snapshot_id: Ulid::new().to_string(),
        base_snapshot_id: None,
        created_at: now_unix_secs(),
        source_root: source_dir.to_string_lossy().into_owned(),
        snapshot_context: json!({
            "hostname": std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown-host".to_string()),
            "username": std::env::var("USERNAME").unwrap_or_else(|_| "unknown-user".to_string()),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "xunyu_version": env!("CARGO_PKG_VERSION"),
            "command_mode": "backup",
            "compression_profile": options.codec.as_u8(),
        }),
        file_count: entries.len() as u64,
        total_raw_bytes,
        entries,
        removed: vec![],
    };
    let mut manifest_record = Vec::new();
    write_manifest_record(&mut manifest_record, manifest_prefix, &manifest_body)?;
    let checkpoint_record_len =
        RECORD_PREFIX_SIZE as u64 + crate::xunbak::constants::CHECKPOINT_PAYLOAD_SIZE as u64;
    let manifest_trailing = checkpoint_record_len + FOOTER_SIZE as u64;
    let (manifest_volume_index, manifest_offset) =
        output.write_record(&manifest_record, manifest_trailing)?;
    let total_volumes = output.total_volumes();
    let manifest_payload = &manifest_record[RECORD_PREFIX_SIZE..];
    let manifest_hash = compute_manifest_hash(manifest_payload);
    let manifest_len = manifest_record.len() as u64;
    let checkpoint_offset = output.current_len;
    let total_container_bytes = output.current_len + checkpoint_record_len + FOOTER_SIZE as u64;
    let checkpoint_payload = CheckpointPayload {
        snapshot_id: Ulid::from_string(&manifest_body.snapshot_id)
            .expect("generated ULID must parse")
            .to_bytes(),
        manifest_offset,
        manifest_len,
        manifest_hash,
        container_end: total_container_bytes,
        blob_count,
        referenced_blob_bytes,
        total_container_bytes,
        prev_checkpoint_offset: 0,
        total_volumes,
    };
    let mut checkpoint_record = Vec::new();
    write_checkpoint_record(&mut checkpoint_record, &checkpoint_payload)?;
    debug_assert_eq!(manifest_volume_index, output.current_index);
    output.write_record(&checkpoint_record, FOOTER_SIZE as u64)?;
    output.write_footer(checkpoint_offset)?;

    Ok(BackupResult {
        container_path: output.last_volume_path().to_path_buf(),
        file_count: scanned.len(),
        blob_count: blob_count as usize,
    })
}

fn update_split_with_progress(
    container_path: &Path,
    source_dir: &Path,
    options: &BackupOptions,
    progress: &mut dyn FnMut(ProgressEvent),
    reader: &ContainerReader,
) -> Result<UpdateResult, WriterError> {
    let baseline = reader.load_manifest()?;
    let mut content_index = build_content_hash_index(&baseline);
    let mut scanned = Vec::new();
    collect_files(source_dir, source_dir, &mut scanned, Some(container_path))?;
    scanned.sort_by(|a, b| a.rel.cmp(&b.rel));
    let total_files = scanned.len();
    let total_bytes: u64 = scanned.iter().map(|item| item.size).sum();
    let started = Instant::now();

    let split = reader
        .header
        .header
        .split
        .ok_or(WriterError::InvalidSplitState)?;
    if let Some(requested) = options.split_size
        && requested != split.split_size
    {
        return Err(WriterError::SplitSizeMismatch {
            expected: split.split_size,
            actual: requested,
        });
    }
    let base_path = split_base_path(&reader.path);
    let last_volume_len_without_footer = reader.file_size - FOOTER_SIZE as u64;
    let mut output = VolumeOutput::from_existing_split(
        &base_path,
        split.split_size,
        split.set_id,
        &reader.volume_paths,
        last_volume_len_without_footer,
    )?;

    let mut entries = Vec::with_capacity(scanned.len());
    let mut added_blob_count = 0usize;
    let mut total_raw_bytes = 0u64;
    let mut processed_files = 0usize;
    let mut processed_bytes = 0u64;

    for item in &scanned {
        total_raw_bytes += item.size;
        if let Some(old) = baseline.entries.iter().find(|entry| entry.path == item.rel)
            && old.size == item.size
            && old.mtime_ns == item.mtime_ns
        {
            entries.push(old.clone());
            processed_files += 1;
            processed_bytes += item.size;
            progress(ProgressEvent {
                processed_files,
                total_files,
                processed_bytes,
                total_bytes,
                elapsed_ms: started.elapsed().as_millis(),
            });
            continue;
        }

        let content = fs::read(&item.path).map_err(|err| WriterError::Io(err.to_string()))?;
        let content_hash = *blake3::hash(&content).as_bytes();
        if let Some(locator) = content_index.get(&content_hash) {
            entries.push(ManifestEntry {
                path: item.rel.clone(),
                blob_id: locator.blob_id,
                content_hash,
                size: item.size,
                mtime_ns: item.mtime_ns,
                created_time_ns: item.created_time_ns,
                win_attributes: item.win_attributes,
                codec: locator.codec,
                blob_offset: locator.blob_offset,
                blob_len: locator.blob_len,
                volume_index: locator.volume_index,
                parts: None,
                ext: None,
            });
            processed_files += 1;
            processed_bytes += item.size;
            progress(ProgressEvent {
                processed_files,
                total_files,
                processed_bytes,
                total_bytes,
                elapsed_ms: started.elapsed().as_millis(),
            });
            continue;
        }

        let codec = effective_codec_for_path(item, options.codec);
        let mut record_bytes = Vec::new();
        let write = write_blob_record(&mut record_bytes, &content, codec, options.zstd_level)
            .map_err(|err| WriterError::Io(err.to_string()))?;
        let blob_len = record_bytes.len() as u64;
        let (volume_index, blob_offset) = output.write_record(&record_bytes, 0)?;
        entries.push(ManifestEntry {
            path: item.rel.clone(),
            blob_id: write.header.blob_id,
            content_hash,
            size: item.size,
            mtime_ns: item.mtime_ns,
            created_time_ns: item.created_time_ns,
            win_attributes: item.win_attributes,
            codec: write.header.codec,
            blob_offset,
            blob_len,
            volume_index,
            parts: None,
            ext: None,
        });
        content_index.insert(
            content_hash,
            BlobLocator {
                blob_id: write.header.blob_id,
                codec: write.header.codec,
                blob_offset,
                blob_len,
                volume_index,
            },
        );
        added_blob_count += 1;
        processed_files += 1;
        processed_bytes += item.size;
        progress(ProgressEvent {
            processed_files,
            total_files,
            processed_bytes,
            total_bytes,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let referenced_blob_bytes = sum_unique_blob_bytes(&entries);
    let entry_count = entries.len() as u64;
    let blob_count = unique_blob_count(&entries) as u64;
    let manifest_prefix = ManifestPrefix {
        manifest_codec: ManifestCodec::JSON,
        manifest_type: ManifestType::FULL,
        manifest_version: 1,
    };
    let manifest_body = ManifestBody {
        snapshot_id: Ulid::new().to_string(),
        base_snapshot_id: None,
        created_at: now_unix_secs(),
        source_root: source_dir.to_string_lossy().into_owned(),
        snapshot_context: json!({
            "hostname": std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown-host".to_string()),
            "username": std::env::var("USERNAME").unwrap_or_else(|_| "unknown-user".to_string()),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "xunyu_version": env!("CARGO_PKG_VERSION"),
            "command_mode": "update",
            "compression_profile": options.codec.as_u8(),
        }),
        file_count: entry_count,
        total_raw_bytes,
        entries,
        removed: vec![],
    };
    let mut manifest_record = Vec::new();
    write_manifest_record(&mut manifest_record, manifest_prefix, &manifest_body)?;
    let checkpoint_record_len =
        RECORD_PREFIX_SIZE as u64 + crate::xunbak::constants::CHECKPOINT_PAYLOAD_SIZE as u64;
    let manifest_trailing = checkpoint_record_len + FOOTER_SIZE as u64;
    let (_manifest_volume_index, manifest_offset) =
        output.write_record(&manifest_record, manifest_trailing)?;
    let total_volumes = output.total_volumes();
    let manifest_payload = &manifest_record[RECORD_PREFIX_SIZE..];
    let manifest_hash = compute_manifest_hash(manifest_payload);
    let manifest_len = manifest_record.len() as u64;
    let checkpoint_offset = output.current_len;
    let total_container_bytes = output.current_len + checkpoint_record_len + FOOTER_SIZE as u64;
    let checkpoint_payload = CheckpointPayload {
        snapshot_id: Ulid::from_string(&manifest_body.snapshot_id)
            .expect("generated ULID must parse")
            .to_bytes(),
        manifest_offset,
        manifest_len,
        manifest_hash,
        container_end: total_container_bytes,
        blob_count,
        referenced_blob_bytes,
        total_container_bytes,
        prev_checkpoint_offset: 0,
        total_volumes,
    };
    let mut checkpoint_record = Vec::new();
    write_checkpoint_record(&mut checkpoint_record, &checkpoint_payload)?;
    output.write_record(&checkpoint_record, FOOTER_SIZE as u64)?;
    output.write_footer(checkpoint_offset)?;

    Ok(UpdateResult {
        container_path: output.last_volume_path().to_path_buf(),
        added_blob_count,
        file_count: scanned.len(),
    })
}

fn cleanup_existing_split_outputs(base_path: &Path) -> Result<(), WriterError> {
    if let Some(parent) = base_path.parent() {
        let prefix = base_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(&format!("{prefix}.")) && name.len() == prefix.len() + 4 {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
    Ok(())
}
