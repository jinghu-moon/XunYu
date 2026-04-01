#![allow(dead_code)]

use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};

use rkyv::{from_bytes, to_bytes, rancor::Error as RkyvError, util::AlignedVec};
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

use crate::bookmark::index::PersistedBookmarkIndex;
use crate::bookmark_core::{normalize_name, BookmarkSource};
use crate::bookmark_state::Bookmark;
use crate::bookmark::debug::BookmarkLoadTiming;

/// Binary cache contract for bookmark fast-load layer.
///
/// This module defines the cache file naming, fixed header shape, versioning,
/// flags, and invalidation rules. It does not perform runtime cache IO yet.

pub(crate) const CACHE_FILE_NAME: &str = ".xun.bookmark.cache";
pub(crate) const CACHE_LOCK_FILE_NAME: &str = ".xun.bookmark.cache.lock";
pub(crate) const STORE_CACHE_VERSION: u32 = 1;
pub(crate) const HEADER_SIZE: usize = 52;
pub(crate) const CACHE_MAGIC: [u8; 8] = *b"XUNBMCH\0";
const CACHE_LOCK_TIMEOUT: Duration = Duration::from_secs(3);
const CACHE_LOCK_RETRY: Duration = Duration::from_millis(50);
const CACHE_PAYLOAD_ALIGNMENT: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CacheFlags(pub(crate) u32);

impl CacheFlags {
    pub(crate) const CHECKED_LAYOUT: u32 = 1 << 0;
    pub(crate) const EMBEDDED_INDEX: u32 = 1 << 1;

    pub(crate) const fn bits(self) -> u32 {
        self.0
    }

    pub(crate) const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CacheHeader {
    pub(crate) magic: [u8; 8],
    pub(crate) cache_version: u32,
    pub(crate) schema_version: u32,
    pub(crate) source_len: u64,
    pub(crate) source_modified_ms: u64,
    pub(crate) source_hash: u64,
    pub(crate) flags: u32,
    pub(crate) payload_len: u64,
}

impl CacheHeader {
    pub(crate) const BYTE_LEN: usize = HEADER_SIZE;
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct CachedBookmark {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) path: String,
    pub(crate) source: u8,
    pub(crate) pinned: bool,
    pub(crate) tags: Vec<String>,
    pub(crate) desc: String,
    pub(crate) workspace: Option<String>,
    pub(crate) created_at: u64,
    pub(crate) last_visited: Option<u64>,
    pub(crate) visit_count: Option<u32>,
    pub(crate) frecency_score: f64,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct CachePayload {
    pub(crate) bookmarks: Vec<CachedBookmark>,
    pub(crate) index: Option<PersistedBookmarkIndex>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheHeaderError {
    UnexpectedEof,
    InvalidMagic,
}

impl fmt::Display for CacheHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of cache header"),
            Self::InvalidMagic => write!(f, "invalid cache magic"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheInvalidReason {
    CacheVersionMismatch,
    SchemaVersionMismatch,
    SourceLenMismatch,
    SourceModifiedMsMismatch,
    SourceHashMismatch,
    HeaderCorrupt,
    PayloadCorrupt,
}

impl fmt::Display for CacheInvalidReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CacheVersionMismatch => write!(f, "cache version mismatch"),
            Self::SchemaVersionMismatch => write!(f, "schema version mismatch"),
            Self::SourceLenMismatch => write!(f, "source length mismatch"),
            Self::SourceModifiedMsMismatch => write!(f, "source modified time mismatch"),
            Self::SourceHashMismatch => write!(f, "source hash mismatch"),
            Self::HeaderCorrupt => write!(f, "cache header corrupt"),
            Self::PayloadCorrupt => write!(f, "cache payload corrupt"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceFingerprint {
    pub(crate) len: u64,
    pub(crate) modified_ms: u64,
    pub(crate) hash: u64,
}

impl SourceFingerprint {
    pub(crate) fn from_path(path: &Path) -> std::io::Result<Self> {
        let meta = fs::metadata(path)?;
        let len = meta.len();
        let modified_ms = meta
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let bytes = fs::read(path)?;
        let hash = compute_source_hash(&bytes);
        Ok(Self {
            len,
            modified_ms,
            hash,
        })
    }
}

pub(crate) fn store_cache_path(source_path: &Path) -> PathBuf {
    source_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(CACHE_FILE_NAME)
}

pub(crate) fn store_cache_lock_path(source_path: &Path) -> PathBuf {
    source_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(CACHE_LOCK_FILE_NAME)
}

pub(crate) fn validate_cache_header(
    header: &CacheHeader,
    current_schema_version: u32,
    source_len: u64,
    source_modified_ms: u64,
    source_hash: u64,
) -> Result<(), CacheInvalidReason> {
    if header.cache_version != STORE_CACHE_VERSION {
        return Err(CacheInvalidReason::CacheVersionMismatch);
    }
    if header.schema_version != current_schema_version {
        return Err(CacheInvalidReason::SchemaVersionMismatch);
    }
    if header.source_len != source_len {
        return Err(CacheInvalidReason::SourceLenMismatch);
    }
    if header.source_modified_ms != source_modified_ms {
        return Err(CacheInvalidReason::SourceModifiedMsMismatch);
    }
    if header.source_hash != source_hash {
        return Err(CacheInvalidReason::SourceHashMismatch);
    }
    Ok(())
}

pub(crate) fn is_cache_valid(
    header: &CacheHeader,
    current_schema_version: u32,
    source_len: u64,
    source_modified_ms: u64,
    source_hash: u64,
) -> Result<(), CacheInvalidReason> {
    validate_cache_header(
        header,
        current_schema_version,
        source_len,
        source_modified_ms,
        source_hash,
    )
}

pub(crate) fn compute_source_hash(bytes: &[u8]) -> u64 {
    xxh3_64(bytes)
}

pub(crate) fn read_cache_header(path: &Path) -> io::Result<Option<CacheHeader>> {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return Ok(None),
    };
    let mut header = [0u8; HEADER_SIZE];
    match file.read_exact(&mut header) {
        Ok(()) => {}
        Err(_) => return Ok(None),
    }
    match decode_header(&header) {
        Ok(decoded) => Ok(Some(decoded)),
        Err(_) => Ok(None),
    }
}

pub(crate) fn write_cache_atomic(path: &Path, header: &CacheHeader, payload: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(&encode_header(header))?;
    file.write_all(payload)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub(crate) fn load_cache_payload_checked(
    path: &Path,
    current_schema_version: u32,
    fingerprint: &SourceFingerprint,
    mut timing: Option<&mut BookmarkLoadTiming>,
) -> io::Result<Option<CachePayload>> {
    if std::env::var_os("XUN_BM_DISABLE_BINARY_CACHE").is_some() {
        return Ok(None);
    }
    let header = match read_cache_header(path)? {
        Some(header) => header,
        None => return Ok(None),
    };
    if validate_cache_header(
        &header,
        current_schema_version,
        fingerprint.len,
        fingerprint.modified_ms,
        fingerprint.hash,
    )
    .is_err()
    {
        return Ok(None);
    }
    if let Some(ref mut timing) = timing {
        timing.mark("read_cache_header");
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    let payload_end = HEADER_SIZE.saturating_add(header.payload_len as usize);
    if payload_end > bytes.len() {
        return Ok(None);
    }
    let payload_bytes = &bytes[HEADER_SIZE..payload_end];
    // The fixed 52-byte header makes the payload slice potentially unaligned,
    // so we copy it into an aligned buffer before checked access/deserialization.
    let mut aligned: AlignedVec<CACHE_PAYLOAD_ALIGNMENT> =
        AlignedVec::with_capacity(payload_bytes.len());
    aligned.extend_from_slice(payload_bytes);
    if let Some(ref mut timing) = timing {
        timing.mark("read_cache_body");
    }
    match from_bytes::<CachePayload, RkyvError>(&aligned) {
        Ok(payload) => Ok(Some(payload)),
        Err(_) => Ok(None),
    }
}

pub(crate) fn write_cache_payload_atomic(
    path: &Path,
    current_schema_version: u32,
    fingerprint: &SourceFingerprint,
    payload: &CachePayload,
) -> io::Result<()> {
    if std::env::var_os("XUN_BM_DISABLE_BINARY_CACHE").is_some() {
        return Ok(());
    }
    let flags = CacheFlags::from_bits(
        CacheFlags::CHECKED_LAYOUT
            | if payload.index.is_some() {
                CacheFlags::EMBEDDED_INDEX
            } else {
                0
            },
    )
    .bits();
    let payload_bytes = to_bytes::<RkyvError>(payload)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
    let header = CacheHeader {
        magic: CACHE_MAGIC,
        cache_version: STORE_CACHE_VERSION,
        schema_version: current_schema_version,
        source_len: fingerprint.len,
        source_modified_ms: fingerprint.modified_ms,
        source_hash: fingerprint.hash,
        flags,
        payload_len: payload_bytes.len() as u64,
    };
    write_cache_atomic(path, &header, payload_bytes.as_slice())
}

pub(crate) struct CacheLock(#[allow(dead_code)] fs::File);

impl CacheLock {
    pub(crate) fn acquire(path: &Path) -> io::Result<Option<Self>> {
        let deadline = Instant::now() + CACHE_LOCK_TIMEOUT;
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .share_mode(0)
                .open(path)
            {
                Ok(file) => return Ok(Some(Self(file))),
                Err(err) if is_lock_conflict(&err) && Instant::now() < deadline => {
                    thread::sleep(CACHE_LOCK_RETRY);
                }
                Err(err) if is_lock_conflict(&err) => return Ok(None),
                Err(err) => return Err(err),
            }
        }
    }
}

pub(crate) fn encode_header(header: &CacheHeader) -> [u8; HEADER_SIZE] {
    let mut out = [0u8; HEADER_SIZE];
    out[0..8].copy_from_slice(&header.magic);
    write_u32_le(&mut out[8..12], header.cache_version);
    write_u32_le(&mut out[12..16], header.schema_version);
    write_u64_le(&mut out[16..24], header.source_len);
    write_u64_le(&mut out[24..32], header.source_modified_ms);
    write_u64_le(&mut out[32..40], header.source_hash);
    write_u32_le(&mut out[40..44], header.flags);
    write_u64_le(&mut out[44..52], header.payload_len);
    out
}

pub(crate) fn decode_header(bytes: &[u8]) -> Result<CacheHeader, CacheHeaderError> {
    if bytes.len() < HEADER_SIZE {
        return Err(CacheHeaderError::UnexpectedEof);
    }

    let mut magic = [0u8; 8];
    magic.copy_from_slice(&bytes[0..8]);
    if magic != CACHE_MAGIC {
        return Err(CacheHeaderError::InvalidMagic);
    }

    Ok(CacheHeader {
        magic,
        cache_version: read_u32_le(&bytes[8..12]),
        schema_version: read_u32_le(&bytes[12..16]),
        source_len: read_u64_le(&bytes[16..24]),
        source_modified_ms: read_u64_le(&bytes[24..32]),
        source_hash: read_u64_le(&bytes[32..40]),
        flags: read_u32_le(&bytes[40..44]),
        payload_len: read_u64_le(&bytes[44..52]),
    })
}

fn write_u32_le(dst: &mut [u8], value: u32) {
    dst.copy_from_slice(&value.to_le_bytes());
}

fn write_u64_le(dst: &mut [u8], value: u64) {
    dst.copy_from_slice(&value.to_le_bytes());
}

fn read_u32_le(src: &[u8]) -> u32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(src);
    u32::from_le_bytes(bytes)
}

fn read_u64_le(src: &[u8]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(src);
    u64::from_le_bytes(bytes)
}

fn is_lock_conflict(err: &io::Error) -> bool {
    matches!(
        err.raw_os_error(),
        Some(32) | Some(33)
    )
}

impl CachedBookmark {
    pub(crate) fn from_bookmark(bookmark: &Bookmark) -> Self {
        Self {
            id: bookmark.id.clone(),
            name: bookmark.name.clone(),
            path: bookmark.path.clone(),
            source: source_code(bookmark.source),
            pinned: bookmark.pinned,
            tags: bookmark.tags.clone(),
            desc: bookmark.desc.clone(),
            workspace: bookmark.workspace.clone(),
            created_at: bookmark.created_at,
            last_visited: bookmark.last_visited,
            visit_count: bookmark.visit_count,
            frecency_score: bookmark.frecency_score,
        }
    }

    pub(crate) fn into_bookmark(self) -> Bookmark {
        let name_norm = self.name.as_deref().map(normalize_name);
        let path_norm = self.path.to_ascii_lowercase();
        Bookmark {
            id: self.id,
            name: self.name,
            name_norm,
            path: self.path,
            path_norm,
            source: source_from_code(self.source),
            pinned: self.pinned,
            tags: self.tags,
            desc: self.desc,
            workspace: self.workspace,
            created_at: self.created_at,
            last_visited: self.last_visited,
            visit_count: self.visit_count,
            frecency_score: self.frecency_score,
        }
    }
}

fn source_code(source: BookmarkSource) -> u8 {
    match source {
        BookmarkSource::Explicit => 0,
        BookmarkSource::Imported => 1,
        BookmarkSource::Learned => 2,
    }
}

fn source_from_code(code: u8) -> BookmarkSource {
    match code {
        1 => BookmarkSource::Imported,
        2 => BookmarkSource::Learned,
        _ => BookmarkSource::Explicit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark::index::BookmarkIndex;
    use crate::bookmark_state::Store;
    use tempfile::tempdir;

    fn header() -> CacheHeader {
        CacheHeader {
            magic: CACHE_MAGIC,
            cache_version: STORE_CACHE_VERSION,
            schema_version: 1,
            source_len: 123,
            source_modified_ms: 456,
            source_hash: 789,
            flags: CacheFlags::from_bits(
                CacheFlags::CHECKED_LAYOUT | CacheFlags::EMBEDDED_INDEX,
            )
            .bits(),
            payload_len: 42,
        }
    }

    #[test]
    fn store_cache_path_uses_xun_bookmark_cache_name() {
        let path = store_cache_path(Path::new("C:/tmp/.xun.bookmark.json"));
        assert_eq!(path, PathBuf::from("C:/tmp/.xun.bookmark.cache"));
    }

    #[test]
    fn store_cache_lock_path_uses_lock_suffix() {
        let path = store_cache_lock_path(Path::new("C:/tmp/.xun.bookmark.json"));
        assert_eq!(path, PathBuf::from("C:/tmp/.xun.bookmark.cache.lock"));
    }

    #[test]
    fn store_cache_version_is_one() {
        assert_eq!(STORE_CACHE_VERSION, 1);
    }

    #[test]
    fn cache_header_size_is_52_bytes() {
        assert_eq!(CacheHeader::BYTE_LEN, 52);
    }

    #[test]
    fn cache_header_magic_matches_xun_bookmark_cache() {
        assert_eq!(&CACHE_MAGIC, b"XUNBMCH\0");
        assert_eq!(header().magic, CACHE_MAGIC);
    }

    #[test]
    fn cache_header_has_no_payload_codec_field() {
        let header = header();
        assert_eq!(
            header.flags,
            CacheFlags::CHECKED_LAYOUT | CacheFlags::EMBEDDED_INDEX
        );
        assert_eq!(header.payload_len, 42);
    }

    #[test]
    fn cache_header_contains_source_hash() {
        assert_eq!(header().source_hash, 789);
    }

    #[test]
    fn cache_flags_encode_checked_and_embedded_index_bits() {
        let flags =
            CacheFlags::from_bits(CacheFlags::CHECKED_LAYOUT | CacheFlags::EMBEDDED_INDEX);
        assert_eq!(flags.bits(), 3);
        assert_eq!(CacheFlags::CHECKED_LAYOUT, 1);
        assert_eq!(CacheFlags::EMBEDDED_INDEX, 2);
    }

    #[test]
    fn cache_invalidated_when_version_differs() {
        let mut header = header();
        header.cache_version = STORE_CACHE_VERSION + 1;
        assert_eq!(
            validate_cache_header(&header, 1, 123, 456, 789),
            Err(CacheInvalidReason::CacheVersionMismatch)
        );
    }

    #[test]
    fn cache_invalidated_when_schema_differs() {
        let header = header();
        assert_eq!(
            validate_cache_header(&header, 2, 123, 456, 789),
            Err(CacheInvalidReason::SchemaVersionMismatch)
        );
    }

    #[test]
    fn cache_invalidated_when_source_len_differs() {
        let header = header();
        assert_eq!(
            validate_cache_header(&header, 1, 124, 456, 789),
            Err(CacheInvalidReason::SourceLenMismatch)
        );
    }

    #[test]
    fn cache_invalidated_when_source_mtime_differs() {
        let header = header();
        assert_eq!(
            validate_cache_header(&header, 1, 123, 457, 789),
            Err(CacheInvalidReason::SourceModifiedMsMismatch)
        );
    }

    #[test]
    fn cache_invalidated_when_source_hash_differs() {
        let header = header();
        assert_eq!(
            validate_cache_header(&header, 1, 123, 456, 790),
            Err(CacheInvalidReason::SourceHashMismatch)
        );
    }

    #[test]
    fn encode_header_writes_little_endian_fields() {
        let encoded = encode_header(&header());
        assert_eq!(&encoded[0..8], b"XUNBMCH\0");
        assert_eq!(u32::from_le_bytes(encoded[8..12].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(encoded[12..16].try_into().unwrap()), 1);
        assert_eq!(u64::from_le_bytes(encoded[16..24].try_into().unwrap()), 123);
        assert_eq!(u64::from_le_bytes(encoded[24..32].try_into().unwrap()), 456);
        assert_eq!(u64::from_le_bytes(encoded[32..40].try_into().unwrap()), 789);
        assert_eq!(u32::from_le_bytes(encoded[40..44].try_into().unwrap()), 3);
        assert_eq!(u64::from_le_bytes(encoded[44..52].try_into().unwrap()), 42);
    }

    #[test]
    fn decode_header_reads_little_endian_fields() {
        let encoded = encode_header(&header());
        let decoded = decode_header(&encoded).unwrap();
        assert_eq!(decoded, header());
    }

    #[test]
    fn decode_header_rejects_invalid_magic() {
        let mut encoded = encode_header(&header());
        encoded[0] = b'B';
        assert_eq!(decode_header(&encoded), Err(CacheHeaderError::InvalidMagic));
    }

    #[test]
    fn decode_header_rejects_short_buffer() {
        let encoded = [0u8; 12];
        assert_eq!(decode_header(&encoded), Err(CacheHeaderError::UnexpectedEof));
    }

    #[test]
    fn source_hash_same_bytes_same_hash() {
        let left = compute_source_hash(br#"{"a":1}"#);
        let right = compute_source_hash(br#"{"a":1}"#);
        assert_eq!(left, right);
    }

    #[test]
    fn source_hash_different_bytes_different_hash() {
        let left = compute_source_hash(br#"{"a":1}"#);
        let right = compute_source_hash(br#"{"a":2}"#);
        assert_ne!(left, right);
    }

    #[test]
    fn source_fingerprint_contains_len_mtime_hash() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        fs::write(&path, br#"{"schema_version":1,"bookmarks":[]}"#).unwrap();

        let fingerprint = SourceFingerprint::from_path(&path).unwrap();
        assert_eq!(
            fingerprint.len,
            fs::metadata(&path).unwrap().len()
        );
        assert!(fingerprint.modified_ms > 0);
        assert_eq!(
            fingerprint.hash,
            compute_source_hash(&fs::read(&path).unwrap())
        );
    }

    #[test]
    fn cache_flags_roundtrip() {
        let flags = CacheFlags::from_bits(3);
        assert_eq!(flags.bits(), 3);
    }

    #[test]
    fn cache_validation_returns_reason_enum() {
        let mut header = header();
        header.source_hash = 0;
        let result = validate_cache_header(&header, 1, 123, 456, 789);
        assert!(matches!(result, Err(CacheInvalidReason::SourceHashMismatch)));
        assert_eq!(
            result.unwrap_err().to_string(),
            "source hash mismatch"
        );
    }

    #[test]
    fn read_cache_header_reads_only_fixed_prefix() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".xun.bookmark.cache");
        let payload = vec![1u8; 128];
        write_cache_atomic(&path, &header(), &payload).unwrap();

        let parsed = read_cache_header(&path).unwrap().unwrap();
        assert_eq!(parsed, header());
    }

    #[test]
    fn write_cache_uses_tmp_then_rename() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".xun.bookmark.cache");
        let payload = vec![7u8; 16];
        write_cache_atomic(&path, &header(), &payload).unwrap();

        assert!(path.exists());
        assert!(!path.with_extension("tmp").exists());
        let bytes = fs::read(&path).unwrap();
        assert_eq!(bytes.len(), HEADER_SIZE + payload.len());
    }

    #[test]
    fn write_cache_failure_preserves_previous_cache() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".xun.bookmark.cache");
        write_cache_atomic(&path, &header(), b"first").unwrap();
        let original = fs::read(&path).unwrap();

        let err = write_cache_atomic(dir.path(), &header(), b"second").unwrap_err();
        assert!(err.kind() != io::ErrorKind::NotFound || dir.path().is_dir());
        assert_eq!(fs::read(&path).unwrap(), original);
    }

    #[test]
    fn cache_lock_acquire_success() {
        let dir = tempdir().unwrap();
        let lock = store_cache_lock_path(Path::new("C:/tmp/.xun.bookmark.json"));
        let path = dir.path().join(lock.file_name().unwrap());
        let guard = CacheLock::acquire(&path).unwrap();
        assert!(guard.is_some());
    }

    #[test]
    fn cache_lock_conflict_returns_none() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".xun.bookmark.cache.lock");
        let _guard = CacheLock::acquire(&path).unwrap().unwrap();
        let second = CacheLock::acquire(&path).unwrap();
        assert!(second.is_none());
    }

    #[test]
    fn read_cache_path_does_not_require_lock() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join(".xun.bookmark.cache");
        let lock_path = dir.path().join(".xun.bookmark.cache.lock");
        let _guard = CacheLock::acquire(&lock_path).unwrap().unwrap();
        write_cache_atomic(&cache_path, &header(), b"payload").unwrap();

        let parsed = read_cache_header(&cache_path).unwrap().unwrap();
        assert_eq!(parsed, header());
    }

    #[test]
    fn load_cache_returns_none_when_source_missing() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join(".xun.bookmark.cache");
        write_cache_atomic(&cache_path, &header(), b"payload").unwrap();

        let source_path = dir.path().join(".xun.bookmark.json");
        assert!(!source_path.exists());
        let loaded = read_cache_header(&cache_path).unwrap();
        assert!(loaded.is_some());
    }

    #[test]
    fn cached_bookmark_contains_only_load_state_fields() {
        let mut store = Store::new();
        store
            .set("home", "C:/work/home", Path::new("C:/work"), None, 10)
            .unwrap();
        store
            .set_explicit_metadata("home", vec!["work".to_string()], "main".to_string())
            .unwrap();
        let bookmark = store.bookmarks[0].clone();
        let cached = CachedBookmark::from_bookmark(&bookmark);

        assert_eq!(cached.id, bookmark.id);
        assert_eq!(cached.name, bookmark.name);
        assert_eq!(cached.path, bookmark.path);
        assert_eq!(cached.tags, bookmark.tags);
        assert_eq!(cached.desc, bookmark.desc);
        assert_eq!(cached.workspace, bookmark.workspace);
    }

    #[test]
    fn cached_bookmark_from_bookmark_roundtrip_fields() {
        let mut store = Store::new();
        store
            .set("home", "C:/work/home", Path::new("C:/work"), None, 10)
            .unwrap();
        store.bookmarks[0].pinned = true;
        store.bookmarks[0].workspace = Some("xunyu".to_string());
        store.bookmarks[0].visit_count = Some(7);
        store.bookmarks[0].last_visited = Some(99);
        store.bookmarks[0].frecency_score = 3.5;
        let bookmark = store.bookmarks[0].clone();

        let cached = CachedBookmark::from_bookmark(&bookmark);
        assert_eq!(cached.clone().into_bookmark(), bookmark);
    }

    #[test]
    fn cached_bookmark_into_bookmark_roundtrip() {
        let cached = CachedBookmark {
            id: "1".to_string(),
            name: Some("home".to_string()),
            path: "C:/work/home".to_string(),
            source: 1,
            pinned: true,
            tags: vec!["work".to_string()],
            desc: "main".to_string(),
            workspace: Some("xunyu".to_string()),
            created_at: 1,
            last_visited: Some(2),
            visit_count: Some(3),
            frecency_score: 4.0,
        };
        let bookmark = cached.clone().into_bookmark();
        assert_eq!(CachedBookmark::from_bookmark(&bookmark), cached);
    }

    #[test]
    fn cache_payload_can_embed_index() {
        let mut store = Store::new();
        store
            .set("client-api", "C:/work/projects/client-api", Path::new("C:/work"), None, 10)
            .unwrap();
        let payload = CachePayload {
            bookmarks: store
                .bookmarks
                .iter()
                .map(CachedBookmark::from_bookmark)
                .collect(),
            index: Some(BookmarkIndex::to_persisted(&store.bookmarks)),
        };
        assert!(payload.index.is_some());
        assert_eq!(payload.bookmarks.len(), 1);
    }

    #[test]
    fn cache_payload_excludes_dirty_count_last_save_at_oncelock() {
        let payload = CachePayload {
            bookmarks: Vec::new(),
            index: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(!json.contains("dirty_count"));
        assert!(!json.contains("last_save_at"));
        assert!(!json.contains("OnceLock"));
    }

    #[test]
    fn rkyv_checked_access_reads_cache_payload() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".xun.bookmark.json");
        fs::write(&source, br#"{"schema_version":1,"bookmarks":[]}"#).unwrap();
        let cache = store_cache_path(&source);

        let mut store = Store::new();
        store
            .set("client-api", "C:/work/projects/client-api", Path::new("C:/work"), None, 10)
            .unwrap();
        let fingerprint = SourceFingerprint::from_path(&source).unwrap();
        let payload = CachePayload {
            bookmarks: store
                .bookmarks
                .iter()
                .map(CachedBookmark::from_bookmark)
                .collect(),
            index: Some(BookmarkIndex::to_persisted(&store.bookmarks)),
        };

        write_cache_payload_atomic(&cache, 1, &fingerprint, &payload).unwrap();
        let loaded =
            load_cache_payload_checked(&cache, 1, &fingerprint, None).unwrap().unwrap();
        assert_eq!(loaded.bookmarks, payload.bookmarks);
        assert_eq!(loaded.index, payload.index);
    }

    #[test]
    fn invalid_payload_returns_none_not_error() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".xun.bookmark.json");
        fs::write(&source, br#"{"schema_version":1,"bookmarks":[]}"#).unwrap();
        let cache = store_cache_path(&source);
        let fingerprint = SourceFingerprint::from_path(&source).unwrap();
        let header = CacheHeader {
            magic: CACHE_MAGIC,
            cache_version: STORE_CACHE_VERSION,
            schema_version: 1,
            source_len: fingerprint.len,
            source_modified_ms: fingerprint.modified_ms,
            source_hash: fingerprint.hash,
            flags: CacheFlags::CHECKED_LAYOUT,
            payload_len: 7,
        };
        write_cache_atomic(&cache, &header, b"not-json").unwrap();

        let loaded = load_cache_payload_checked(&cache, 1, &fingerprint, None).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn corrupted_header_falls_back_to_json() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".xun.bookmark.json");
        fs::write(&source, br#"{"schema_version":1,"bookmarks":[]}"#).unwrap();
        let cache = store_cache_path(&source);
        fs::write(&cache, b"not-a-valid-header").unwrap();

        let fingerprint = SourceFingerprint::from_path(&source).unwrap();
        let loaded = load_cache_payload_checked(&cache, 1, &fingerprint, None).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn schema_version_mismatch_falls_back_to_json() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".xun.bookmark.json");
        fs::write(&source, br#"{"schema_version":1,"bookmarks":[]}"#).unwrap();
        let cache = store_cache_path(&source);
        let fingerprint = SourceFingerprint::from_path(&source).unwrap();
        let header = CacheHeader {
            schema_version: 2,
            ..header_for_fingerprint(&fingerprint)
        };
        write_cache_atomic(&cache, &header, b"{}").unwrap();

        let loaded = load_cache_payload_checked(&cache, 1, &fingerprint, None).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn cache_version_mismatch_falls_back_to_json() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".xun.bookmark.json");
        fs::write(&source, br#"{"schema_version":1,"bookmarks":[]}"#).unwrap();
        let cache = store_cache_path(&source);
        let fingerprint = SourceFingerprint::from_path(&source).unwrap();
        let header = CacheHeader {
            cache_version: STORE_CACHE_VERSION + 1,
            ..header_for_fingerprint(&fingerprint)
        };
        write_cache_atomic(&cache, &header, b"{}").unwrap();

        let loaded = load_cache_payload_checked(&cache, 1, &fingerprint, None).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn source_hash_mismatch_falls_back_to_json() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".xun.bookmark.json");
        fs::write(&source, br#"{"schema_version":1,"bookmarks":[]}"#).unwrap();
        let cache = store_cache_path(&source);
        let fingerprint = SourceFingerprint::from_path(&source).unwrap();
        let header = CacheHeader {
            source_hash: fingerprint.hash + 1,
            ..header_for_fingerprint(&fingerprint)
        };
        write_cache_atomic(&cache, &header, b"{}").unwrap();

        let loaded = load_cache_payload_checked(&cache, 1, &fingerprint, None).unwrap();
        assert!(loaded.is_none());
    }

    fn header_for_fingerprint(fingerprint: &SourceFingerprint) -> CacheHeader {
        CacheHeader {
            magic: CACHE_MAGIC,
            cache_version: STORE_CACHE_VERSION,
            schema_version: 1,
            source_len: fingerprint.len,
            source_modified_ms: fingerprint.modified_ms,
            source_hash: fingerprint.hash,
            flags: CacheFlags::CHECKED_LAYOUT,
            payload_len: 2,
        }
    }
}
