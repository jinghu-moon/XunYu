use std::collections::HashMap;
use std::ffi::c_void;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use xun::xunbak::blob::{BlobHeader, BlobRecordError, copy_blob_record_content_to_writer};
use xun::xunbak::checkpoint::{
    CheckpointError, CheckpointPayload, compute_manifest_hash, read_checkpoint_record,
};
use xun::xunbak::constants::{
    BLOB_HEADER_SIZE, FLAG_SPLIT, FOOTER_SIZE, HEADER_SIZE, RECORD_PREFIX_SIZE,
};
use xun::xunbak::footer::{Footer, FooterError};
use xun::xunbak::header::{DecodedHeader, Header, HeaderError};
use xun::xunbak::manifest::{ManifestBody, ManifestEntry, ManifestError, read_manifest_record};

pub trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

pub trait VolumeSource {
    fn open(&self, volume_name: &str) -> Result<Box<dyn ReadSeek>, CoreError>;
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(String),
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
    #[error("container too small: {actual}")]
    ContainerTooSmall { actual: u64 },
    #[error("container not found: {0}")]
    ContainerNotFound(String),
    #[error("path has no file name: {0}")]
    PathHasNoFileName(String),
    #[error("invalid split state: {0}")]
    InvalidSplitState(String),
    #[error("missing split volume: {0}")]
    MissingSplitVolume(String),
    #[error("manifest hash mismatch")]
    ManifestHashMismatch,
    #[error("missing archive item: {0}")]
    MissingItem(String),
    #[error("volume index out of range: requested={requested}, available={available}")]
    VolumeIndexOutOfRange { requested: u16, available: usize },
    #[error("seek offset overflow: {0}")]
    SeekOverflow(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveItem {
    pub path: String,
    pub size: u64,
    pub packed_size: u64,
    pub mtime_ns: u64,
    pub created_time_ns: u64,
    pub win_attributes: u32,
    pub volume_index: u16,
    pub codec_id: u32,
}

#[derive(Clone, Debug)]
pub struct FsVolumeSource {
    pub root: PathBuf,
}

#[derive(Clone, Debug)]
pub struct MemoryVolumeSource {
    volumes: HashMap<String, Arc<[u8]>>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XunbakVolumeCallbacks {
    pub ctx: *mut c_void,
    pub open_volume: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            volume_name_ptr: *const u8,
            volume_name_len: usize,
            out_handle: *mut *mut c_void,
        ) -> i32,
    >,
    pub read: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            stream_handle: *mut c_void,
            out_buf: *mut u8,
            buf_len: usize,
            out_read: *mut usize,
        ) -> i32,
    >,
    pub seek: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            stream_handle: *mut c_void,
            offset: i64,
            origin: u32,
            out_pos: *mut u64,
        ) -> i32,
    >,
    pub close_volume: Option<unsafe extern "C" fn(ctx: *mut c_void, stream_handle: *mut c_void)>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XunbakWriteCallbacks {
    pub ctx: *mut c_void,
    pub write: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            data_ptr: *const u8,
            data_len: usize,
            out_written: *mut usize,
        ) -> i32,
    >,
}

#[derive(Clone, Copy)]
pub struct CallbackVolumeSource {
    callbacks: XunbakVolumeCallbacks,
}

impl FsVolumeSource {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl VolumeSource for FsVolumeSource {
    fn open(&self, volume_name: &str) -> Result<Box<dyn ReadSeek>, CoreError> {
        let path = self.root.join(volume_name);
        let file = File::open(&path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                CoreError::MissingSplitVolume(path.display().to_string())
            } else {
                CoreError::Io(err.to_string())
            }
        })?;
        Ok(Box::new(file))
    }
}

impl MemoryVolumeSource {
    pub fn new(primary_name: impl Into<String>, bytes: Arc<[u8]>) -> Self {
        let primary_name = primary_name.into();
        let mut volumes = HashMap::new();
        volumes.insert(primary_name, bytes);
        Self { volumes }
    }

    fn from_volume_map(volumes: HashMap<String, Arc<[u8]>>) -> Self {
        Self { volumes }
    }
}

impl VolumeSource for MemoryVolumeSource {
    fn open(&self, volume_name: &str) -> Result<Box<dyn ReadSeek>, CoreError> {
        let bytes = self
            .volumes
            .get(volume_name)
            .ok_or_else(|| CoreError::MissingSplitVolume(volume_name.to_string()))?;
        Ok(Box::new(std::io::Cursor::new(bytes.clone())))
    }
}

impl CallbackVolumeSource {
    pub fn new(callbacks: XunbakVolumeCallbacks) -> Result<Self, CoreError> {
        if callbacks.open_volume.is_none()
            || callbacks.read.is_none()
            || callbacks.seek.is_none()
            || callbacks.close_volume.is_none()
        {
            return Err(CoreError::InvalidSplitState(
                "callback vtable is incomplete".to_string(),
            ));
        }
        Ok(Self { callbacks })
    }
}

impl VolumeSource for CallbackVolumeSource {
    fn open(&self, volume_name: &str) -> Result<Box<dyn ReadSeek>, CoreError> {
        let open_volume = self.callbacks.open_volume.ok_or_else(|| {
            CoreError::InvalidSplitState("open_volume callback missing".to_string())
        })?;
        let mut handle = std::ptr::null_mut();
        let rc = unsafe {
            open_volume(
                self.callbacks.ctx,
                volume_name.as_ptr(),
                volume_name.len(),
                &mut handle,
            )
        };
        if rc != XUNBAK_OK {
            if rc == XUNBAK_ERR_OPEN {
                return Err(CoreError::MissingSplitVolume(volume_name.to_string()));
            }
            return Err(CoreError::Io(format!(
                "open_volume callback failed for {volume_name}: {rc}"
            )));
        }
        if handle.is_null() {
            return Err(CoreError::Io(format!(
                "open_volume callback returned null handle for {volume_name}"
            )));
        }
        Ok(Box::new(CallbackStream {
            callbacks: self.callbacks,
            handle,
            buffer: vec![0u8; 64 * 1024],
            buffer_pos: 0,
            buffer_len: 0,
        }))
    }
}

#[derive(Clone, Debug)]
pub struct XunbakArchive<S> {
    source: S,
    items: Vec<ArchiveItem>,
    volume_names: Vec<String>,
    is_split: bool,
    manifest: ManifestBody,
    #[allow(dead_code)]
    checkpoint: CheckpointPayload,
    #[allow(dead_code)]
    header: DecodedHeader,
}

impl XunbakArchive<FsVolumeSource> {
    pub fn open_path(path: &Path) -> Result<Self, CoreError> {
        let primary_path = resolve_primary_path(path)?;
        let root = primary_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let primary_name = primary_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| CoreError::PathHasNoFileName(primary_path.display().to_string()))?;
        Self::open_with_source(FsVolumeSource::new(root), primary_name)
    }
}

impl XunbakArchive<MemoryVolumeSource> {
    pub fn open_bytes(bytes: &[u8]) -> Result<Self, CoreError> {
        let bytes_arc = Arc::<[u8]>::from(bytes.to_vec());
        if let Some(volumes) =
            inline_split_volumes_from_concatenated_bytes("memory.xunbak", &bytes_arc)?
        {
            return Self::open_with_source(
                MemoryVolumeSource::from_volume_map(volumes),
                "memory.xunbak.001",
            );
        }
        Self::open_with_source(
            MemoryVolumeSource::new("memory.xunbak", bytes_arc),
            "memory.xunbak",
        )
    }
}

impl<S: VolumeSource> XunbakArchive<S> {
    pub fn open_with_source(source: S, primary_name: impl Into<String>) -> Result<Self, CoreError> {
        let primary_name = primary_name.into();
        let (header, primary_size) = read_header(&source, &primary_name)?;
        if primary_size < (HEADER_SIZE + FOOTER_SIZE) as u64 && !is_split_member_name(&primary_name)
        {
            return Err(CoreError::ContainerTooSmall {
                actual: primary_size,
            });
        }

        let mut volume_names = if header.header.flags & FLAG_SPLIT != 0 {
            discover_split_volume_names(&source, &primary_name, &header)?
        } else {
            vec![primary_name.clone()]
        };

        let mut last_size = stream_len(
            &mut *source.open(
                volume_names
                    .last()
                    .expect("volume_names always has at least one element"),
            )?,
        )?;
        let footer = read_footer(&source, volume_names.last().expect("checked"), last_size)?;
        let checkpoint = read_checkpoint(&source, volume_names.last().expect("checked"), &footer)?;

        if header.header.flags & FLAG_SPLIT != 0 {
            let expected = checkpoint.total_volumes as usize;
            if expected == 0 || expected > volume_names.len() {
                return Err(CoreError::InvalidSplitState(format!(
                    "checkpoint expects {expected} volume(s), discovered {}",
                    volume_names.len()
                )));
            }
            volume_names.truncate(expected);
            last_size = stream_len(
                &mut *source.open(
                    volume_names
                        .last()
                        .expect("volume_names always has at least one element"),
                )?,
            )?;
        }

        let manifest = load_manifest(
            &source,
            volume_names.last().expect("checked"),
            &checkpoint,
            last_size,
        )?;
        let items = build_items(&source, &volume_names, &manifest)?;

        Ok(Self {
            source,
            items,
            volume_names,
            is_split: header.header.flags & FLAG_SPLIT != 0,
            manifest,
            checkpoint,
            header,
        })
    }

    pub fn is_split(&self) -> bool {
        self.is_split
    }

    pub fn volume_count(&self) -> usize {
        self.volume_names.len()
    }

    pub fn items(&self) -> &[ArchiveItem] {
        &self.items
    }

    pub fn extract_item_to_writer<W: Write>(
        &self,
        path: &str,
        writer: &mut W,
    ) -> Result<(), CoreError> {
        let entry = self
            .manifest
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .ok_or_else(|| CoreError::MissingItem(path.to_string()))?;

        let mut hashing_writer = HashingWriter::new(writer);
        if let Some(parts) = &entry.parts {
            for part in parts {
                let volume_name = self.volume_name(part.volume_index)?;
                let mut reader = self.source.open(volume_name)?;
                reader
                    .seek(SeekFrom::Start(part.blob_offset))
                    .map_err(|err| CoreError::Io(err.to_string()))?;
                let result = copy_blob_record_content_to_writer(&mut reader, &mut hashing_writer)?;
                if result.header.blob_id != part.blob_id {
                    return Err(CoreError::Blob(BlobRecordError::BlobHashMismatch));
                }
            }
            if hashing_writer.finalize() != entry.content_hash {
                return Err(CoreError::Blob(BlobRecordError::BlobHashMismatch));
            }
            return Ok(());
        }

        let volume_name = self.volume_name(entry.volume_index)?;
        let mut reader = self.source.open(volume_name)?;
        reader
            .seek(SeekFrom::Start(entry.blob_offset))
            .map_err(|err| CoreError::Io(err.to_string()))?;
        let result = copy_blob_record_content_to_writer(&mut reader, &mut hashing_writer)?;
        if result.header.blob_id != entry.blob_id || hashing_writer.finalize() != entry.content_hash
        {
            return Err(CoreError::Blob(BlobRecordError::BlobHashMismatch));
        }
        Ok(())
    }

    fn volume_name(&self, volume_index: u16) -> Result<&str, CoreError> {
        self.volume_names
            .get(volume_index as usize)
            .map(|name| name.as_str())
            .ok_or(CoreError::VolumeIndexOutOfRange {
                requested: volume_index,
                available: self.volume_names.len(),
            })
    }
}

enum ArchiveHandleInner {
    Memory(XunbakArchive<MemoryVolumeSource>),
    Callback(XunbakArchive<CallbackVolumeSource>),
}

impl ArchiveHandleInner {
    fn items(&self) -> &[ArchiveItem] {
        match self {
            Self::Memory(archive) => archive.items(),
            Self::Callback(archive) => archive.items(),
        }
    }

    fn extract_item_to_writer<W: Write>(
        &self,
        path: &str,
        writer: &mut W,
    ) -> Result<(), CoreError> {
        match self {
            Self::Memory(archive) => archive.extract_item_to_writer(path, writer),
            Self::Callback(archive) => archive.extract_item_to_writer(path, writer),
        }
    }

    fn volume_count(&self) -> usize {
        match self {
            Self::Memory(archive) => archive.volume_count(),
            Self::Callback(archive) => archive.volume_count(),
        }
    }
}

pub struct XunbakArchiveHandle {
    archive: ArchiveHandleInner,
}

pub const XUNBAK_OK: i32 = 0;
pub const XUNBAK_ERR_INVALID_ARG: i32 = 1;
pub const XUNBAK_ERR_OPEN: i32 = 2;
pub const XUNBAK_ERR_RANGE: i32 = 3;
pub const XUNBAK_ERR_BUFFER_TOO_SMALL: i32 = 4;
pub const XUNBAK_ERR_IO: i32 = 5;

pub const XUNBAK_PROP_PATH: u32 = 0;
pub const XUNBAK_PROP_SIZE: u32 = 1;
pub const XUNBAK_PROP_PACKED_SIZE: u32 = 2;
pub const XUNBAK_PROP_MTIME_NS: u32 = 3;
pub const XUNBAK_PROP_CTIME_NS: u32 = 4;
pub const XUNBAK_PROP_WIN_ATTRIBUTES: u32 = 5;
pub const XUNBAK_PROP_CODEC_ID: u32 = 6;

#[unsafe(no_mangle)]
pub extern "C" fn xunbak_open(
    data: *const u8,
    len: usize,
    out: *mut *mut XunbakArchiveHandle,
) -> i32 {
    if out.is_null() || (data.is_null() && len != 0) {
        return XUNBAK_ERR_INVALID_ARG;
    }
    let bytes = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    };
    match XunbakArchive::open_bytes(bytes) {
        Ok(archive) => {
            unsafe {
                *out = Box::into_raw(Box::new(XunbakArchiveHandle {
                    archive: ArchiveHandleInner::Memory(archive),
                }));
            }
            XUNBAK_OK
        }
        Err(err) => map_core_error(&err),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn xunbak_open_with_callbacks(
    primary_name_ptr: *const u8,
    primary_name_len: usize,
    callbacks: *const XunbakVolumeCallbacks,
    out: *mut *mut XunbakArchiveHandle,
) -> i32 {
    if primary_name_ptr.is_null() || callbacks.is_null() || out.is_null() {
        return XUNBAK_ERR_INVALID_ARG;
    }
    let primary_name = match std::str::from_utf8(unsafe {
        std::slice::from_raw_parts(primary_name_ptr, primary_name_len)
    }) {
        Ok(value) if !value.is_empty() => value,
        _ => return XUNBAK_ERR_INVALID_ARG,
    };
    let callbacks = unsafe { *callbacks };
    let source = match CallbackVolumeSource::new(callbacks) {
        Ok(source) => source,
        Err(err) => return map_core_error(&err),
    };
    match XunbakArchive::open_with_source(source, primary_name) {
        Ok(archive) => {
            unsafe {
                *out = Box::into_raw(Box::new(XunbakArchiveHandle {
                    archive: ArchiveHandleInner::Callback(archive),
                }));
            }
            XUNBAK_OK
        }
        Err(err) => map_core_error(&err),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn xunbak_close(handle: *mut XunbakArchiveHandle) {
    if handle.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(handle));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn xunbak_item_count(handle: *const XunbakArchiveHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    unsafe { (*handle).archive.items().len() as u32 }
}

#[unsafe(no_mangle)]
pub extern "C" fn xunbak_volume_count(handle: *const XunbakArchiveHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    unsafe { (*handle).archive.volume_count() as u32 }
}

#[unsafe(no_mangle)]
pub extern "C" fn xunbak_get_property(
    archive: *const XunbakArchiveHandle,
    index: u32,
    prop_id: u32,
    out_buf: *mut c_void,
    buf_len: usize,
    out_written: *mut usize,
) -> i32 {
    let item = match item_at(archive, index) {
        Ok(item) => item,
        Err(code) => return code,
    };
    match prop_id {
        XUNBAK_PROP_PATH => {
            let utf16: Vec<u16> = item.path.encode_utf16().collect();
            let bytes_len = utf16.len() * std::mem::size_of::<u16>();
            if !out_written.is_null() {
                unsafe { *out_written = bytes_len };
            }
            if bytes_len == 0 {
                return XUNBAK_OK;
            }
            if out_buf.is_null() || buf_len < bytes_len {
                return XUNBAK_ERR_BUFFER_TOO_SMALL;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(
                    utf16.as_ptr().cast::<u8>(),
                    out_buf.cast::<u8>(),
                    bytes_len,
                );
            }
            XUNBAK_OK
        }
        XUNBAK_PROP_SIZE => write_scalar_property(item.size, out_buf, buf_len, out_written),
        XUNBAK_PROP_PACKED_SIZE => {
            write_scalar_property(item.packed_size, out_buf, buf_len, out_written)
        }
        XUNBAK_PROP_MTIME_NS => write_scalar_property(item.mtime_ns, out_buf, buf_len, out_written),
        XUNBAK_PROP_CTIME_NS => {
            write_scalar_property(item.created_time_ns, out_buf, buf_len, out_written)
        }
        XUNBAK_PROP_WIN_ATTRIBUTES => {
            write_scalar_property(item.win_attributes, out_buf, buf_len, out_written)
        }
        XUNBAK_PROP_CODEC_ID => write_scalar_property(item.codec_id, out_buf, buf_len, out_written),
        _ => XUNBAK_ERR_INVALID_ARG,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn xunbak_extract(
    archive: *const XunbakArchiveHandle,
    index: u32,
    out_buf: *mut u8,
    buf_len: usize,
    out_written: *mut usize,
) -> i32 {
    let handle = match archive_ref(archive) {
        Ok(handle) => handle,
        Err(code) => return code,
    };
    let item = match item_at(archive, index) {
        Ok(item) => item,
        Err(code) => return code,
    };

    let mut content = Vec::new();
    if let Err(err) = handle
        .archive
        .extract_item_to_writer(&item.path, &mut content)
    {
        return map_core_error(&err);
    }
    if !out_written.is_null() {
        unsafe { *out_written = content.len() };
    }
    if out_buf.is_null() || buf_len < content.len() {
        return XUNBAK_ERR_BUFFER_TOO_SMALL;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(content.as_ptr(), out_buf, content.len());
    }
    XUNBAK_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn xunbak_extract_with_writer(
    archive: *const XunbakArchiveHandle,
    index: u32,
    callbacks: *const XunbakWriteCallbacks,
    out_written: *mut usize,
) -> i32 {
    let handle = match archive_ref(archive) {
        Ok(handle) => handle,
        Err(code) => return code,
    };
    let item = match item_at(archive, index) {
        Ok(item) => item,
        Err(code) => return code,
    };
    if callbacks.is_null() {
        return XUNBAK_ERR_INVALID_ARG;
    }
    let callbacks = unsafe { *callbacks };
    let mut writer = match CallbackWriter::new(callbacks) {
        Ok(writer) => writer,
        Err(code) => return code,
    };
    if let Err(err) = handle
        .archive
        .extract_item_to_writer(&item.path, &mut writer)
    {
        return map_core_error(&err);
    }
    if !out_written.is_null() {
        unsafe {
            *out_written = writer.total_written;
        }
    }
    XUNBAK_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn xunbak_item_size(
    archive: *const XunbakArchiveHandle,
    index: u32,
    out_size: *mut u64,
) -> i32 {
    if out_size.is_null() {
        return XUNBAK_ERR_INVALID_ARG;
    }
    let item = match item_at(archive, index) {
        Ok(item) => item,
        Err(code) => return code,
    };
    unsafe {
        *out_size = item.size;
    }
    XUNBAK_OK
}

fn archive_ref<'a>(archive: *const XunbakArchiveHandle) -> Result<&'a XunbakArchiveHandle, i32> {
    if archive.is_null() {
        return Err(XUNBAK_ERR_INVALID_ARG);
    }
    Ok(unsafe { &*archive })
}

fn item_at(archive: *const XunbakArchiveHandle, index: u32) -> Result<ArchiveItem, i32> {
    let archive = archive_ref(archive)?;
    archive
        .archive
        .items()
        .get(index as usize)
        .cloned()
        .ok_or(XUNBAK_ERR_RANGE)
}

fn write_scalar_property<T: Copy>(
    value: T,
    out_buf: *mut c_void,
    buf_len: usize,
    out_written: *mut usize,
) -> i32 {
    let value_size = std::mem::size_of::<T>();
    if !out_written.is_null() {
        unsafe { *out_written = value_size };
    }
    if out_buf.is_null() || buf_len < value_size {
        return XUNBAK_ERR_BUFFER_TOO_SMALL;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(
            (&value as *const T).cast::<u8>(),
            out_buf.cast::<u8>(),
            value_size,
        );
    }
    XUNBAK_OK
}

fn map_core_error(err: &CoreError) -> i32 {
    match err {
        CoreError::ContainerNotFound(_)
        | CoreError::ContainerTooSmall { .. }
        | CoreError::Header(_)
        | CoreError::Footer(_)
        | CoreError::Checkpoint(_)
        | CoreError::Manifest(_)
        | CoreError::ManifestHashMismatch
        | CoreError::InvalidSplitState(_)
        | CoreError::MissingSplitVolume(_) => XUNBAK_ERR_OPEN,
        CoreError::MissingItem(_) | CoreError::VolumeIndexOutOfRange { .. } => XUNBAK_ERR_RANGE,
        CoreError::Io(_) | CoreError::Blob(_) | CoreError::SeekOverflow(_) => XUNBAK_ERR_IO,
        CoreError::PathHasNoFileName(_) => XUNBAK_ERR_INVALID_ARG,
    }
}

struct CallbackWriter {
    callbacks: XunbakWriteCallbacks,
    total_written: usize,
}

impl CallbackWriter {
    fn new(callbacks: XunbakWriteCallbacks) -> Result<Self, i32> {
        if callbacks.write.is_none() {
            return Err(XUNBAK_ERR_INVALID_ARG);
        }
        Ok(Self {
            callbacks,
            total_written: 0,
        })
    }
}

impl Write for CallbackWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let write = self.callbacks.write.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "write callback missing")
        })?;
        let mut written_total = 0usize;
        while written_total < buf.len() {
            let mut written = 0usize;
            let rc = unsafe {
                write(
                    self.callbacks.ctx,
                    buf[written_total..].as_ptr(),
                    buf.len() - written_total,
                    &mut written,
                )
            };
            if rc != XUNBAK_OK {
                return Err(std::io::Error::other(format!(
                    "write callback failed: {rc}"
                )));
            }
            if written == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "write callback returned zero",
                ));
            }
            written_total += written;
            self.total_written += written;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct CallbackStream {
    callbacks: XunbakVolumeCallbacks,
    handle: *mut c_void,
    buffer: Vec<u8>,
    buffer_pos: usize,
    buffer_len: usize,
}

impl Drop for CallbackStream {
    fn drop(&mut self) {
        if let Some(close_volume) = self.callbacks.close_volume {
            unsafe {
                close_volume(self.callbacks.ctx, self.handle);
            }
        }
    }
}

impl Read for CallbackStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.buffer_pos < self.buffer_len {
            let available = self.buffer_len - self.buffer_pos;
            let to_copy = available.min(buf.len());
            buf[..to_copy]
                .copy_from_slice(&self.buffer[self.buffer_pos..self.buffer_pos + to_copy]);
            self.buffer_pos += to_copy;
            return Ok(to_copy);
        }

        let read = self
            .callbacks
            .read
            .ok_or_else(|| std::io::Error::other("read callback missing"))?;

        if buf.len() >= self.buffer.len() {
            let mut out_read = 0usize;
            let rc = unsafe {
                read(
                    self.callbacks.ctx,
                    self.handle,
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut out_read,
                )
            };
            if rc != XUNBAK_OK {
                return Err(std::io::Error::other(format!("read callback failed: {rc}")));
            }
            return Ok(out_read);
        }

        let mut out_read = 0usize;
        let rc = unsafe {
            read(
                self.callbacks.ctx,
                self.handle,
                self.buffer.as_mut_ptr(),
                self.buffer.len(),
                &mut out_read,
            )
        };
        if rc != XUNBAK_OK {
            return Err(std::io::Error::other(format!("read callback failed: {rc}")));
        }
        self.buffer_pos = 0;
        self.buffer_len = out_read;
        let to_copy = self.buffer_len.min(buf.len());
        buf[..to_copy].copy_from_slice(&self.buffer[..to_copy]);
        self.buffer_pos = to_copy;
        Ok(to_copy)
    }
}

impl Seek for CallbackStream {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        const SEEK_FROM_START: u32 = 0;
        const SEEK_FROM_CURRENT: u32 = 1;
        const SEEK_FROM_END: u32 = 2;

        let seek = self
            .callbacks
            .seek
            .ok_or_else(|| std::io::Error::other("seek callback missing"))?;
        let (offset, origin) = match pos {
            SeekFrom::Start(value) => {
                let offset = i64::try_from(value).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek start too large")
                })?;
                (offset, SEEK_FROM_START)
            }
            SeekFrom::Current(value) => (value, SEEK_FROM_CURRENT),
            SeekFrom::End(value) => (value, SEEK_FROM_END),
        };
        let mut out_pos = 0u64;
        let rc = unsafe {
            seek(
                self.callbacks.ctx,
                self.handle,
                offset,
                origin,
                &mut out_pos,
            )
        };
        if rc != XUNBAK_OK {
            return Err(std::io::Error::other(format!("seek callback failed: {rc}")));
        }
        self.buffer_pos = 0;
        self.buffer_len = 0;
        Ok(out_pos)
    }
}

fn resolve_primary_path(path: &Path) -> Result<PathBuf, CoreError> {
    if path.exists() {
        return Ok(path.to_path_buf());
    }
    let split_first = PathBuf::from(format!("{}.001", path.display()));
    if split_first.exists() {
        return Ok(split_first);
    }
    Err(CoreError::ContainerNotFound(path.display().to_string()))
}

fn read_header<S: VolumeSource>(
    source: &S,
    volume_name: &str,
) -> Result<(DecodedHeader, u64), CoreError> {
    let mut reader = source.open(volume_name)?;
    let size = stream_len(&mut *reader)?;
    let mut header_bytes = [0u8; HEADER_SIZE];
    reader
        .read_exact(&mut header_bytes)
        .map_err(|err| CoreError::Io(err.to_string()))?;
    let header = Header::from_bytes(&header_bytes)?;
    Ok((header, size))
}

fn stream_len(reader: &mut dyn ReadSeek) -> Result<u64, CoreError> {
    let current = reader
        .stream_position()
        .map_err(|err| CoreError::Io(err.to_string()))?;
    let end = reader
        .seek(SeekFrom::End(0))
        .map_err(|err| CoreError::Io(err.to_string()))?;
    reader
        .seek(SeekFrom::Start(current))
        .map_err(|err| CoreError::Io(err.to_string()))?;
    Ok(end)
}

fn discover_split_volume_names<S: VolumeSource>(
    source: &S,
    primary_name: &str,
    header: &DecodedHeader,
) -> Result<Vec<String>, CoreError> {
    let split = header
        .header
        .split
        .ok_or_else(|| CoreError::InvalidSplitState(primary_name.to_string()))?;
    let base_name = split_base_name(primary_name);
    let mut volume_names = Vec::new();

    for index in 0..=u16::MAX {
        let volume_name = split_volume_name(&base_name, index);
        match source.open(&volume_name) {
            Ok(mut reader) => {
                let mut header_bytes = [0u8; HEADER_SIZE];
                reader
                    .read_exact(&mut header_bytes)
                    .map_err(|err| CoreError::Io(err.to_string()))?;
                let decoded = Header::from_bytes(&header_bytes)?;
                let decoded_split = decoded
                    .header
                    .split
                    .ok_or_else(|| CoreError::InvalidSplitState(volume_name.clone()))?;
                if decoded_split.volume_index != index || decoded_split.set_id != split.set_id {
                    return Err(CoreError::InvalidSplitState(volume_name));
                }
                volume_names.push(volume_name);
            }
            Err(CoreError::MissingSplitVolume(_)) => {
                if index == 0 {
                    return Err(CoreError::MissingSplitVolume(split_volume_name(
                        &base_name, index,
                    )));
                }
                break;
            }
            Err(err) => return Err(err),
        }
    }

    if volume_names.is_empty() {
        return Err(CoreError::MissingSplitVolume(primary_name.to_string()));
    }
    Ok(volume_names)
}

fn read_footer<S: VolumeSource>(
    source: &S,
    volume_name: &str,
    file_size: u64,
) -> Result<Footer, CoreError> {
    if file_size < FOOTER_SIZE as u64 {
        return Err(CoreError::ContainerTooSmall { actual: file_size });
    }
    let mut reader = source.open(volume_name)?;
    reader
        .seek(SeekFrom::Start(file_size - FOOTER_SIZE as u64))
        .map_err(|err| CoreError::Io(err.to_string()))?;
    let mut footer_bytes = [0u8; FOOTER_SIZE];
    reader
        .read_exact(&mut footer_bytes)
        .map_err(|err| CoreError::Io(err.to_string()))?;
    Ok(Footer::from_bytes(&footer_bytes, file_size)?)
}

fn read_checkpoint<S: VolumeSource>(
    source: &S,
    volume_name: &str,
    footer: &Footer,
) -> Result<CheckpointPayload, CoreError> {
    let mut reader = source.open(volume_name)?;
    reader
        .seek(SeekFrom::Start(footer.checkpoint_offset))
        .map_err(|err| CoreError::Io(err.to_string()))?;
    Ok(read_checkpoint_record(&mut reader)?.payload)
}

fn load_manifest<S: VolumeSource>(
    source: &S,
    volume_name: &str,
    checkpoint: &CheckpointPayload,
    volume_size: u64,
) -> Result<ManifestBody, CoreError> {
    if checkpoint.manifest_offset >= volume_size {
        return Err(CoreError::InvalidSplitState(format!(
            "manifest offset {} out of range for volume {volume_name}",
            checkpoint.manifest_offset
        )));
    }

    let mut reader = source.open(volume_name)?;
    reader
        .seek(SeekFrom::Start(checkpoint.manifest_offset))
        .map_err(|err| CoreError::Io(err.to_string()))?;
    let manifest = read_manifest_record(&mut reader)?.body;

    let payload_len = checkpoint
        .manifest_len
        .checked_sub(RECORD_PREFIX_SIZE as u64)
        .ok_or(CoreError::ManifestHashMismatch)?;
    let mut payload = vec![0u8; payload_len as usize];
    reader
        .seek(SeekFrom::Start(
            checkpoint.manifest_offset + RECORD_PREFIX_SIZE as u64,
        ))
        .map_err(|err| CoreError::Io(err.to_string()))?;
    reader
        .read_exact(&mut payload)
        .map_err(|err| CoreError::Io(err.to_string()))?;
    if compute_manifest_hash(&payload) != checkpoint.manifest_hash {
        return Err(CoreError::ManifestHashMismatch);
    }

    Ok(manifest)
}

fn build_items<S: VolumeSource>(
    source: &S,
    volume_names: &[String],
    manifest: &ManifestBody,
) -> Result<Vec<ArchiveItem>, CoreError> {
    manifest
        .entries
        .iter()
        .map(|entry| build_item(source, volume_names, entry))
        .collect()
}

fn build_item<S: VolumeSource>(
    source: &S,
    volume_names: &[String],
    entry: &ManifestEntry,
) -> Result<ArchiveItem, CoreError> {
    let packed_size = if let Some(parts) = &entry.parts {
        parts.iter().map(|part| part.stored_size).sum()
    } else {
        let volume_name = volume_names.get(entry.volume_index as usize).ok_or(
            CoreError::VolumeIndexOutOfRange {
                requested: entry.volume_index,
                available: volume_names.len(),
            },
        )?;
        read_blob_header(source, volume_name, entry.blob_offset)?.stored_size
    };

    Ok(ArchiveItem {
        path: entry.path.clone(),
        size: entry.size,
        packed_size,
        mtime_ns: entry.mtime_ns,
        created_time_ns: entry.created_time_ns,
        win_attributes: entry.win_attributes,
        volume_index: entry.volume_index,
        codec_id: entry.codec.as_u8() as u32,
    })
}

fn read_blob_header<S: VolumeSource>(
    source: &S,
    volume_name: &str,
    blob_offset: u64,
) -> Result<BlobHeader, CoreError> {
    let header_offset = blob_offset
        .checked_add(RECORD_PREFIX_SIZE as u64)
        .ok_or(CoreError::SeekOverflow(blob_offset))?;
    let mut reader = source.open(volume_name)?;
    reader
        .seek(SeekFrom::Start(header_offset))
        .map_err(|err| CoreError::Io(err.to_string()))?;
    let mut bytes = [0u8; BLOB_HEADER_SIZE];
    reader
        .read_exact(&mut bytes)
        .map_err(|err| CoreError::Io(err.to_string()))?;
    BlobHeader::from_bytes(&bytes).map_err(|err| CoreError::Blob(BlobRecordError::Header(err)))
}

fn is_split_member_name(name: &str) -> bool {
    name.len() >= 4
        && name.as_bytes()[name.len() - 4] == b'.'
        && name[name.len() - 3..].chars().all(|ch| ch.is_ascii_digit())
}

fn split_base_name(name: &str) -> String {
    if is_split_member_name(name) {
        name[..name.len() - 4].to_string()
    } else {
        name.to_string()
    }
}

fn split_volume_name(base_name: &str, volume_index: u16) -> String {
    format!("{base_name}.{:03}", volume_index + 1)
}

fn inline_split_volumes_from_concatenated_bytes(
    base_name: &str,
    bytes: &Arc<[u8]>,
) -> Result<Option<HashMap<String, Arc<[u8]>>>, CoreError> {
    if bytes.len() < HEADER_SIZE {
        return Ok(None);
    }
    let header = match Header::from_bytes(&bytes[..HEADER_SIZE]) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    if header.header.flags & FLAG_SPLIT == 0 {
        return Ok(None);
    }
    let split = header
        .header
        .split
        .ok_or_else(|| CoreError::InvalidSplitState(base_name.to_string()))?;
    let mut start = 0usize;
    let mut expected_index = 0u16;
    let mut volumes = HashMap::new();

    loop {
        if start + HEADER_SIZE > bytes.len() {
            return Ok(None);
        }
        let current = match Header::from_bytes(&bytes[start..start + HEADER_SIZE]) {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let current_split = match current.header.split {
            Some(value)
                if current.header.flags & FLAG_SPLIT != 0
                    && value.volume_index == expected_index
                    && value.set_id == split.set_id =>
            {
                value
            }
            _ => return Ok(None),
        };

        let mut offset = start + HEADER_SIZE;
        loop {
            if offset + FOOTER_SIZE <= bytes.len() {
                let candidate_volume_len = (offset + FOOTER_SIZE - start) as u64;
                if let Ok(footer) =
                    Footer::from_bytes(&bytes[offset..offset + FOOTER_SIZE], candidate_volume_len)
                {
                    let checkpoint_start = start + footer.checkpoint_offset as usize;
                    if checkpoint_start < offset + FOOTER_SIZE {
                        let mut cursor = std::io::Cursor::new(&bytes[checkpoint_start..offset]);
                        if let Ok(checkpoint) = read_checkpoint_record(&mut cursor)
                            && checkpoint.payload.total_volumes == expected_index + 1
                        {
                            volumes.insert(
                                split_volume_name(base_name, current_split.volume_index),
                                Arc::<[u8]>::from(bytes[start..offset + FOOTER_SIZE].to_vec()),
                            );
                            return Ok(Some(volumes));
                        }
                    }
                }
            }

            if offset + HEADER_SIZE <= bytes.len()
                && let Ok(next_header) = Header::from_bytes(&bytes[offset..offset + HEADER_SIZE])
                && let Some(next_split) = next_header.header.split
                && next_header.header.flags & FLAG_SPLIT != 0
                && next_split.set_id == split.set_id
                && next_split.volume_index == expected_index + 1
            {
                volumes.insert(
                    split_volume_name(base_name, current_split.volume_index),
                    Arc::<[u8]>::from(bytes[start..offset].to_vec()),
                );
                start = offset;
                expected_index += 1;
                break;
            }

            if offset + RECORD_PREFIX_SIZE > bytes.len() {
                return Ok(None);
            }
            let prefix = match xun::xunbak::record::RecordPrefix::from_bytes(
                &bytes[offset..offset + RECORD_PREFIX_SIZE],
            ) {
                Ok(value) => value,
                Err(_) => return Ok(None),
            };
            let payload_start = offset + RECORD_PREFIX_SIZE;
            let payload_end = payload_start.saturating_add(prefix.record_len as usize);
            if payload_end > bytes.len() {
                return Ok(None);
            }
            let payload = &bytes[payload_start..payload_end];
            let payload_for_crc: &[u8] = if prefix.record_type
                == xun::xunbak::constants::RecordType::BLOB
                && payload.len() >= BLOB_HEADER_SIZE
            {
                &payload[..BLOB_HEADER_SIZE]
            } else {
                payload
            };
            let crc = xun::xunbak::record::compute_record_crc(
                prefix.record_type,
                prefix.record_len.to_le_bytes(),
                payload_for_crc,
            );
            if crc != prefix.record_crc {
                return Ok(None);
            }
            offset = payload_end;
        }
    }
}

struct HashingWriter<'a, W> {
    inner: &'a mut W,
    hasher: blake3::Hasher,
}

impl<'a, W> HashingWriter<'a, W> {
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

impl<W: Write> Write for HashingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write_all(buf)?;
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::io::{Cursor, Read as _};
    use std::mem::size_of;
    use std::os::raw::c_void;

    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tempfile::tempdir;
    use xun::xunbak::constants::Codec;
    use xun::xunbak::writer::{BackupOptions, ContainerWriter};

    use super::{
        CallbackVolumeSource, VolumeSource, XUNBAK_OK, XUNBAK_PROP_PATH, XunbakArchive,
        XunbakVolumeCallbacks, XunbakWriteCallbacks, xunbak_close, xunbak_extract,
        xunbak_extract_with_writer, xunbak_get_property, xunbak_item_count, xunbak_item_size,
        xunbak_open, xunbak_open_with_callbacks, xunbak_volume_count,
    };

    fn split_options() -> BackupOptions {
        BackupOptions {
            codec: Codec::NONE,
            zstd_level: 1,
            split_size: Some(1900),
        }
    }

    #[test]
    fn open_single_container_indexes_items_and_extracts_content() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("a.txt"), "alpha").unwrap();
        fs::write(source.join("nested").join("b.txt"), "bravo-bravo").unwrap();
        let container = dir.path().join("sample.xunbak");

        ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

        let archive = XunbakArchive::open_path(&container).unwrap();
        assert!(!archive.is_split());
        assert_eq!(archive.volume_count(), 1);

        let mut paths: Vec<&str> = archive
            .items()
            .iter()
            .map(|item| item.path.as_str())
            .collect();
        paths.sort_unstable();
        assert_eq!(paths, vec!["a.txt", "nested/b.txt"]);

        let mut restored = Vec::new();
        archive
            .extract_item_to_writer("nested/b.txt", &mut restored)
            .unwrap();
        assert_eq!(restored, b"bravo-bravo");
    }

    #[test]
    fn open_split_container_from_base_path_discovers_all_volumes_and_extracts() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "a".repeat(80)).unwrap();
        fs::write(source.join("b.txt"), "b".repeat(80)).unwrap();
        fs::write(source.join("c.txt"), "c".repeat(80)).unwrap();
        let base = dir.path().join("sample.xunbak");

        ContainerWriter::backup(&base, &source, &split_options()).unwrap();

        let archive = XunbakArchive::open_path(&base).unwrap();
        assert!(archive.is_split());
        assert!(archive.volume_count() >= 2);

        let mut restored = Vec::new();
        archive
            .extract_item_to_writer("c.txt", &mut restored)
            .unwrap();
        assert_eq!(restored, "c".repeat(80).into_bytes());
    }

    #[test]
    fn open_split_container_from_first_volume_path_also_works() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "a".repeat(80)).unwrap();
        fs::write(source.join("b.txt"), "b".repeat(80)).unwrap();
        fs::write(source.join("c.txt"), "c".repeat(80)).unwrap();
        let base = dir.path().join("sample.xunbak");

        ContainerWriter::backup(&base, &source, &split_options()).unwrap();

        let archive = XunbakArchive::open_path(&dir.path().join("sample.xunbak.001")).unwrap();
        assert!(archive.is_split());
        assert!(archive.volume_count() >= 2);
        assert_eq!(archive.items().len(), 3);
    }

    #[test]
    fn open_bytes_indexes_single_file_container() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("only.txt"), "hello-from-bytes").unwrap();
        let container = dir.path().join("sample.xunbak");

        ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
        let bytes = fs::read(&container).unwrap();

        let archive = XunbakArchive::open_bytes(&bytes).unwrap();
        assert!(!archive.is_split());
        assert_eq!(archive.items().len(), 1);

        let mut restored = Vec::new();
        archive
            .extract_item_to_writer("only.txt", &mut restored)
            .unwrap();
        assert_eq!(restored, b"hello-from-bytes");
    }

    #[test]
    fn ffi_open_count_property_and_extract_work_for_single_file_container() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("ffi.txt"), "ffi-roundtrip").unwrap();
        let container = dir.path().join("sample.xunbak");

        ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
        let bytes = fs::read(&container).unwrap();

        let mut handle = std::ptr::null_mut();
        assert_eq!(xunbak_open(bytes.as_ptr(), bytes.len(), &mut handle), 0);
        assert!(!handle.is_null());
        assert_eq!(xunbak_item_count(handle), 1);
        assert_eq!(xunbak_volume_count(handle), 1);

        let mut size = 0u64;
        assert_eq!(xunbak_item_size(handle, 0, &mut size), 0);
        assert_eq!(size, "ffi-roundtrip".len() as u64);

        let mut path_buf = [0u16; 64];
        let mut written = 0usize;
        assert_eq!(
            xunbak_get_property(
                handle,
                XUNBAK_PROP_PATH,
                0,
                path_buf.as_mut_ptr().cast(),
                path_buf.len() * size_of::<u16>(),
                &mut written,
            ),
            0
        );
        let path_units = written / size_of::<u16>();
        let path = String::from_utf16(&path_buf[..path_units]).unwrap();
        assert_eq!(path, "ffi.txt");

        let mut out = vec![0u8; size as usize];
        let mut out_written = 0usize;
        assert_eq!(
            xunbak_extract(handle, 0, out.as_mut_ptr(), out.len(), &mut out_written),
            0
        );
        assert_eq!(out_written, out.len());
        assert_eq!(out, b"ffi-roundtrip");

        let mut streamed = Vec::new();
        let callbacks = XunbakWriteCallbacks {
            ctx: (&mut streamed) as *mut Vec<u8> as *mut c_void,
            write: Some(collect_write_callback),
        };
        let mut streamed_written = 0usize;
        assert_eq!(
            xunbak_extract_with_writer(handle, 0, &callbacks, &mut streamed_written),
            0
        );
        assert_eq!(streamed_written, streamed.len());
        assert_eq!(streamed, b"ffi-roundtrip");

        xunbak_close(handle);
    }

    struct CallbackTestContext {
        volumes: HashMap<String, Vec<u8>>,
    }

    unsafe extern "C" fn callback_open_volume(
        ctx: *mut c_void,
        volume_name_ptr: *const u8,
        volume_name_len: usize,
        out_handle: *mut *mut c_void,
    ) -> i32 {
        if ctx.is_null() || volume_name_ptr.is_null() || out_handle.is_null() {
            return 1;
        }
        let ctx = unsafe { &mut *(ctx as *mut CallbackTestContext) };
        let name = match std::str::from_utf8(unsafe {
            std::slice::from_raw_parts(volume_name_ptr, volume_name_len)
        }) {
            Ok(value) => value,
            Err(_) => return 1,
        };
        let Some(bytes) = ctx.volumes.get(name) else {
            return 2;
        };
        let cursor = Box::new(Cursor::new(bytes.clone()));
        unsafe {
            *out_handle = Box::into_raw(cursor) as *mut c_void;
        }
        XUNBAK_OK
    }

    unsafe extern "C" fn callback_read(
        _ctx: *mut c_void,
        stream_handle: *mut c_void,
        out_buf: *mut u8,
        buf_len: usize,
        out_read: *mut usize,
    ) -> i32 {
        if stream_handle.is_null() || out_buf.is_null() || out_read.is_null() {
            return 1;
        }
        let cursor = unsafe { &mut *(stream_handle as *mut Cursor<Vec<u8>>) };
        let buf = unsafe { std::slice::from_raw_parts_mut(out_buf, buf_len) };
        match cursor.read(buf) {
            Ok(read) => {
                unsafe { *out_read = read };
                XUNBAK_OK
            }
            Err(_) => 5,
        }
    }

    unsafe extern "C" fn callback_seek(
        _ctx: *mut c_void,
        stream_handle: *mut c_void,
        offset: i64,
        origin: u32,
        out_pos: *mut u64,
    ) -> i32 {
        if stream_handle.is_null() || out_pos.is_null() {
            return 1;
        }
        let cursor = unsafe { &mut *(stream_handle as *mut Cursor<Vec<u8>>) };
        let seek_from = match origin {
            0 => std::io::SeekFrom::Start(offset as u64),
            1 => std::io::SeekFrom::Current(offset),
            2 => std::io::SeekFrom::End(offset),
            _ => return 1,
        };
        match std::io::Seek::seek(cursor, seek_from) {
            Ok(pos) => {
                unsafe { *out_pos = pos };
                XUNBAK_OK
            }
            Err(_) => 5,
        }
    }

    unsafe extern "C" fn callback_close_volume(_ctx: *mut c_void, stream_handle: *mut c_void) {
        if stream_handle.is_null() {
            return;
        }
        unsafe {
            drop(Box::from_raw(stream_handle as *mut Cursor<Vec<u8>>));
        }
    }

    unsafe extern "C" fn collect_write_callback(
        ctx: *mut c_void,
        data_ptr: *const u8,
        data_len: usize,
        out_written: *mut usize,
    ) -> i32 {
        if ctx.is_null() || data_ptr.is_null() || out_written.is_null() {
            return 1;
        }
        let out = unsafe { &mut *(ctx as *mut Vec<u8>) };
        let data = unsafe { std::slice::from_raw_parts(data_ptr, data_len) };
        out.extend_from_slice(data);
        unsafe {
            *out_written = data_len;
        }
        XUNBAK_OK
    }

    struct CountingReadContext {
        bytes: Vec<u8>,
        read_calls: Arc<AtomicUsize>,
    }

    unsafe extern "C" fn counting_open_volume(
        ctx: *mut c_void,
        _volume_name_ptr: *const u8,
        _volume_name_len: usize,
        out_handle: *mut *mut c_void,
    ) -> i32 {
        if ctx.is_null() || out_handle.is_null() {
            return 1;
        }
        let ctx = unsafe { &*(ctx as *const CountingReadContext) };
        let cursor = Box::new(Cursor::new(ctx.bytes.clone()));
        unsafe {
            *out_handle = Box::into_raw(cursor) as *mut c_void;
        }
        XUNBAK_OK
    }

    unsafe extern "C" fn counting_read(
        ctx: *mut c_void,
        stream_handle: *mut c_void,
        out_buf: *mut u8,
        buf_len: usize,
        out_read: *mut usize,
    ) -> i32 {
        if ctx.is_null() || stream_handle.is_null() || out_buf.is_null() || out_read.is_null() {
            return 1;
        }
        let ctx = unsafe { &*(ctx as *const CountingReadContext) };
        ctx.read_calls.fetch_add(1, Ordering::SeqCst);
        let cursor = unsafe { &mut *(stream_handle as *mut Cursor<Vec<u8>>) };
        let buf = unsafe { std::slice::from_raw_parts_mut(out_buf, buf_len) };
        match cursor.read(buf) {
            Ok(read) => {
                unsafe {
                    *out_read = read;
                }
                XUNBAK_OK
            }
            Err(_) => 5,
        }
    }

    #[test]
    fn ffi_open_with_callbacks_extracts_from_split_container() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "a".repeat(80)).unwrap();
        fs::write(source.join("b.txt"), "b".repeat(80)).unwrap();
        fs::write(source.join("c.txt"), "c".repeat(80)).unwrap();
        let base = dir.path().join("sample.xunbak");

        ContainerWriter::backup(&base, &source, &split_options()).unwrap();

        let mut volumes = HashMap::new();
        for entry in fs::read_dir(dir.path()).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("sample.xunbak.") {
                volumes.insert(name, fs::read(entry.path()).unwrap());
            }
        }
        let mut ctx = Box::new(CallbackTestContext { volumes });
        let callbacks = XunbakVolumeCallbacks {
            ctx: (&mut *ctx) as *mut CallbackTestContext as *mut c_void,
            open_volume: Some(callback_open_volume),
            read: Some(callback_read),
            seek: Some(callback_seek),
            close_volume: Some(callback_close_volume),
        };

        let mut handle = std::ptr::null_mut();
        assert_eq!(
            xunbak_open_with_callbacks(
                "sample.xunbak.001".as_ptr(),
                "sample.xunbak.001".len(),
                &callbacks,
                &mut handle
            ),
            0
        );
        assert!(!handle.is_null());
        assert_eq!(xunbak_item_count(handle), 3);
        assert_eq!(xunbak_volume_count(handle), 2);

        let mut size = 0u64;
        assert_eq!(xunbak_item_size(handle, 2, &mut size), 0);
        let mut out = vec![0u8; size as usize];
        let mut out_written = 0usize;
        assert_eq!(
            xunbak_extract(handle, 2, out.as_mut_ptr(), out.len(), &mut out_written),
            0
        );
        assert_eq!(out_written, out.len());
        assert_eq!(out, "c".repeat(80).into_bytes());

        let mut streamed = Vec::new();
        let callbacks = XunbakWriteCallbacks {
            ctx: (&mut streamed) as *mut Vec<u8> as *mut c_void,
            write: Some(collect_write_callback),
        };
        let mut streamed_written = 0usize;
        assert_eq!(
            xunbak_extract_with_writer(handle, 2, &callbacks, &mut streamed_written),
            0
        );
        assert_eq!(streamed_written, streamed.len());
        assert_eq!(streamed, "c".repeat(80).into_bytes());

        xunbak_close(handle);
        drop(ctx);
    }

    #[test]
    fn open_bytes_supports_concatenated_split_stream() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "a".repeat(80)).unwrap();
        fs::write(source.join("b.txt"), "b".repeat(80)).unwrap();
        fs::write(source.join("c.txt"), "c".repeat(80)).unwrap();
        let base = dir.path().join("sample.xunbak");

        ContainerWriter::backup(&base, &source, &split_options()).unwrap();
        let combined = [
            fs::read(dir.path().join("sample.xunbak.001")).unwrap(),
            fs::read(dir.path().join("sample.xunbak.002")).unwrap(),
        ]
        .concat();

        let archive = XunbakArchive::open_bytes(&combined).unwrap();
        assert!(archive.is_split());
        assert_eq!(archive.items().len(), 3);

        let mut restored = Vec::new();
        archive
            .extract_item_to_writer("c.txt", &mut restored)
            .unwrap();
        assert_eq!(restored, "c".repeat(80).into_bytes());
    }

    #[test]
    fn callback_stream_buffers_small_reads() {
        let read_calls = Arc::new(AtomicUsize::new(0));
        let mut ctx = Box::new(CountingReadContext {
            bytes: vec![7u8; 1024],
            read_calls: read_calls.clone(),
        });
        let callbacks = XunbakVolumeCallbacks {
            ctx: (&mut *ctx) as *mut CountingReadContext as *mut c_void,
            open_volume: Some(counting_open_volume),
            read: Some(counting_read),
            seek: Some(callback_seek),
            close_volume: Some(callback_close_volume),
        };

        let source = CallbackVolumeSource::new(callbacks).unwrap();
        let mut stream = source.open("buffered.xunbak").unwrap();
        let mut buf = [0u8; 1];
        for _ in 0..64 {
            let read = stream.read(&mut buf).unwrap();
            assert_eq!(read, 1);
            assert_eq!(buf[0], 7);
        }

        assert!(
            read_calls.load(Ordering::SeqCst) <= 2,
            "expected buffered callback reads, got {}",
            read_calls.load(Ordering::SeqCst)
        );
    }
}
