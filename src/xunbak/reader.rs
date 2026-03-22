use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::xunbak::blob::{BlobRecordError, read_blob_record};
use crate::xunbak::checkpoint::{CheckpointError, CheckpointPayload, read_checkpoint_record};
use crate::xunbak::constants::{
    BLOB_HEADER_SIZE, FOOTER_SIZE, HEADER_SIZE, RECORD_PREFIX_SIZE, RecordType,
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
        let mut file = File::open(path).map_err(|err| ReaderError::Io(err.to_string()))?;
        let file_size = file
            .metadata()
            .map_err(|err| ReaderError::Io(err.to_string()))?
            .len();
        if file_size < (HEADER_SIZE + FOOTER_SIZE) as u64 {
            return Err(ReaderError::ContainerTooSmall { actual: file_size });
        }

        let mut header_bytes = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_bytes)
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let header = Header::from_bytes(&header_bytes)?;

        let (footer, checkpoint) = match read_footer(&mut file, file_size) {
            Ok(footer) => {
                file.seek(SeekFrom::Start(footer.checkpoint_offset))
                    .map_err(|err| ReaderError::Io(err.to_string()))?;
                let checkpoint = read_checkpoint_record(&mut file)?.payload;
                (footer, checkpoint)
            }
            Err(_) => {
                let (offset, checkpoint) = fallback_scan(&mut file)?;
                (
                    Footer {
                        checkpoint_offset: offset,
                    },
                    checkpoint,
                )
            }
        };

        Ok(Self {
            path: path.to_path_buf(),
            file_size,
            header,
            footer,
            checkpoint,
        })
    }

    pub fn load_manifest(&self) -> Result<ManifestBody, ReaderError> {
        let mut file = File::open(&self.path).map_err(|err| ReaderError::Io(err.to_string()))?;
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
                let mut file =
                    File::open(&self.path).map_err(|err| ReaderError::Io(err.to_string()))?;
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

        let mut file = File::open(&self.path).map_err(|err| ReaderError::Io(err.to_string()))?;
        file.seek(SeekFrom::Start(entry.blob_offset))
            .map_err(|err| ReaderError::Io(err.to_string()))?;
        let blob = read_blob_record(&mut file)?;
        if *blake3::hash(&blob.content).as_bytes() != entry.content_hash {
            return Err(ReaderError::Blob(BlobRecordError::BlobHashMismatch));
        }
        Ok(blob.content)
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

fn fallback_scan(file: &mut File) -> Result<(u64, CheckpointPayload), ReaderError> {
    file.seek(SeekFrom::Start(HEADER_SIZE as u64))
        .map_err(|err| ReaderError::Io(err.to_string()))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|err| ReaderError::Io(err.to_string()))?;
    let mut offset = 0usize;
    let mut last_checkpoint_offset = None;
    while offset + RECORD_PREFIX_SIZE <= bytes.len() {
        let prefix = match RecordPrefix::from_bytes(&bytes[offset..offset + RECORD_PREFIX_SIZE]) {
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
            last_checkpoint_offset = Some(HEADER_SIZE as u64 + offset as u64);
        }
        offset = payload_end;
    }
    let checkpoint_offset = last_checkpoint_offset.ok_or(ReaderError::UnrecoverableContainer)?;
    file.seek(SeekFrom::Start(checkpoint_offset))
        .map_err(|err| ReaderError::Io(err.to_string()))?;
    let checkpoint = read_checkpoint_record(file)?.payload;
    Ok((checkpoint_offset, checkpoint))
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
        for entry in &manifest.entries {
            if !predicate(entry) {
                continue;
            }
            let content = self.read_and_verify_blob(entry)?;
            let dest = target_dir.join(entry.path.replace('/', "\\"));
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|err| ReaderError::Io(err.to_string()))?;
            }
            std::fs::write(&dest, &content).map_err(|err| ReaderError::Io(err.to_string()))?;
            apply_windows_metadata(&dest, entry)?;
            restored += 1;
        }
        Ok(RestoreResult {
            restored_files: restored,
        })
    }
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
    let raw = path.to_string_lossy();
    if raw.starts_with(r"\\?\") {
        return path.to_path_buf();
    }
    PathBuf::from(format!(r"\\?\{raw}"))
}
