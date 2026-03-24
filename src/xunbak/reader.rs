use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::xunbak::blob::{BlobRecordError, copy_blob_record_content_to_writer, read_blob_record};
use crate::xunbak::checkpoint::{CheckpointError, CheckpointPayload, read_checkpoint_record};
use crate::xunbak::constants::{
    BLOB_HEADER_SIZE, FLAG_SPLIT, FOOTER_SIZE, HEADER_SIZE, RECORD_PREFIX_SIZE, RecordType,
};
use crate::xunbak::footer::{Footer, FooterError};
use crate::xunbak::header::{DecodedHeader, Header, HeaderError};
use crate::xunbak::manifest::{
    ManifestBody, ManifestEntry, ManifestError, read_manifest_record, unix_ns_to_filetime,
};
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreResult {
    pub restored_files: usize,
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
    #[error("path not found: {0}")]
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
        Ok(Self {
            path: primary_path,
            file_size: active_file_size,
            header,
            footer,
            checkpoint,
            is_split: header.header.flags & FLAG_SPLIT != 0,
            volume_paths: active_volume_paths,
        })
    }

    pub fn load_manifest(&self) -> Result<ManifestBody, ReaderError> {
        let manifest_path = self
            .volume_paths
            .last()
            .cloned()
            .unwrap_or_else(|| self.path.clone());
        let mut file =
            File::open(&manifest_path).map_err(|err| ReaderError::Io(err.to_string()))?;
        file.seek(SeekFrom::Start(self.checkpoint.manifest_offset))
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let manifest = read_manifest_record(&mut file)?;

        let payload_len = self
            .checkpoint
            .manifest_len
            .checked_sub(RECORD_PREFIX_SIZE as u64)
            .ok_or_else(|| {
                ReaderError::Io("manifest_len smaller than record prefix".to_string())
            })?;
        let mut payload = vec![0u8; payload_len as usize];
        file.seek(SeekFrom::Start(
            self.checkpoint.manifest_offset + RECORD_PREFIX_SIZE as u64,
        ))
        .map_err(|err| ReaderError::Io(err.to_string()))?;
        file.read_exact(&mut payload)
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        if crate::xunbak::checkpoint::compute_manifest_hash(&payload)
            != self.checkpoint.manifest_hash
        {
            return Err(ReaderError::ManifestHashMismatch);
        }

        Ok(manifest.body)
    }

    pub fn read_and_verify_blob(&self, entry: &ManifestEntry) -> Result<Vec<u8>, ReaderError> {
        if let Some(parts) = &entry.parts {
            let mut out = Vec::new();
            for part in parts {
                let volume_path = self.volume_path(part.volume_index)?;
                let mut file =
                    File::open(volume_path).map_err(|err| ReaderError::Io(err.to_string()))?;
                file.seek(SeekFrom::Start(part.blob_offset))
                    .map_err(|err| ReaderError::Io(err.to_string()))?;
                let blob = read_blob_record(&mut file)?;
                out.extend_from_slice(&blob.content);
            }
            if *blake3::hash(&out).as_bytes() != entry.content_hash {
                return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
            }
            return Ok(out);
        }

        let volume_path = self.volume_path(entry.volume_index)?;
        let mut file = File::open(volume_path).map_err(|err| ReaderError::Io(err.to_string()))?;
        file.seek(SeekFrom::Start(entry.blob_offset))
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let blob = read_blob_record(&mut file)?;
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
                let volume_path = self.volume_path(part.volume_index)?;
                let mut file =
                    File::open(volume_path).map_err(|err| ReaderError::Io(err.to_string()))?;
                file.seek(SeekFrom::Start(part.blob_offset))
                    .map_err(|err| ReaderError::Io(err.to_string()))?;
                let result = copy_blob_record_content_to_writer(&mut file, &mut hashing_writer)?;
                if result.header.blob_id != part.blob_id {
                    return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
                }
            }
            if hashing_writer.finalize() != entry.content_hash {
                return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
            }
            return Ok(());
        }

        let volume_path = self.volume_path(entry.volume_index)?;
        let mut file = File::open(volume_path).map_err(|err| ReaderError::Io(err.to_string()))?;
        file.seek(SeekFrom::Start(entry.blob_offset))
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let result = copy_blob_record_content_to_writer(&mut file, &mut hashing_writer)?;
        if result.header.blob_id != entry.blob_id || hashing_writer.finalize() != entry.content_hash
        {
            return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
        }
        Ok(())
    }

    pub fn restore_all(&self, target_dir: &Path) -> Result<RestoreResult, ReaderError> {
        let manifest = self.load_manifest()?;
        std::fs::create_dir_all(target_dir).map_err(|err| ReaderError::Io(err.to_string()))?;
        self.restore_matching(&manifest, target_dir, |entry| {
            let _ = entry;
            true
        })
    }

    pub fn restore_file(
        &self,
        path: &str,
        target_dir: &Path,
    ) -> Result<RestoreResult, ReaderError> {
        let manifest = self.load_manifest()?;
        let result = self.restore_matching(&manifest, target_dir, |entry| {
            entry.path.eq_ignore_ascii_case(path)
        })?;
        if result.restored_files == 0 {
            return Err(ReaderError::PathNotFound(path.to_string()));
        }
        Ok(result)
    }

    pub fn restore_glob(
        &self,
        pattern: &str,
        target_dir: &Path,
    ) -> Result<RestoreResult, ReaderError> {
        let manifest = self.load_manifest()?;
        self.restore_matching(&manifest, target_dir, |entry| {
            glob_match(pattern, &entry.path)
        })
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
    fn volume_path(&self, volume_index: u16) -> Result<&Path, ReaderError> {
        self.volume_paths
            .get(volume_index as usize)
            .map(|path| path.as_path())
            .ok_or(ReaderError::UnrecoverableContainer)
    }
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
    use super::sorted_restore_entries;
    use crate::xunbak::constants::Codec;
    use crate::xunbak::manifest::{ManifestBody, ManifestEntry};

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
        mut predicate: F,
    ) -> Result<RestoreResult, ReaderError>
    where
        F: FnMut(&ManifestEntry) -> bool,
    {
        let mut restored = 0usize;
        let mut entries = sorted_restore_entries(manifest, |entry| predicate(entry));
        for entry in entries.drain(..) {
            let dest = target_dir.join(entry.path.replace('/', "\\"));
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|err| ReaderError::Io(err.to_string()))?;
            }
            let mut file = File::create(&dest).map_err(|err| ReaderError::Io(err.to_string()))?;
            self.copy_and_verify_blob(entry, &mut file)?;
            apply_windows_metadata(&dest, entry)?;
            restored += 1;
        }
        Ok(RestoreResult {
            restored_files: restored,
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

#[cfg(windows)]
fn apply_windows_metadata(path: &Path, entry: &ManifestEntry) -> Result<(), ReaderError> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, SetFileTime};

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|err| ReaderError::Io(err.to_string()))?;
    let created_filetime = unix_ns_to_filetime(entry.created_time_ns as i128);
    let modified_filetime = unix_ns_to_filetime(entry.mtime_ns as i128);
    let created = windows_sys::Win32::Foundation::FILETIME {
        dwLowDateTime: created_filetime as u32,
        dwHighDateTime: (created_filetime >> 32) as u32,
    };
    let modified = windows_sys::Win32::Foundation::FILETIME {
        dwLowDateTime: modified_filetime as u32,
        dwHighDateTime: (modified_filetime >> 32) as u32,
    };
    let ok = unsafe {
        SetFileTime(
            file.as_raw_handle() as HANDLE,
            &created,
            std::ptr::null(),
            &modified,
        )
    };
    if ok == 0 {
        return Err(ReaderError::Io(format!(
            "SetFileTime failed for {}",
            path.display()
        )));
    }

    let verbatim = to_verbatim_path(path);
    let mut wide: Vec<u16> = verbatim.as_os_str().encode_wide().collect();
    wide.push(0);
    let set_attr_ok = unsafe { SetFileAttributesW(wide.as_ptr(), entry.win_attributes) };
    if set_attr_ok == 0 {
        return Err(ReaderError::Io(format!(
            "SetFileAttributesW failed for {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(windows))]
fn apply_windows_metadata(_path: &Path, _entry: &ManifestEntry) -> Result<(), ReaderError> {
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
