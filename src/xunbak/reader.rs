use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use rayon::prelude::*;
use uuid::Uuid;

use crate::backup::common::cli::restore_path_not_found_message;
use crate::backup::common::hash::compute_file_content_hash;
use crate::commands::restore_core::emit_restore_dry_run;
use crate::xunbak::blob::{
    BlobRecordError, copy_encoded_blob_record_content_to_writer, decode_blob_record,
    read_blob_record_encoded,
};
use crate::xunbak::checkpoint::{CheckpointError, CheckpointPayload, read_checkpoint_record};
use crate::xunbak::constants::{
    BLOB_HEADER_SIZE, FLAG_SPLIT, FOOTER_SIZE, HEADER_SIZE, RECORD_PREFIX_SIZE, RecordType,
};
use crate::xunbak::footer::{Footer, FooterError};
use crate::xunbak::header::{DecodedHeader, Header, HeaderError};
use crate::xunbak::manifest::{
    ManifestBody, ManifestEntry, ManifestError, read_manifest_record, unix_ns_to_filetime,
};
use crate::xunbak::memory::reserve_buffer_capacity;
use crate::xunbak::record::{RecordPrefix, compute_record_crc};

#[derive(Debug)]
pub struct ContainerReader {
    pub path: PathBuf,
    pub file_size: u64,
    pub header: DecodedHeader,
    pub footer: Footer,
    pub checkpoint: CheckpointPayload,
    pub is_split: bool,
    pub volume_paths: Vec<PathBuf>,
    manifest_cache: OnceLock<ManifestBody>,
    volume_files: Vec<Arc<Mutex<Option<CachedVolumeFile>>>>,
}

#[derive(Debug)]
struct CachedVolumeFile {
    file: File,
    position: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RestoreWriteStats {
    open: Duration,
    copy: Duration,
    metadata: Duration,
    commit: Duration,
    cleanup: Duration,
    direct_files: usize,
    staged_files: usize,
}

#[derive(Debug)]
struct RestoreDebugStats {
    mode: &'static str,
    dry_run: bool,
    workers: usize,
    total_start: Instant,
    manifest_load: Duration,
    plan: Duration,
    precreate_dirs: Duration,
    write_open: Duration,
    write_copy: Duration,
    write_metadata: Duration,
    write_commit: Duration,
    write_cleanup: Duration,
    files: usize,
    parent_dirs: usize,
    direct_files: usize,
    staged_files: usize,
    skipped_unchanged: usize,
}

impl RestoreDebugStats {
    fn new(mode: &'static str, dry_run: bool, workers: usize, manifest_load: Duration) -> Self {
        Self {
            mode,
            dry_run,
            workers,
            total_start: Instant::now(),
            manifest_load,
            plan: Duration::ZERO,
            precreate_dirs: Duration::ZERO,
            write_open: Duration::ZERO,
            write_copy: Duration::ZERO,
            write_metadata: Duration::ZERO,
            write_commit: Duration::ZERO,
            write_cleanup: Duration::ZERO,
            files: 0,
            parent_dirs: 0,
            direct_files: 0,
            staged_files: 0,
            skipped_unchanged: 0,
        }
    }

    fn record_write(&mut self, stats: RestoreWriteStats) {
        self.write_open += stats.open;
        self.write_copy += stats.copy;
        self.write_metadata += stats.metadata;
        self.write_commit += stats.commit;
        self.write_cleanup += stats.cleanup;
        self.direct_files += stats.direct_files;
        self.staged_files += stats.staged_files;
    }

    fn emit(&self, restored_files: usize) {
        eprintln!(
            "perf: xunbak restore mode={} dry_run={} workers={} files={} restored={} parents={} direct={} staged={} skipped={} total_ms={} manifest_ms={} plan_ms={} precreate_dirs_ms={} open_ms={} copy_ms={} metadata_ms={} commit_ms={} cleanup_ms={}",
            self.mode,
            self.dry_run,
            self.workers,
            self.files,
            restored_files,
            self.parent_dirs,
            self.direct_files,
            self.staged_files,
            self.skipped_unchanged,
            self.total_start.elapsed().as_millis(),
            self.manifest_load.as_millis(),
            self.plan.as_millis(),
            self.precreate_dirs.as_millis(),
            self.write_open.as_millis(),
            self.write_copy.as_millis(),
            self.write_metadata.as_millis(),
            self.write_commit.as_millis(),
            self.write_cleanup.as_millis(),
        );
    }
}

impl RestoreWriteStats {
    fn merge(&mut self, other: RestoreWriteStats) {
        self.open += other.open;
        self.copy += other.copy;
        self.metadata += other.metadata;
        self.commit += other.commit;
        self.cleanup += other.cleanup;
        self.direct_files += other.direct_files;
        self.staged_files += other.staged_files;
    }
}

#[derive(Debug)]
struct RestoreJob<'a> {
    entry: &'a ManifestEntry,
    dest: PathBuf,
}

static XUNBAK_RESTORE_THREAD_POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();

#[derive(Debug)]
struct RestoreJobGroup<'a> {
    jobs: Vec<RestoreJob<'a>>,
    total_bytes: u64,
    first_index: usize,
}

#[derive(Debug)]
struct RestoreWorkerLane<'a> {
    jobs: Vec<RestoreJob<'a>>,
    total_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreResult {
    pub restored_files: usize,
    pub skipped_unchanged: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ReaderError {
    #[error("container too small: {actual} bytes")]
    ContainerTooSmall { actual: u64 },
    #[error(transparent)]
    Header(#[from] HeaderError),
    #[error(transparent)]
    Footer(#[from] FooterError),
    #[error(transparent)]
    Checkpoint(#[from] CheckpointError),
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error(transparent)]
    Blob(#[from] BlobRecordError),
    #[error("manifest hash mismatch")]
    ManifestHashMismatch,
    #[error("manifest length mismatch: checkpoint_len={checkpoint_len}, actual_len={actual_len}")]
    ManifestLengthMismatch {
        checkpoint_len: u64,
        actual_len: u64,
    },
    #[error("resource limit: {0}")]
    ResourceLimit(String),
    #[error("{0}")]
    PathNotFound(String),
    #[error("unrecoverable container")]
    UnrecoverableContainer,
    #[error("I/O error: {0}")]
    Io(String),
}

impl ContainerReader {
    pub fn open(path: &Path) -> Result<Self, ReaderError> {
        let primary_path = resolve_primary_path(path)?;
        let mut file = File::open(&primary_path).map_err(|err| ReaderError::Io(err.to_string()))?;
        let file_size = file
            .metadata()
            .map_err(|err| ReaderError::Io(err.to_string()))?
            .len();
        if file_size < (HEADER_SIZE + FOOTER_SIZE) as u64 && !is_split_member_path(&primary_path) {
            return Err(ReaderError::ContainerTooSmall { actual: file_size });
        }

        let mut header_bytes = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_bytes)
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let header = Header::from_bytes(&header_bytes)?;

        let (volume_paths, last_path, last_size) = if header.header.flags & FLAG_SPLIT != 0 {
            let paths = discover_split_volumes(&primary_path, &header)?;
            let last_path = paths
                .last()
                .cloned()
                .ok_or(ReaderError::UnrecoverableContainer)?;
            let last_size = std::fs::metadata(&last_path)
                .map_err(|err| ReaderError::Io(err.to_string()))?
                .len();
            (paths, last_path, last_size)
        } else {
            (vec![primary_path.clone()], primary_path.clone(), file_size)
        };

        let (footer, checkpoint, used_fallback) = {
            let mut last_file =
                File::open(&last_path).map_err(|err| ReaderError::Io(err.to_string()))?;
            match read_footer(&mut last_file, last_size) {
                Ok(footer) => {
                    last_file
                        .seek(SeekFrom::Start(footer.checkpoint_offset))
                        .map_err(|err| ReaderError::Io(err.to_string()))?;
                    let checkpoint = read_checkpoint_record(&mut last_file)?.payload;
                    (footer, checkpoint, false)
                }
                Err(_) => {
                    let (offset, checkpoint) = fallback_scan(&volume_paths)?;
                    (
                        Footer {
                            checkpoint_offset: offset,
                        },
                        checkpoint,
                        true,
                    )
                }
            }
        };

        let mut active_volume_paths = volume_paths;
        let active_file_size = if header.header.flags & FLAG_SPLIT != 0 {
            let expected = checkpoint.total_volumes as usize;
            let actual = active_volume_paths.len();
            if expected == 0 || expected > actual {
                return Err(ReaderError::UnrecoverableContainer);
            }
            if expected != actual {
                if !used_fallback {
                    return Err(ReaderError::UnrecoverableContainer);
                }
                active_volume_paths.truncate(expected);
            }
            std::fs::metadata(active_volume_paths.last().expect("checked non-empty"))
                .map_err(|err| ReaderError::Io(err.to_string()))?
                .len()
        } else {
            last_size
        };
        let volume_files = active_volume_paths
            .iter()
            .map(|_| Arc::new(Mutex::new(None)))
            .collect::<Vec<_>>();
        Ok(Self {
            path: primary_path,
            file_size: active_file_size,
            header,
            footer,
            checkpoint,
            is_split: header.header.flags & FLAG_SPLIT != 0,
            volume_paths: active_volume_paths,
            manifest_cache: OnceLock::new(),
            volume_files,
        })
    }

    pub fn load_manifest(&self) -> Result<ManifestBody, ReaderError> {
        if let Some(manifest) = self.manifest_cache.get() {
            return Ok(manifest.clone());
        }
        let manifest_volume = self.volume_paths.len().saturating_sub(1) as u16;
        let manifest =
            self.with_volume_file_at(manifest_volume, self.checkpoint.manifest_offset, |file| {
                let manifest = read_manifest_record(file)?;
                let actual_len = manifest.record_len + RECORD_PREFIX_SIZE as u64;
                if self.checkpoint.manifest_len != actual_len {
                    return Err(ReaderError::ManifestLengthMismatch {
                        checkpoint_len: self.checkpoint.manifest_len,
                        actual_len,
                    });
                }
                if manifest.payload_hash != self.checkpoint.manifest_hash {
                    return Err(ReaderError::ManifestHashMismatch);
                }
                Ok(manifest.body)
            })?;
        let _ = self.manifest_cache.set(manifest.clone());
        Ok(manifest)
    }

    pub fn read_and_verify_blob(&self, entry: &ManifestEntry) -> Result<Vec<u8>, ReaderError> {
        if let Some(parts) = &entry.parts {
            let mut out = Vec::new();
            reserve_buffer_capacity(&mut out, entry.size, "multipart blob content")
                .map_err(ReaderError::ResourceLimit)?;
            for part in parts {
                let encoded =
                    self.with_volume_file_at(part.volume_index, part.blob_offset, |file| {
                        read_blob_record_encoded(file).map_err(ReaderError::from)
                    })?;
                let blob = decode_blob_record(encoded).map_err(ReaderError::from)?;
                out.extend_from_slice(&blob.content);
            }
            if *blake3::hash(&out).as_bytes() != entry.content_hash {
                return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
            }
            return Ok(out);
        }

        let encoded = self.with_volume_file_at(entry.volume_index, entry.blob_offset, |file| {
            read_blob_record_encoded(file).map_err(ReaderError::from)
        })?;
        let blob = decode_blob_record(encoded).map_err(ReaderError::from)?;
        if *blake3::hash(&blob.content).as_bytes() != entry.content_hash {
            return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
        }
        Ok(blob.content)
    }

    pub fn copy_and_verify_blob<W: Write + ?Sized>(
        &self,
        entry: &ManifestEntry,
        writer: &mut W,
    ) -> Result<(), ReaderError> {
        let mut hashing_writer = ManifestHashingWriter::new(writer);
        if let Some(parts) = &entry.parts {
            for part in parts {
                let encoded =
                    self.with_volume_file_at(part.volume_index, part.blob_offset, |file| {
                        read_blob_record_encoded(file).map_err(ReaderError::from)
                    })?;
                if encoded.header.blob_id != part.blob_id {
                    return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
                }
                let result =
                    copy_encoded_blob_record_content_to_writer(encoded, &mut hashing_writer)
                        .map_err(ReaderError::from)?;
                if result.header.blob_id != part.blob_id {
                    return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
                }
            }
            if hashing_writer.finalize() != entry.content_hash {
                return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
            }
            return Ok(());
        }

        let encoded = self.with_volume_file_at(entry.volume_index, entry.blob_offset, |file| {
            read_blob_record_encoded(file).map_err(ReaderError::from)
        })?;
        if encoded.header.blob_id != entry.blob_id {
            return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
        }
        let result = copy_encoded_blob_record_content_to_writer(encoded, &mut hashing_writer)
            .map_err(ReaderError::from)?;
        if result.header.blob_id != entry.blob_id || hashing_writer.finalize() != entry.content_hash
        {
            return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
        }
        Ok(())
    }

    pub fn restore_all(&self, target_dir: &Path) -> Result<RestoreResult, ReaderError> {
        let t_manifest = Instant::now();
        let manifest = self.load_manifest()?;
        let workers = xunbak_restore_parallel_workers(
            manifest.entries.len(),
            manifest.entries.iter().map(|entry| entry.size).sum(),
        );
        let mut debug = xunbak_restore_timing_enabled()
            .then(|| RestoreDebugStats::new("all", false, workers, t_manifest.elapsed()));
        std::fs::create_dir_all(target_dir).map_err(|err| ReaderError::Io(err.to_string()))?;
        let result = self.restore_matching(
            &manifest,
            target_dir,
            false,
            workers,
            |entry| {
                let _ = entry;
                true
            },
            debug.as_mut(),
        )?;
        if let Some(debug) = debug.as_ref() {
            debug.emit(result.restored_files);
        }
        Ok(result)
    }

    pub fn dry_run_restore_all(&self, target_dir: &Path) -> Result<RestoreResult, ReaderError> {
        let t_manifest = Instant::now();
        let manifest = self.load_manifest()?;
        let mut debug = xunbak_restore_timing_enabled()
            .then(|| RestoreDebugStats::new("all", true, 1, t_manifest.elapsed()));
        let result = self.restore_matching(
            &manifest,
            target_dir,
            true,
            1,
            |entry| {
                let _ = entry;
                true
            },
            debug.as_mut(),
        )?;
        if let Some(debug) = debug.as_ref() {
            debug.emit(result.restored_files);
        }
        Ok(result)
    }

    pub fn restore_file(
        &self,
        path: &str,
        target_dir: &Path,
    ) -> Result<RestoreResult, ReaderError> {
        let t_manifest = Instant::now();
        let manifest = self.load_manifest()?;
        let mut debug = xunbak_restore_timing_enabled()
            .then(|| RestoreDebugStats::new("file", false, 1, t_manifest.elapsed()));
        let result = self.restore_matching(
            &manifest,
            target_dir,
            false,
            1,
            |entry| entry.path.eq_ignore_ascii_case(path),
            debug.as_mut(),
        )?;
        if let Some(debug) = debug.as_ref() {
            debug.emit(result.restored_files);
        }
        if result.restored_files == 0 {
            return Err(ReaderError::PathNotFound(restore_path_not_found_message(
                path,
            )));
        }
        Ok(result)
    }

    pub fn dry_run_restore_file(
        &self,
        path: &str,
        target_dir: &Path,
    ) -> Result<RestoreResult, ReaderError> {
        let t_manifest = Instant::now();
        let manifest = self.load_manifest()?;
        let mut debug = xunbak_restore_timing_enabled()
            .then(|| RestoreDebugStats::new("file", true, 1, t_manifest.elapsed()));
        let result = self.restore_matching(
            &manifest,
            target_dir,
            true,
            1,
            |entry| entry.path.eq_ignore_ascii_case(path),
            debug.as_mut(),
        )?;
        if let Some(debug) = debug.as_ref() {
            debug.emit(result.restored_files);
        }
        if result.restored_files == 0 {
            return Err(ReaderError::PathNotFound(restore_path_not_found_message(
                path,
            )));
        }
        Ok(result)
    }

    pub fn restore_glob(
        &self,
        pattern: &str,
        target_dir: &Path,
    ) -> Result<RestoreResult, ReaderError> {
        let t_manifest = Instant::now();
        let manifest = self.load_manifest()?;
        let mut debug = xunbak_restore_timing_enabled()
            .then(|| RestoreDebugStats::new("glob", false, 1, t_manifest.elapsed()));
        let result = self.restore_matching(
            &manifest,
            target_dir,
            false,
            1,
            |entry| glob_match(pattern, &entry.path),
            debug.as_mut(),
        )?;
        if let Some(debug) = debug.as_ref() {
            debug.emit(result.restored_files);
        }
        Ok(result)
    }

    pub fn dry_run_restore_glob(
        &self,
        pattern: &str,
        target_dir: &Path,
    ) -> Result<RestoreResult, ReaderError> {
        let t_manifest = Instant::now();
        let manifest = self.load_manifest()?;
        let mut debug = xunbak_restore_timing_enabled()
            .then(|| RestoreDebugStats::new("glob", true, 1, t_manifest.elapsed()));
        let result = self.restore_matching(
            &manifest,
            target_dir,
            true,
            1,
            |entry| glob_match(pattern, &entry.path),
            debug.as_mut(),
        )?;
        if let Some(debug) = debug.as_ref() {
            debug.emit(result.restored_files);
        }
        Ok(result)
    }
}

fn read_footer(file: &mut File, file_size: u64) -> Result<Footer, ReaderError> {
    file.seek(SeekFrom::Start(file_size - FOOTER_SIZE as u64))
        .map_err(|err| ReaderError::Io(err.to_string()))?;
    let mut footer_bytes = [0u8; FOOTER_SIZE];
    file.read_exact(&mut footer_bytes)
        .map_err(|err| ReaderError::Io(err.to_string()))?;
    Footer::from_bytes(&footer_bytes, file_size).map_err(ReaderError::from)
}

fn fallback_scan(volume_paths: &[PathBuf]) -> Result<(u64, CheckpointPayload), ReaderError> {
    let mut last: Option<(PathBuf, u64)> = None;
    for volume_path in volume_paths {
        let mut file = File::open(volume_path).map_err(|err| ReaderError::Io(err.to_string()))?;
        file.seek(SeekFrom::Start(HEADER_SIZE as u64))
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let mut offset = 0usize;
        while offset + RECORD_PREFIX_SIZE <= bytes.len() {
            let prefix = match RecordPrefix::from_bytes(&bytes[offset..offset + RECORD_PREFIX_SIZE])
            {
                Ok(prefix) => prefix,
                Err(_) => break,
            };
            let payload_start = offset + RECORD_PREFIX_SIZE;
            let payload_end = payload_start.saturating_add(prefix.record_len as usize);
            if payload_end > bytes.len() {
                break;
            }
            let payload = &bytes[payload_start..payload_end];
            let payload_for_crc: &[u8] =
                if prefix.record_type == RecordType::BLOB && payload.len() >= BLOB_HEADER_SIZE {
                    &payload[..BLOB_HEADER_SIZE]
                } else {
                    payload
                };
            let crc = compute_record_crc(
                prefix.record_type,
                prefix.record_len.to_le_bytes(),
                payload_for_crc,
            );
            if crc != prefix.record_crc {
                break;
            }
            if prefix.record_type == RecordType::CHECKPOINT {
                last = Some((volume_path.clone(), HEADER_SIZE as u64 + offset as u64));
            }
            offset = payload_end;
        }
    }
    let (path, checkpoint_offset) = last.ok_or(ReaderError::UnrecoverableContainer)?;
    let mut file = File::open(path).map_err(|err| ReaderError::Io(err.to_string()))?;
    file.seek(SeekFrom::Start(checkpoint_offset))
        .map_err(|err| ReaderError::Io(err.to_string()))?;
    let checkpoint = read_checkpoint_record(&mut file)?.payload;
    Ok((checkpoint_offset, checkpoint))
}

impl ContainerReader {
    fn with_volume_file_at<R, F>(
        &self,
        volume_index: u16,
        offset: u64,
        action: F,
    ) -> Result<R, ReaderError>
    where
        F: FnOnce(&mut File) -> Result<R, ReaderError>,
    {
        let file = self
            .volume_files
            .get(volume_index as usize)
            .ok_or(ReaderError::UnrecoverableContainer)?;
        let mut file = file
            .lock()
            .map_err(|_| ReaderError::Io("volume file cache poisoned".to_string()))?;
        if file.is_none() {
            let path = self
                .volume_paths
                .get(volume_index as usize)
                .ok_or(ReaderError::UnrecoverableContainer)?;
            *file = Some(CachedVolumeFile {
                file: File::open(path).map_err(|err| ReaderError::Io(err.to_string()))?,
                position: 0,
            });
        }
        let file = file
            .as_mut()
            .ok_or_else(|| ReaderError::Io("missing cached volume file".to_string()))?;
        if file.position != offset {
            file.file
                .seek(SeekFrom::Start(offset))
                .map_err(|err| ReaderError::Io(err.to_string()))?;
            file.position = offset;
        }
        let result = action(&mut file.file)?;
        file.position = file
            .file
            .stream_position()
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        Ok(result)
    }
}

#[cfg(test)]
pub(crate) fn opened_volume_file_count_for_tests(reader: &ContainerReader) -> usize {
    reader
        .volume_files
        .iter()
        .filter(|file| file.lock().map(|entry| entry.is_some()).unwrap_or(false))
        .count()
}

fn resolve_primary_path(input: &Path) -> Result<PathBuf, ReaderError> {
    if input.exists() {
        return Ok(input.to_path_buf());
    }
    let split_first = PathBuf::from(format!("{}.001", input.display()));
    if split_first.exists() {
        return Ok(split_first);
    }
    Err(ReaderError::Io(format!(
        "container not found: {}",
        input.display()
    )))
}

fn discover_split_volumes(
    first_volume: &Path,
    first_header: &DecodedHeader,
) -> Result<Vec<PathBuf>, ReaderError> {
    let base = split_base_path(first_volume);
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let prefix = base
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let expected_set_id = first_header
        .header
        .split
        .as_ref()
        .map(|split| split.set_id)
        .ok_or(ReaderError::UnrecoverableContainer)?;
    let mut volumes = Vec::new();
    for entry in std::fs::read_dir(parent).map_err(|err| ReaderError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| ReaderError::Io(err.to_string()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{prefix}.")) && name.len() == prefix.len() + 4 {
            volumes.push(entry.path());
        }
    }
    volumes.sort();
    if volumes.is_empty() {
        return Err(ReaderError::UnrecoverableContainer);
    }
    for (index, path) in volumes.iter().enumerate() {
        let mut file = File::open(path).map_err(|err| ReaderError::Io(err.to_string()))?;
        let mut header_bytes = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_bytes)
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let header = Header::from_bytes(&header_bytes)?;
        let split = header
            .header
            .split
            .ok_or(ReaderError::UnrecoverableContainer)?;
        if split.volume_index as usize != index || split.set_id != expected_set_id {
            return Err(ReaderError::UnrecoverableContainer);
        }
    }
    Ok(volumes)
}

fn split_base_path(path: &Path) -> PathBuf {
    if is_split_member_path(path) {
        let raw = path.to_string_lossy();
        PathBuf::from(raw[..raw.len() - 4].to_string())
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::{
        RestoreJob, build_restore_worker_lanes, decide_xunbak_restore_workers,
        opened_volume_file_count_for_tests, sorted_restore_entries,
    };
    use crate::xunbak::constants::Codec;
    use crate::xunbak::manifest::{ManifestBody, ManifestEntry};
    use crate::xunbak::writer::{BackupOptions, ContainerWriter};

    fn entry(path: &str, volume_index: u16, blob_offset: u64) -> ManifestEntry {
        ManifestEntry {
            path: path.to_string(),
            blob_id: [0; 32],
            content_hash: [0; 32],
            size: 1,
            mtime_ns: 0,
            created_time_ns: 0,
            win_attributes: 0,
            codec: Codec::NONE,
            blob_offset,
            blob_len: 64,
            volume_index,
            parts: None,
            ext: None,
        }
    }

    #[test]
    fn restore_plan_sorts_by_volume_then_blob_offset() {
        let manifest = ManifestBody {
            snapshot_id: "01JTESTSNAPSHOTID0000000000".to_string(),
            base_snapshot_id: None,
            created_at: 0,
            source_root: ".".to_string(),
            snapshot_context: serde_json::json!({}),
            file_count: 4,
            total_raw_bytes: 4,
            entries: vec![
                entry("d.txt", 1, 120),
                entry("b.txt", 0, 90),
                entry("a.txt", 0, 40),
                entry("c.txt", 1, 30),
            ],
            removed: vec![],
        };

        let planned = sorted_restore_entries(&manifest, |_| true);
        let ordered: Vec<(u16, u64, &str)> = planned
            .into_iter()
            .map(|entry| (entry.volume_index, entry.blob_offset, entry.path.as_str()))
            .collect();

        assert_eq!(
            ordered,
            vec![
                (0, 40, "a.txt"),
                (0, 90, "b.txt"),
                (1, 30, "c.txt"),
                (1, 120, "d.txt"),
            ]
        );
    }

    #[test]
    fn split_reader_opens_volume_files_lazily() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "a".repeat(1200)).unwrap();
        fs::write(source.join("b.txt"), "b".repeat(1200)).unwrap();
        let base = dir.path().join("backup.xunbak");

        ContainerWriter::backup(
            &base,
            &source,
            &BackupOptions {
                codec: Codec::NONE,
                auto_compression: false,
                zstd_level: 1,
                split_size: Some(1900),
            },
        )
        .unwrap();

        let reader = super::ContainerReader::open(&base).unwrap();
        assert_eq!(opened_volume_file_count_for_tests(&reader), 0);

        let manifest = reader.load_manifest().unwrap();
        assert_eq!(opened_volume_file_count_for_tests(&reader), 1);

        let _ = reader
            .read_and_verify_blob(
                manifest
                    .entries
                    .iter()
                    .find(|entry| entry.path == "a.txt")
                    .unwrap(),
            )
            .unwrap();
        assert!(opened_volume_file_count_for_tests(&reader) >= 1);
        assert!(opened_volume_file_count_for_tests(&reader) <= reader.volume_paths.len());
    }

    #[test]
    fn read_and_verify_blob_rejects_oversized_multipart_capacity() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "plain").unwrap();
        let base = dir.path().join("backup.xunbak");

        ContainerWriter::backup(
            &base,
            &source,
            &BackupOptions {
                codec: Codec::NONE,
                auto_compression: false,
                zstd_level: 1,
                split_size: None,
            },
        )
        .unwrap();

        let reader = super::ContainerReader::open(&base).unwrap();
        let oversized = ManifestEntry {
            path: "oversized.bin".to_string(),
            blob_id: [0; 32],
            content_hash: [0; 32],
            size: u64::MAX,
            mtime_ns: 0,
            created_time_ns: 0,
            win_attributes: 0,
            codec: Codec::NONE,
            blob_offset: 0,
            blob_len: 0,
            volume_index: 0,
            parts: Some(Vec::new()),
            ext: None,
        };

        assert!(matches!(
            reader.read_and_verify_blob(&oversized),
            Err(super::ReaderError::ResourceLimit(_))
        ));
    }

    #[test]
    fn restore_worker_strategy_keeps_small_jobs_single_threaded() {
        assert_eq!(decide_xunbak_restore_workers(4, 32, 32 * 4096), 1);
    }

    #[test]
    fn restore_worker_strategy_uses_more_workers_for_many_small_files() {
        assert_eq!(decide_xunbak_restore_workers(8, 1000, 1000 * 4096), 4);
        assert_eq!(decide_xunbak_restore_workers(3, 300, 300 * 8192), 2);
    }

    #[test]
    fn restore_worker_strategy_keeps_large_payloads_conservative() {
        assert_eq!(decide_xunbak_restore_workers(4, 16, 256 * 1024 * 1024), 2);
        assert_eq!(decide_xunbak_restore_workers(4, 4, 256 * 1024 * 1024), 1);
    }

    #[test]
    fn restore_worker_lanes_keep_same_parent_jobs_together() {
        let manifest = ManifestBody {
            snapshot_id: "01JTESTSNAPSHOTID0000000000".to_string(),
            base_snapshot_id: None,
            created_at: 0,
            source_root: ".".to_string(),
            snapshot_context: serde_json::json!({}),
            file_count: 4,
            total_raw_bytes: 4,
            entries: vec![
                entry("a/x.txt", 0, 10),
                entry("b/y.txt", 0, 20),
                entry("a/z.txt", 0, 30),
                entry("c/w.txt", 0, 40),
            ],
            removed: vec![],
        };
        let jobs = sorted_restore_entries(&manifest, |_| true)
            .into_iter()
            .map(|entry| RestoreJob {
                dest: PathBuf::from(entry.path.replace('/', "\\")),
                entry,
            })
            .collect::<Vec<_>>();
        let lanes = build_restore_worker_lanes(jobs, 2);
        let lanes_with_a = lanes
            .iter()
            .filter(|lane| {
                lane.jobs.iter().any(|job| {
                    job.dest
                        .parent()
                        .and_then(|path| path.to_str())
                        .is_some_and(|value| value == "a")
                })
            })
            .count();
        assert_eq!(lanes_with_a, 1);
    }
}

fn is_split_member_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    name.len() >= 4
        && name.as_bytes()[name.len() - 4] == b'.'
        && name[name.len() - 3..].chars().all(|ch| ch.is_ascii_digit())
}

impl ContainerReader {
    fn restore_matching<F>(
        &self,
        manifest: &ManifestBody,
        target_dir: &Path,
        dry_run: bool,
        parallel_workers: usize,
        mut predicate: F,
        mut debug: Option<&mut RestoreDebugStats>,
    ) -> Result<RestoreResult, ReaderError>
    where
        F: FnMut(&ManifestEntry) -> bool,
    {
        let mut restored = 0usize;
        let mut skipped_unchanged = 0usize;
        let t_plan = Instant::now();
        let mut entries = sorted_restore_entries(manifest, |entry| predicate(entry));
        if dry_run {
            entries.sort_by(|left, right| left.path.cmp(&right.path));
        }
        let allow_incremental_skip = target_dir_has_entries(target_dir);
        let jobs = entries
            .drain(..)
            .filter_map(|entry| {
                let dest = target_dir.join(entry.path.replace('/', "\\"));
                if allow_incremental_skip
                    && restore_destination_matches_entry(entry, &dest).unwrap_or(false)
                {
                    if let Some(debug) = debug.as_deref_mut() {
                        debug.skipped_unchanged += 1;
                    }
                    skipped_unchanged += 1;
                    return None;
                }
                Some(RestoreJob { dest, entry })
            })
            .collect::<Vec<_>>();
        if let Some(debug) = debug.as_deref_mut() {
            debug.plan += t_plan.elapsed();
            debug.files = jobs.len();
        }
        if !dry_run {
            let t_dirs = Instant::now();
            let mut parent_dirs = HashSet::new();
            for job in &jobs {
                if let Some(parent) = job.dest.parent() {
                    parent_dirs.insert(parent.to_path_buf());
                }
            }
            for dir in &parent_dirs {
                std::fs::create_dir_all(dir).map_err(|err| ReaderError::Io(err.to_string()))?;
            }
            if let Some(debug) = debug.as_deref_mut() {
                debug.precreate_dirs += t_dirs.elapsed();
                debug.parent_dirs = parent_dirs.len();
            }
        }
        if !dry_run && parallel_workers > 1 && jobs.len() >= parallel_workers * 2 {
            let lanes = build_restore_worker_lanes(jobs, parallel_workers);
            let results = xunbak_restore_thread_pool()
                .expect("parallel workers checked")
                .install(|| {
                    lanes
                        .par_iter()
                        .map(|lane| -> Result<(usize, RestoreWriteStats), ReaderError> {
                            let mut restored = 0usize;
                            let mut stats = RestoreWriteStats::default();
                            for job in &lane.jobs {
                                let write_stats =
                                    restore_entry_to_destination(self, job.entry, &job.dest)?;
                                stats.merge(write_stats);
                                restored += 1;
                            }
                            Ok((restored, stats))
                        })
                        .collect::<Vec<_>>()
                });
            for result in results {
                let (chunk_restored, chunk_stats) = result?;
                restored += chunk_restored;
                if let Some(debug) = debug.as_deref_mut() {
                    debug.record_write(chunk_stats);
                }
            }
            return Ok(RestoreResult {
                restored_files: restored,
                skipped_unchanged,
            });
        }
        for job in jobs {
            if dry_run {
                emit_restore_dry_run(job.dest.strip_prefix(target_dir).unwrap_or(&job.dest));
                restored += 1;
                continue;
            }
            let stats = restore_entry_to_destination(self, job.entry, &job.dest)?;
            if let Some(debug) = debug.as_deref_mut() {
                debug.record_write(stats);
            }
            restored += 1;
        }
        Ok(RestoreResult {
            restored_files: restored,
            skipped_unchanged,
        })
    }
}

struct ManifestHashingWriter<'a, W: ?Sized> {
    inner: &'a mut W,
    hasher: blake3::Hasher,
}

impl<'a, W: Write + ?Sized> ManifestHashingWriter<'a, W> {
    fn new(inner: &'a mut W) -> Self {
        Self {
            inner,
            hasher: blake3::Hasher::new(),
        }
    }

    fn finalize(self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }
}

impl<W: Write + ?Sized> Write for ManifestHashingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write_all(buf)?;
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn sorted_restore_entries<'a, F>(
    manifest: &'a ManifestBody,
    mut predicate: F,
) -> Vec<&'a ManifestEntry>
where
    F: FnMut(&ManifestEntry) -> bool,
{
    let mut entries: Vec<&ManifestEntry> = manifest
        .entries
        .iter()
        .filter(|entry| predicate(entry))
        .collect();
    entries.sort_by(|left, right| {
        left.volume_index
            .cmp(&right.volume_index)
            .then(left.blob_offset.cmp(&right.blob_offset))
            .then(left.path.cmp(&right.path))
    });
    entries
}

fn glob_match(pattern: &str, path: &str) -> bool {
    glob_match_parts(pattern.as_bytes(), path.as_bytes())
}

fn glob_match_parts(pat: &[u8], s: &[u8]) -> bool {
    if pat.starts_with(b"**") {
        let rest_pat = if pat.len() > 2 && pat[2] == b'/' {
            &pat[3..]
        } else {
            &pat[2..]
        };
        if glob_match_parts(rest_pat, s) {
            return true;
        }
        let mut i = 0;
        while i < s.len() {
            if s[i] == b'/' && glob_match_parts(pat, &s[i + 1..]) {
                return true;
            }
            i += 1;
        }
        return false;
    }

    match (pat.first(), s.first()) {
        (None, None) => true,
        (None, _) | (Some(_), None) => false,
        (Some(b'*'), _) => {
            if s[0] == b'/' {
                return false;
            }
            glob_match_parts(&pat[1..], s) || glob_match_parts(pat, &s[1..])
        }
        (Some(b'?'), Some(ch)) => *ch != b'/' && glob_match_parts(&pat[1..], &s[1..]),
        (Some(pc), Some(sc)) if pc.eq_ignore_ascii_case(sc) => glob_match_parts(&pat[1..], &s[1..]),
        _ => false,
    }
}

fn xunbak_restore_timing_enabled() -> bool {
    std::env::var_os("XUN_XUNBAK_RESTORE_TIMING").is_some()
}

fn xunbak_restore_parallelism_cap() -> usize {
    static WORKERS: OnceLock<usize> = OnceLock::new();
    *WORKERS.get_or_init(|| {
        std::env::var("XUN_XUNBAK_RESTORE_WORKERS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|workers| *workers > 0)
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|parallelism| parallelism.get().min(4))
                    .unwrap_or(1)
            })
    })
}

fn decide_xunbak_restore_workers(cap: usize, job_count: usize, total_bytes: u64) -> usize {
    if cap <= 1 {
        return 1;
    }
    if total_bytes >= 128 * 1024 * 1024 && job_count >= 8 {
        return cap.min(2);
    }
    if job_count < 64 {
        return 1;
    }
    let avg_bytes = total_bytes / job_count.max(1) as u64;
    if avg_bytes <= 64 * 1024 {
        if job_count >= 768 {
            return cap.min(4);
        }
        if job_count >= 192 {
            return cap.min(2);
        }
        return 1;
    }
    1
}

fn xunbak_restore_parallel_workers(job_count: usize, total_bytes: u64) -> usize {
    decide_xunbak_restore_workers(xunbak_restore_parallelism_cap(), job_count, total_bytes)
}

fn xunbak_restore_thread_pool() -> Option<&'static rayon::ThreadPool> {
    let workers = xunbak_restore_parallelism_cap();
    if workers <= 1 {
        return None;
    }
    Some(XUNBAK_RESTORE_THREAD_POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .thread_name(|index| format!("xunbak-restore-{index}"))
            .build()
            .expect("xunbak restore thread pool")
    }))
}

fn target_dir_has_entries(target_dir: &Path) -> bool {
    fs::read_dir(target_dir)
        .ok()
        .and_then(|mut entries| entries.next())
        .is_some()
}

fn build_restore_worker_lanes<'a>(
    jobs: Vec<RestoreJob<'a>>,
    requested_workers: usize,
) -> Vec<RestoreWorkerLane<'a>> {
    use std::collections::HashMap;

    let mut groups = Vec::<RestoreJobGroup<'a>>::new();
    let mut group_index = HashMap::<PathBuf, usize>::new();
    for (index, job) in jobs.into_iter().enumerate() {
        let parent = job.dest.parent().map(Path::to_path_buf).unwrap_or_default();
        let entry = group_index.entry(parent).or_insert_with(|| {
            let next = groups.len();
            groups.push(RestoreJobGroup {
                jobs: Vec::new(),
                total_bytes: 0,
                first_index: index,
            });
            next
        });
        let group = &mut groups[*entry];
        group.total_bytes += job.entry.size;
        group.jobs.push(job);
    }

    groups.sort_by(|left, right| {
        right
            .total_bytes
            .cmp(&left.total_bytes)
            .then(left.first_index.cmp(&right.first_index))
    });

    let lane_count = requested_workers.min(groups.len().max(1));
    let mut lanes = (0..lane_count)
        .map(|_| RestoreWorkerLane {
            jobs: Vec::new(),
            total_bytes: 0,
        })
        .collect::<Vec<_>>();
    for group in groups {
        let lane = lanes
            .iter_mut()
            .min_by_key(|lane| lane.total_bytes)
            .expect("lane_count >= 1");
        lane.total_bytes += group.total_bytes;
        lane.jobs.extend(group.jobs);
    }
    lanes.retain(|lane| !lane.jobs.is_empty());
    lanes
}

fn restore_destination_matches_entry(
    entry: &ManifestEntry,
    dest: &Path,
) -> Result<bool, ReaderError> {
    if !dest.exists() {
        return Ok(false);
    }
    let meta = fs::metadata(dest).map_err(|err| ReaderError::Io(err.to_string()))?;
    if !meta.is_file() || meta.len() != entry.size {
        return Ok(false);
    }
    Ok(
        compute_file_content_hash(dest).map_err(|err| ReaderError::Io(err.to_string()))?
            == entry.content_hash,
    )
}

fn restore_temp_path(dest: &Path) -> PathBuf {
    let base_name = dest
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("xunbak-restore");
    dest.with_file_name(format!(".{base_name}.{}.xunbak-tmp", Uuid::new_v4()))
}

fn restore_entry_to_destination(
    reader: &ContainerReader,
    entry: &ManifestEntry,
    dest: &Path,
) -> Result<RestoreWriteStats, ReaderError> {
    restore_entry_direct_or_stage(reader, entry, dest)
}

fn restore_entry_direct_or_stage(
    reader: &ContainerReader,
    entry: &ManifestEntry,
    dest: &Path,
) -> Result<RestoreWriteStats, ReaderError> {
    let mut stats = RestoreWriteStats::default();
    let mut cleanup_dest = false;
    let result = (|| {
        let t_open = Instant::now();
        let mut file = match create_restore_output_file(dest) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                stats.open += t_open.elapsed();
                return restore_entry_via_staged_commit(reader, entry, dest);
            }
            Err(err) => return Err(ReaderError::Io(err.to_string())),
        };
        stats.open += t_open.elapsed();
        stats.direct_files += 1;
        cleanup_dest = true;
        let t_copy = Instant::now();
        reader.copy_and_verify_blob(entry, &mut file)?;
        stats.copy += t_copy.elapsed();
        file.flush()
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let t_metadata = Instant::now();
        apply_windows_metadata_to_file(&file, dest, entry)?;
        stats.metadata += t_metadata.elapsed();
        drop(file);
        cleanup_dest = false;
        Ok(stats)
    })();
    if let Err(err) = result {
        if cleanup_dest {
            let t_cleanup = Instant::now();
            let _ = std::fs::remove_file(dest);
            stats.cleanup += t_cleanup.elapsed();
        }
        return Err(err);
    }
    result
}

fn restore_entry_via_staged_commit(
    reader: &ContainerReader,
    entry: &ManifestEntry,
    dest: &Path,
) -> Result<RestoreWriteStats, ReaderError> {
    let mut stats = RestoreWriteStats {
        staged_files: 1,
        ..RestoreWriteStats::default()
    };
    let temp = restore_temp_path(dest);
    let mut cleanup_temp = true;
    let result = (|| {
        let t_open = Instant::now();
        let mut file =
            create_restore_output_file(&temp).map_err(|err| ReaderError::Io(err.to_string()))?;
        stats.open += t_open.elapsed();
        let t_copy = Instant::now();
        reader.copy_and_verify_blob(entry, &mut file)?;
        stats.copy += t_copy.elapsed();
        file.flush()
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let t_metadata = Instant::now();
        apply_windows_metadata_to_file(&file, &temp, entry)?;
        stats.metadata += t_metadata.elapsed();
        drop(file);
        cleanup_temp = false;
        let t_commit = Instant::now();
        commit_restored_file(&temp, dest)?;
        stats.commit += t_commit.elapsed();
        Ok::<RestoreWriteStats, ReaderError>(stats)
    })();
    if let Err(err) = result {
        if cleanup_temp {
            let t_cleanup = Instant::now();
            let _ = std::fs::remove_file(&temp);
            stats.cleanup += t_cleanup.elapsed();
        }
        return Err(err);
    }
    result
}

#[cfg(windows)]
fn create_restore_output_file(path: &Path) -> std::io::Result<File> {
    use std::os::windows::io::FromRawHandle;

    use windows_sys::Win32::Foundation::{GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_SEQUENTIAL_SCAN,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_WRITE_ATTRIBUTES,
    };

    let wide = encode_win32_path(path);
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_WRITE | FILE_WRITE_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_SEQUENTIAL_SCAN,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_handle(handle as _) })
}

#[cfg(not(windows))]
fn create_restore_output_file(path: &Path) -> std::io::Result<File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

#[cfg(windows)]
fn commit_restored_file(temp: &Path, dest: &Path) -> Result<(), ReaderError> {
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let temp_w = encode_win32_path(temp);
    let dest_w = encode_win32_path(dest);
    let ok = unsafe {
        MoveFileExW(
            temp_w.as_ptr(),
            dest_w.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        return Err(ReaderError::Io(std::io::Error::last_os_error().to_string()));
    }
    Ok(())
}

#[cfg(not(windows))]
fn commit_restored_file(temp: &Path, dest: &Path) -> Result<(), ReaderError> {
    if dest.exists() {
        std::fs::remove_file(dest).map_err(|err| ReaderError::Io(err.to_string()))?;
    }
    std::fs::rename(temp, dest).map_err(|err| {
        ReaderError::Io(format!(
            "finalize restored file failed {} -> {}: {err}",
            temp.display(),
            dest.display()
        ))
    })
}

#[cfg(windows)]
fn apply_windows_metadata_to_file(
    file: &File,
    path: &Path,
    entry: &ManifestEntry,
) -> Result<(), ReaderError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_BASIC_INFO, FileBasicInfo, SetFileInformationByHandle,
    };

    let created_filetime = unix_ns_to_filetime(entry.created_time_ns as i128) as i64;
    let modified_filetime = unix_ns_to_filetime(entry.mtime_ns as i128) as i64;
    let info = FILE_BASIC_INFO {
        CreationTime: created_filetime,
        LastAccessTime: modified_filetime,
        LastWriteTime: modified_filetime,
        ChangeTime: modified_filetime,
        FileAttributes: entry.win_attributes,
    };
    let ok = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle() as HANDLE,
            FileBasicInfo,
            &info as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    };
    if ok == 0 {
        return Err(ReaderError::Io(format!(
            "SetFileInformationByHandle failed for {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(windows))]
fn apply_windows_metadata_to_file(
    _file: &File,
    _path: &Path,
    _entry: &ManifestEntry,
) -> Result<(), ReaderError> {
    Ok(())
}

#[cfg(windows)]
fn to_verbatim_path(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy().replace('/', "\\");
    if raw.starts_with(r"\\?\") {
        return path.to_path_buf();
    }
    PathBuf::from(format!(r"\\?\{raw}"))
}

#[cfg(windows)]
fn encode_win32_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    let has_verbatim_prefix =
        wide.len() >= 4 && wide[0] == 92 && wide[1] == 92 && wide[2] == 63 && wide[3] == 92;
    if wide.len() < 248 || has_verbatim_prefix {
        wide.push(0);
        return wide;
    }
    let mut wide: Vec<u16> = to_verbatim_path(path).as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}
