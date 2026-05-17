use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use crate::backup::artifact::entry::{SourceEntry, SourceKind};
use crate::backup::artifact::sevenz::SevenZReaderSource;
use crate::backup::artifact::sevenz_segmented::{MultiVolumeReader, resolve_multivolume_base};
use crate::backup::artifact::zip_ppmd::{
    copy_ppmd_zip_entry_to_writer, needs_manual_ppmd_fallback,
};
use crate::output::CliError;
use crate::util::normalize_glob_path;
use uuid::Uuid;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn copy_entry_to_path(entry: &SourceEntry, dest: &Path) -> Result<(), CliError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                1,
                format!("Create output directory failed {}: {err}", parent.display()),
            )
        })?;
    }

    let mut file = fs::File::create(dest).map_err(|err| {
        CliError::new(
            1,
            format!("Create output file failed {}: {err}", dest.display()),
        )
    })?;
    copy_entry_to_writer(entry, &mut file)?;
    apply_entry_metadata(dest, entry)
}

pub(crate) fn copy_entry_to_path_with_hash(
    entry: &SourceEntry,
    dest: &Path,
) -> Result<[u8; 32], CliError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                1,
                format!("Create output directory failed {}: {err}", parent.display()),
            )
        })?;
    }

    let mut file = fs::File::create(dest).map_err(|err| {
        CliError::new(
            1,
            format!("Create output file failed {}: {err}", dest.display()),
        )
    })?;
    let hash = copy_entry_to_writer_with_hash(entry, &mut file)?;
    apply_entry_metadata(dest, entry)?;
    Ok(hash)
}

pub(crate) fn copy_entry_to_writer(
    entry: &SourceEntry,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    match entry.kind {
        SourceKind::ZipArtifact => return copy_zip_entry_to_writer(entry, writer),
        SourceKind::SevenZArtifact => return copy_7z_entry_to_writer(entry, writer),
        SourceKind::XunbakArtifact => return copy_xunbak_entry_to_writer(entry, writer),
        _ => {}
    }
    let mut reader = open_entry_reader(entry)?;
    std::io::copy(&mut reader, writer)
        .map_err(|err| CliError::new(1, format!("Copy entry failed {}: {err}", entry.path)))?;
    Ok(())
}

pub(crate) fn copy_entry_to_writer_with_hash(
    entry: &SourceEntry,
    writer: &mut dyn Write,
) -> Result<[u8; 32], CliError> {
    let mut reader = open_entry_reader(entry)?;
    let mut hashing_writer = HashingWriter::new(writer);
    std::io::copy(&mut reader, &mut hashing_writer)
        .map_err(|err| CliError::new(1, format!("Copy entry failed {}: {err}", entry.path)))?;
    Ok(hashing_writer.finalize())
}

pub(crate) fn open_entry_reader(entry: &SourceEntry) -> Result<EntryReader, CliError> {
    match entry.kind {
        SourceKind::Filesystem | SourceKind::DirArtifact => open_filesystem_reader(entry),
        SourceKind::ZipArtifact => open_zip_reader(entry),
        SourceKind::XunbakArtifact => open_xunbak_reader(entry),
        SourceKind::SevenZArtifact => open_7z_reader(entry),
    }
}

pub(crate) enum EntryReader {
    File(fs::File),
    Temp(TempReadableFile),
    Stream(StreamingEntryReader),
}

struct HashingWriter<'a> {
    inner: &'a mut dyn Write,
    hasher: blake3::Hasher,
}

impl<'a> HashingWriter<'a> {
    fn new(inner: &'a mut dyn Write) -> Self {
        Self {
            inner,
            hasher: blake3::Hasher::new(),
        }
    }

    fn finalize(self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }
}

impl Write for HashingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write_all(buf)?;
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl Read for EntryReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::File(file) => file.read(buf),
            Self::Temp(file) => file.read(buf),
            Self::Stream(reader) => reader.read(buf),
        }
    }
}

impl Seek for EntryReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match self {
            Self::File(file) => file.seek(pos),
            Self::Temp(file) => file.seek(pos),
            Self::Stream(reader) => reader.seek(pos),
        }
    }
}

enum StreamMessage {
    Data(Vec<u8>),
    Error(String),
    Eof,
}

pub(crate) struct StreamingEntryReader {
    rx: Receiver<StreamMessage>,
    current: std::io::Cursor<Vec<u8>>,
    done: bool,
    join: Option<thread::JoinHandle<()>>,
}

impl StreamingEntryReader {
    fn new(rx: Receiver<StreamMessage>, join: thread::JoinHandle<()>) -> Self {
        Self {
            rx,
            current: std::io::Cursor::new(Vec::new()),
            done: false,
            join: Some(join),
        }
    }

    fn fill_next_chunk(&mut self) -> std::io::Result<bool> {
        if self.done {
            return Ok(false);
        }
        match self.rx.recv() {
            Ok(StreamMessage::Data(chunk)) => {
                self.current = std::io::Cursor::new(chunk);
                Ok(true)
            }
            Ok(StreamMessage::Error(message)) => {
                self.done = true;
                Err(std::io::Error::other(message))
            }
            Ok(StreamMessage::Eof) | Err(_) => {
                self.done = true;
                Ok(false)
            }
        }
    }
}

impl Read for StreamingEntryReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let read = self.current.read(buf)?;
            if read > 0 {
                return Ok(read);
            }
            if !self.fill_next_chunk()? {
                return Ok(0);
            }
        }
    }
}

impl Seek for StreamingEntryReader {
    fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "streaming entry reader does not support seek",
        ))
    }
}

impl Drop for StreamingEntryReader {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub(crate) struct TempReadableFile {
    file: fs::File,
    path: std::path::PathBuf,
}

impl TempReadableFile {
    fn new(prefix: &str) -> Result<Self, CliError> {
        let mut path = std::env::temp_dir();
        path.push(format!("{prefix}-{}", Uuid::new_v4()));
        let file = fs::File::create(&path).map_err(|err| {
            CliError::new(
                1,
                format!("Create temp entry file failed {}: {err}", path.display()),
            )
        })?;
        Ok(Self { file, path })
    }

    fn reopen_for_read(mut self) -> Result<Self, CliError> {
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|err| CliError::new(1, format!("Seek temp entry file failed: {err}")))?;
        let file = fs::File::open(&self.path).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Reopen temp entry file failed {}: {err}",
                    self.path.display()
                ),
            )
        })?;
        self.file = file;
        Ok(self)
    }
}

impl Read for TempReadableFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for TempReadableFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl Write for TempReadableFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl Drop for TempReadableFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn open_filesystem_reader(entry: &SourceEntry) -> Result<EntryReader, CliError> {
    let source = entry.source_path.as_ref().ok_or_else(|| {
        CliError::new(
            1,
            format!("Missing source path for filesystem entry: {}", entry.path),
        )
    })?;
    let file = fs::File::open(source).map_err(|err| {
        CliError::new(
            1,
            format!("Open source file failed {}: {err}", source.display()),
        )
    })?;
    Ok(EntryReader::File(file))
}

fn open_zip_reader(entry: &SourceEntry) -> Result<EntryReader, CliError> {
    let zip_path = entry.source_path.as_ref().ok_or_else(|| {
        CliError::new(
            1,
            format!("Missing zip source path for entry: {}", entry.path),
        )
    })?;
    match open_zip_reader_with_crate(entry, zip_path) {
        Ok(reader) => Ok(reader),
        Err(err) if needs_manual_ppmd_fallback(&err.message) => {
            let mut temp = TempReadableFile::new("xun-entry-zip-ppmd")?;
            copy_ppmd_zip_entry_to_writer(zip_path, &entry.path, &mut temp.file)?;
            Ok(EntryReader::Temp(temp.reopen_for_read()?))
        }
        Err(err) => Err(err),
    }
}

fn open_zip_reader_with_crate(
    entry: &SourceEntry,
    zip_path: &Path,
) -> Result<EntryReader, CliError> {
    let cache = load_cached_zip_archive(zip_path)?;
    let index = zip_entry_index(&cache, &entry.path, zip_path)?;
    let mut temp = TempReadableFile::new("xun-entry-zip")?;
    {
        let mut archive = cache
            .archive
            .lock()
            .map_err(|_| CliError::new(2, "zip reader cache poisoned".to_string()))?;
        let mut zip_entry = archive
            .by_index(index)
            .map_err(|err| CliError::new(1, format!("Zip entry error: {err}")))?;
        std::io::copy(&mut zip_entry, &mut temp).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Copy zip entry failed {}::{}: {err}",
                    zip_path.display(),
                    entry.path
                ),
            )
        })?;
    }
    Ok(EntryReader::Temp(temp.reopen_for_read()?))
}

fn copy_zip_entry_to_writer(entry: &SourceEntry, writer: &mut dyn Write) -> Result<(), CliError> {
    let zip_path = entry.source_path.as_ref().ok_or_else(|| {
        CliError::new(
            1,
            format!("Missing zip source path for entry: {}", entry.path),
        )
    })?;
    match copy_zip_entry_to_writer_with_crate(entry, zip_path, writer) {
        Ok(()) => Ok(()),
        Err(err) if needs_manual_ppmd_fallback(&err.message) => {
            copy_ppmd_zip_entry_to_writer(zip_path, &entry.path, writer)
        }
        Err(err) => Err(err),
    }
}

fn copy_zip_entry_to_writer_with_crate(
    entry: &SourceEntry,
    zip_path: &Path,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let cache = load_cached_zip_archive(zip_path)?;
    let index = zip_entry_index(&cache, &entry.path, zip_path)?;
    let mut archive = cache
        .archive
        .lock()
        .map_err(|_| CliError::new(2, "zip reader cache poisoned".to_string()))?;
    let mut zip_entry = archive
        .by_index(index)
        .map_err(|err| CliError::new(1, format!("Zip entry error: {err}")))?;
    std::io::copy(&mut zip_entry, writer).map_err(|err| {
        CliError::new(
            1,
            format!(
                "Copy zip entry failed {}::{}: {err}",
                zip_path.display(),
                entry.path
            ),
        )
    })?;
    Ok(())
}

fn open_7z_reader(entry: &SourceEntry) -> Result<EntryReader, CliError> {
    let source = entry.source_path.as_ref().ok_or_else(|| {
        CliError::new(
            1,
            format!("Missing 7z source path for entry: {}", entry.path),
        )
    })?;
    let cache = load_cached_sevenz_reader(source)?;
    let indexed = cache
        .index
        .get(&normalize_artifact_entry_key(&entry.path))
        .cloned()
        .ok_or_else(|| {
            CliError::new(
                1,
                format!("7z entry not found {}::{}", source.display(), entry.path),
            )
        })?;
    let (tx, rx) = sync_channel::<StreamMessage>(2);
    let source_path = source.to_path_buf();
    let entry_path = entry.path.clone();
    let join = thread::spawn(move || {
        stream_cached_7z_entry(cache, indexed.actual, source_path, entry_path, tx);
    });
    Ok(EntryReader::Stream(StreamingEntryReader::new(rx, join)))
}

fn copy_7z_entry_to_writer(entry: &SourceEntry, writer: &mut dyn Write) -> Result<(), CliError> {
    let source = entry.source_path.as_ref().ok_or_else(|| {
        CliError::new(
            1,
            format!("Missing 7z source path for entry: {}", entry.path),
        )
    })?;
    copy_cached_7z_entry_to_writer(source, &entry.path, writer)
}

#[cfg(windows)]
fn apply_entry_metadata(path: &Path, entry: &SourceEntry) -> Result<(), CliError> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{FILETIME, HANDLE};
    use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, SetFileTime};

    if entry.created_time_ns.is_none() && entry.mtime_ns.is_none() && entry.win_attributes == 0 {
        return Ok(());
    }

    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|err| {
            CliError::new(
                1,
                format!("Open output file failed {}: {err}", path.display()),
            )
        })?;

    let created = entry
        .created_time_ns
        .map(|value| filetime_from_unix_ns(value as i128));
    let modified = entry
        .mtime_ns
        .map(|value| filetime_from_unix_ns(value as i128));
    let created_ptr = created
        .as_ref()
        .map(|value| value as *const FILETIME)
        .unwrap_or(std::ptr::null());
    let modified_ptr = modified
        .as_ref()
        .map(|value| value as *const FILETIME)
        .unwrap_or(std::ptr::null());
    if !created_ptr.is_null() || !modified_ptr.is_null() {
        let ok = unsafe {
            SetFileTime(
                file.as_raw_handle() as HANDLE,
                created_ptr,
                std::ptr::null(),
                modified_ptr,
            )
        };
        if ok == 0 {
            return Err(CliError::new(
                1,
                format!("SetFileTime failed for {}", path.display()),
            ));
        }
    }

    if entry.win_attributes != 0 {
        let verbatim = to_verbatim_path(path);
        let mut wide: Vec<u16> = verbatim.as_os_str().encode_wide().collect();
        wide.push(0);
        let ok = unsafe { SetFileAttributesW(wide.as_ptr(), entry.win_attributes) };
        if ok == 0 {
            return Err(CliError::new(
                1,
                format!("SetFileAttributesW failed for {}", path.display()),
            ));
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn apply_entry_metadata(_path: &Path, _entry: &SourceEntry) -> Result<(), CliError> {
    Ok(())
}

#[cfg(windows)]
fn filetime_from_unix_ns(unix_ns: i128) -> windows_sys::Win32::Foundation::FILETIME {
    let filetime = (unix_ns / 100 + 116_444_736_000_000_000) as u64;
    windows_sys::Win32::Foundation::FILETIME {
        dwLowDateTime: filetime as u32,
        dwHighDateTime: (filetime >> 32) as u32,
    }
}

#[cfg(windows)]
fn to_verbatim_path(path: &Path) -> std::path::PathBuf {
    let raw = path.to_string_lossy().replace('/', "\\");
    if raw.starts_with(r"\\?\") {
        return path.to_path_buf();
    }
    std::path::PathBuf::from(format!(r"\\?\{raw}"))
}

struct ZipArchiveCache {
    archive: Mutex<zip::ZipArchive<fs::File>>,
    index: std::collections::HashMap<String, usize>,
}

struct SevenZArchiveCache {
    reader: Mutex<sevenz_rust2::ArchiveReader<SevenZReaderSource>>,
    index: std::collections::HashMap<String, SevenZIndexEntry>,
}

#[derive(Clone)]
struct SevenZIndexEntry {
    actual: String,
    file_index: usize,
}

fn zip_reader_cache()
-> &'static Mutex<std::collections::HashMap<std::path::PathBuf, Arc<ZipArchiveCache>>> {
    static CACHE: OnceLock<
        Mutex<std::collections::HashMap<std::path::PathBuf, Arc<ZipArchiveCache>>>,
    > = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn sevenz_reader_cache()
-> &'static Mutex<std::collections::HashMap<std::path::PathBuf, Arc<SevenZArchiveCache>>> {
    static CACHE: OnceLock<
        Mutex<std::collections::HashMap<std::path::PathBuf, Arc<SevenZArchiveCache>>>,
    > = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

#[cfg(test)]
pub(crate) fn clear_zip_reader_cache_for_tests() {
    if let Ok(mut cache) = zip_reader_cache().lock() {
        cache.clear();
    }
}

#[cfg(test)]
pub(crate) fn clear_sevenz_reader_cache_for_tests() {
    if let Ok(mut cache) = sevenz_reader_cache().lock() {
        cache.clear();
    }
}

#[cfg(test)]
pub(crate) fn cached_zip_reader_count_for_tests() -> usize {
    zip_reader_cache()
        .lock()
        .map(|cache| cache.len())
        .unwrap_or(0)
}

#[cfg(test)]
pub(crate) fn cached_sevenz_reader_count_for_tests() -> usize {
    sevenz_reader_cache()
        .lock()
        .map(|cache| cache.len())
        .unwrap_or(0)
}

fn normalize_artifact_entry_key(path: &str) -> String {
    normalize_glob_path(path)
}

fn load_cached_zip_archive(path: &Path) -> Result<Arc<ZipArchiveCache>, CliError> {
    let key = path.to_path_buf();
    {
        let cache = zip_reader_cache()
            .lock()
            .map_err(|_| CliError::new(2, "zip reader cache poisoned".to_string()))?;
        if let Some(entry) = cache.get(&key) {
            return Ok(entry.clone());
        }
    }

    let file = fs::File::open(path)
        .map_err(|err| CliError::new(1, format!("Open zip failed {}: {err}", path.display())))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|err| CliError::new(1, format!("Read zip failed {}: {err}", path.display())))?;
    let mut index = std::collections::HashMap::new();
    for idx in 0..archive.len() {
        let name = {
            let entry = archive
                .by_index(idx)
                .map_err(|err| CliError::new(1, format!("Zip entry error: {err}")))?;
            entry.name().replace('\\', "/")
        };
        index
            .entry(normalize_artifact_entry_key(&name))
            .or_insert(idx);
    }
    let entry = Arc::new(ZipArchiveCache {
        archive: Mutex::new(archive),
        index,
    });
    let mut cache = zip_reader_cache()
        .lock()
        .map_err(|_| CliError::new(2, "zip reader cache poisoned".to_string()))?;
    cache.insert(key, entry.clone());
    Ok(entry)
}

fn zip_entry_index(
    cache: &ZipArchiveCache,
    path: &str,
    zip_path: &Path,
) -> Result<usize, CliError> {
    cache
        .index
        .get(&normalize_artifact_entry_key(path))
        .copied()
        .ok_or_else(|| {
            CliError::new(
                1,
                format!("Zip entry not found {}::{}", zip_path.display(), path),
            )
        })
}

fn load_cached_sevenz_reader(path: &Path) -> Result<Arc<SevenZArchiveCache>, CliError> {
    let key = path.to_path_buf();
    {
        let cache = sevenz_reader_cache()
            .lock()
            .map_err(|_| CliError::new(2, "7z reader cache poisoned".to_string()))?;
        if let Some(entry) = cache.get(&key) {
            return Ok(entry.clone());
        }
    }

    let source = if resolve_multivolume_base(path).is_some() {
        let reader = MultiVolumeReader::open(path).map_err(|err| {
            CliError::new(
                1,
                format!("Open split 7z volumes failed {}: {err}", path.display()),
            )
        })?;
        SevenZReaderSource::Multi(reader)
    } else {
        let file = fs::File::open(path)
            .map_err(|err| CliError::new(1, format!("Open 7z failed {}: {err}", path.display())))?;
        SevenZReaderSource::File(file)
    };
    let reader = sevenz_rust2::ArchiveReader::new(source, sevenz_rust2::Password::empty())
        .map_err(|err| CliError::new(1, format!("Open 7z failed {}: {err}", path.display())))?;
    let mut index = std::collections::HashMap::new();
    for (file_index, entry) in reader.archive().files.iter().enumerate() {
        if entry.is_directory || !entry.has_stream {
            continue;
        }
        let actual = entry.name.replace('\\', "/");
        index
            .entry(normalize_artifact_entry_key(&actual))
            .or_insert(SevenZIndexEntry { actual, file_index });
    }
    let entry = Arc::new(SevenZArchiveCache {
        reader: Mutex::new(reader),
        index,
    });
    let mut cache = sevenz_reader_cache()
        .lock()
        .map_err(|_| CliError::new(2, "7z reader cache poisoned".to_string()))?;
    cache.insert(key, entry.clone());
    Ok(entry)
}

fn copy_cached_7z_entry_to_writer(
    path: &Path,
    entry_path: &str,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let cache = load_cached_sevenz_reader(path)?;
    let indexed = cache
        .index
        .get(&normalize_artifact_entry_key(entry_path))
        .cloned()
        .ok_or_else(|| {
            CliError::new(
                1,
                format!("7z entry not found {}::{}", path.display(), entry_path),
            )
        })?;
    let mut reader = cache
        .reader
        .lock()
        .map_err(|_| CliError::new(2, "7z reader cache poisoned".to_string()))?;
    let mut found = false;
    reader
        .for_each_entries(
            &mut |archive_entry: &sevenz_rust2::ArchiveEntry, data: &mut dyn Read| {
                if archive_entry.is_directory() || !archive_entry.has_stream() {
                    return Ok(true);
                }
                if archive_entry.name().replace('\\', "/") != indexed.actual {
                    return Ok(true);
                }
                std::io::copy(data, writer)?;
                found = true;
                Ok(false)
            },
        )
        .map_err(|err| CliError::new(1, format!("Read 7z file failed {entry_path}: {err}")))?;
    if !found {
        return Err(CliError::new(
            1,
            format!("7z entry not found {}::{}", path.display(), entry_path),
        ));
    }
    Ok(())
}

fn stream_cached_7z_entry(
    cache: Arc<SevenZArchiveCache>,
    actual: String,
    source_path: std::path::PathBuf,
    entry_path: String,
    tx: SyncSender<StreamMessage>,
) {
    let send_error = |message: String, tx: &SyncSender<StreamMessage>| {
        let _ = tx.send(StreamMessage::Error(message));
    };
    let mut reader = match cache.reader.lock() {
        Ok(reader) => reader,
        Err(_) => {
            send_error("7z reader cache poisoned".to_string(), &tx);
            return;
        }
    };
    let mut found = false;
    let result =
        reader.for_each_entries(&mut |archive_entry: &sevenz_rust2::ArchiveEntry,
                                      data: &mut dyn Read| {
            if archive_entry.is_directory() || !archive_entry.has_stream() {
                return Ok(true);
            }
            if archive_entry.name().replace('\\', "/") != actual {
                return Ok(true);
            }
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let read = data.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                if tx
                    .send(StreamMessage::Data(buffer[..read].to_vec()))
                    .is_err()
                {
                    return Ok(false);
                }
            }
            found = true;
            Ok(false)
        });
    match result {
        Ok(()) if found => {
            let _ = tx.send(StreamMessage::Eof);
        }
        Ok(()) => {
            send_error(
                format!(
                    "7z entry not found {}::{}",
                    source_path.display(),
                    entry_path
                ),
                &tx,
            );
        }
        Err(err) => {
            send_error(format!("Read 7z file failed {entry_path}: {err}"), &tx);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ReadOrderKey {
    class_rank: u8,
    primary: u64,
    secondary: u64,
}

pub(crate) fn sort_entry_refs_for_read_locality(
    entries: &mut Vec<&SourceEntry>,
) -> Result<(), CliError> {
    let keys = build_read_order_keys(entries.iter().map(|entry| *entry))?;
    entries.sort_by(|left, right| {
        let left_key = keys.get(&entry_identity(left));
        let right_key = keys.get(&entry_identity(right));
        left_key.cmp(&right_key).then(left.path.cmp(&right.path))
    });
    Ok(())
}

pub(crate) fn sort_entries_for_read_locality(
    entries: &mut Vec<SourceEntry>,
) -> Result<(), CliError> {
    let keys = build_read_order_keys(entries.iter())?;
    entries.sort_by(|left, right| {
        let left_key = keys.get(&entry_identity(left));
        let right_key = keys.get(&entry_identity(right));
        left_key.cmp(&right_key).then(left.path.cmp(&right.path))
    });
    Ok(())
}

fn build_read_order_keys<'a, I>(
    entries: I,
) -> Result<std::collections::HashMap<(String, String), ReadOrderKey>, CliError>
where
    I: IntoIterator<Item = &'a SourceEntry>,
{
    let mut keys = std::collections::HashMap::new();
    for entry in entries {
        let key = match entry.kind {
            SourceKind::XunbakArtifact => xunbak_entry_read_order(entry)?,
            SourceKind::ZipArtifact => zip_entry_read_order(entry)?,
            SourceKind::SevenZArtifact => sevenz_entry_read_order(entry)?,
            _ => ReadOrderKey {
                class_rank: 255,
                primary: 0,
                secondary: 0,
            },
        };
        keys.insert(entry_identity(entry), key);
    }
    Ok(keys)
}

fn entry_identity(entry: &SourceEntry) -> (String, String) {
    (
        entry.path.clone(),
        entry
            .source_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default(),
    )
}

fn xunbak_entry_read_order(_entry: &SourceEntry) -> Result<ReadOrderKey, CliError> {
    #[cfg(feature = "xunbak")]
    {
        if let Some(path) = _entry.source_path.as_deref() {
            let (_reader, manifest) = load_cached_xunbak_reader_and_manifest(path)?;
            if let Some(manifest_entry) = manifest.get(&normalize_artifact_entry_key(&_entry.path)) {
                return Ok(ReadOrderKey {
                    class_rank: 0,
                    primary: manifest_entry.volume_index as u64,
                    secondary: manifest_entry.blob_offset,
                });
            }
        }
    }
    Ok(ReadOrderKey {
        class_rank: 255,
        primary: 0,
        secondary: 0,
    })
}

fn zip_entry_read_order(entry: &SourceEntry) -> Result<ReadOrderKey, CliError> {
    if let Some(path) = entry.source_path.as_deref() {
        let cache = match load_cached_zip_archive(path) {
            Ok(cache) => cache,
            Err(err) if needs_manual_ppmd_fallback(&err.message) => {
                return Ok(ReadOrderKey {
                    class_rank: 1,
                    primary: u64::MAX,
                    secondary: 0,
                });
            }
            Err(err) => return Err(err),
        };
        let index = cache
            .index
            .get(&normalize_artifact_entry_key(&entry.path))
            .copied()
            .unwrap_or(usize::MAX) as u64;
        return Ok(ReadOrderKey {
            class_rank: 1,
            primary: index,
            secondary: 0,
        });
    }
    Ok(ReadOrderKey {
        class_rank: 255,
        primary: 0,
        secondary: 0,
    })
}

fn sevenz_entry_read_order(entry: &SourceEntry) -> Result<ReadOrderKey, CliError> {
    if let Some(path) = entry.source_path.as_deref() {
        let cache = load_cached_sevenz_reader(path)?;
        let index = cache
            .index
            .get(&normalize_artifact_entry_key(&entry.path))
            .map(|value| value.file_index)
            .unwrap_or(usize::MAX) as u64;
        return Ok(ReadOrderKey {
            class_rank: 2,
            primary: index,
            secondary: 0,
        });
    }
    Ok(ReadOrderKey {
        class_rank: 255,
        primary: 0,
        secondary: 0,
    })
}

#[cfg(feature = "xunbak")]
fn xunbak_reader_cache() -> &'static Mutex<
    std::collections::HashMap<
        std::path::PathBuf,
        (
            Arc<crate::xunbak::reader::ContainerReader>,
            Arc<std::collections::HashMap<String, crate::xunbak::manifest::ManifestEntry>>,
        ),
    >,
> {
    static CACHE: OnceLock<
        Mutex<
            std::collections::HashMap<
                std::path::PathBuf,
                (
                    Arc<crate::xunbak::reader::ContainerReader>,
                    Arc<std::collections::HashMap<String, crate::xunbak::manifest::ManifestEntry>>,
                ),
            >,
        >,
    > = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

#[cfg(all(test, feature = "xunbak"))]
pub(crate) fn clear_xunbak_reader_cache_for_tests() {
    if let Ok(mut cache) = xunbak_reader_cache().lock() {
        cache.clear();
    }
}

#[cfg(all(test, feature = "xunbak"))]
pub(crate) fn cached_xunbak_reader_count_for_tests() -> usize {
    xunbak_reader_cache()
        .lock()
        .map(|cache| cache.len())
        .unwrap_or(0)
}

#[cfg(feature = "xunbak")]
fn load_cached_xunbak_reader_and_manifest(
    path: &Path,
) -> Result<
    (
        Arc<crate::xunbak::reader::ContainerReader>,
        Arc<std::collections::HashMap<String, crate::xunbak::manifest::ManifestEntry>>,
    ),
    CliError,
> {
    let key = path.to_path_buf();
    {
        let cache = xunbak_reader_cache()
            .lock()
            .map_err(|_| CliError::new(2, "xunbak reader cache poisoned".to_string()))?;
        if let Some((reader, manifest)) = cache.get(&key) {
            return Ok((reader.clone(), manifest.clone()));
        }
    }

    use crate::xunbak::reader::ContainerReader;
    let reader =
        Arc::new(ContainerReader::open(path).map_err(|err| CliError::new(2, err.to_string()))?);
    let manifest = reader
        .load_manifest()
        .map_err(|err| CliError::new(2, err.to_string()))?;
    let manifest_index = Arc::new(
        manifest
            .entries
            .into_iter()
            .map(|entry| (normalize_artifact_entry_key(&entry.path), entry))
            .collect::<std::collections::HashMap<_, _>>(),
    );

    let mut cache = xunbak_reader_cache()
        .lock()
        .map_err(|_| CliError::new(2, "xunbak reader cache poisoned".to_string()))?;
    cache.insert(key, (reader.clone(), manifest_index.clone()));
    Ok((reader, manifest_index))
}

#[cfg(feature = "xunbak")]
fn open_xunbak_reader(entry: &SourceEntry) -> Result<EntryReader, CliError> {
    let path = entry.source_path.as_ref().ok_or_else(|| {
        CliError::new(
            1,
            format!("Missing xunbak source path for entry: {}", entry.path),
        )
    })?;
    let (reader, manifest) = load_cached_xunbak_reader_and_manifest(path)?;
    let manifest_entry = manifest
        .get(&normalize_artifact_entry_key(&entry.path))
        .ok_or_else(|| CliError::new(1, format!("xunbak entry not found: {}", entry.path)))?;
    let mut temp = TempReadableFile::new("xun-entry-xunbak")?;
    reader
        .copy_and_verify_blob(manifest_entry, &mut temp.file)
        .map_err(|err| CliError::new(2, err.to_string()))?;
    Ok(EntryReader::Temp(temp.reopen_for_read()?))
}

#[cfg(feature = "xunbak")]
fn copy_xunbak_entry_to_writer(
    entry: &SourceEntry,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let path = entry.source_path.as_ref().ok_or_else(|| {
        CliError::new(
            1,
            format!("Missing xunbak source path for entry: {}", entry.path),
        )
    })?;
    let (reader, manifest) = load_cached_xunbak_reader_and_manifest(path)?;
    let manifest_entry = manifest
        .get(&normalize_artifact_entry_key(&entry.path))
        .ok_or_else(|| CliError::new(1, format!("xunbak entry not found: {}", entry.path)))?;
    reader
        .copy_and_verify_blob(manifest_entry, writer)
        .map_err(|err| CliError::new(2, err.to_string()))
}

#[cfg(not(feature = "xunbak"))]
fn copy_xunbak_entry_to_writer(
    entry: &SourceEntry,
    _writer: &mut dyn Write,
) -> Result<(), CliError> {
    Err(CliError::with_details(
        2,
        format!(
            "xunbak artifact reading is not enabled in this build: {}",
            entry.path
        ),
        &["Fix: Rebuild with `--features xunbak`."],
    ))
}

#[cfg(not(feature = "xunbak"))]
fn open_xunbak_reader(entry: &SourceEntry) -> Result<EntryReader, CliError> {
    Err(CliError::with_details(
        2,
        format!(
            "xunbak artifact reading is not enabled in this build: {}",
            entry.path
        ),
        &["Fix: Rebuild with `--features xunbak`."],
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Write};
    use std::sync::Arc;

    use serial_test::serial;
    use tempfile::tempdir;

    use crate::backup::artifact::entry::{SourceEntry, SourceKind};
    use crate::backup::artifact::source::read_artifact_entries;

    use super::{
        cached_sevenz_reader_count_for_tests, cached_zip_reader_count_for_tests,
        clear_sevenz_reader_cache_for_tests, clear_zip_reader_cache_for_tests, copy_entry_to_path,
        copy_entry_to_writer, load_cached_sevenz_reader, load_cached_zip_archive,
        open_entry_reader, sort_entries_for_read_locality,
    };
    #[cfg(feature = "xunbak")]
    use super::{
        cached_xunbak_reader_count_for_tests, clear_xunbak_reader_cache_for_tests,
        load_cached_xunbak_reader_and_manifest,
    };

    struct ChunkLimitedWriter {
        max_write_len: usize,
        total_bytes: usize,
    }

    impl ChunkLimitedWriter {
        fn new(max_write_len: usize) -> Self {
            Self {
                max_write_len,
                total_bytes: 0,
            }
        }
    }

    impl Write for ChunkLimitedWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if buf.len() > self.max_write_len {
                return Err(std::io::Error::other(format!(
                    "chunk too large: {} > {}",
                    buf.len(),
                    self.max_write_len
                )));
            }
            self.total_bytes += buf.len();
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn copy_entry_to_path_copies_directory_artifact_file() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src").join("main.rs");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "fn main() {}").unwrap();
        let dest = dir.path().join("out").join("src").join("main.rs");
        let entry = SourceEntry {
            path: "src/main.rs".to_string(),
            source_path: Some(source),
            size: 12,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };

        copy_entry_to_path(&entry, &dest).unwrap();

        assert_eq!(fs::read_to_string(dest).unwrap(), "fn main() {}");
    }

    #[test]
    fn copy_entry_to_path_copies_zip_entry() {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("artifact.zip");
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("nested/data.txt", options).unwrap();
        writer.write_all(b"hello zip").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        fs::write(&zip_path, bytes).unwrap();

        let entry = SourceEntry {
            path: "nested/data.txt".to_string(),
            source_path: Some(zip_path),
            size: 9,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::ZipArtifact,
        };
        let dest = dir.path().join("out").join("nested").join("data.txt");

        copy_entry_to_path(&entry, &dest).unwrap();

        assert_eq!(fs::read_to_string(dest).unwrap(), "hello zip");
    }

    #[test]
    fn open_entry_reader_returns_readable_stream_for_directory_artifact() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("a.txt");
        fs::write(&source, "hello").unwrap();
        let entry = SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(source),
            size: 5,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };

        let mut reader = open_entry_reader(&entry).unwrap();
        let mut content = String::new();
        reader.read_to_string(&mut content).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn open_entry_reader_returns_readable_stream_for_zip_artifact() {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("artifact.zip");
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("nested/data.txt", options).unwrap();
        writer.write_all(b"hello zip").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        fs::write(&zip_path, bytes).unwrap();

        let entry = SourceEntry {
            path: "nested/data.txt".to_string(),
            source_path: Some(zip_path),
            size: 9,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::ZipArtifact,
        };

        let mut reader = open_entry_reader(&entry).unwrap();
        let mut content = String::new();
        reader.read_to_string(&mut content).unwrap();
        assert_eq!(content, "hello zip");
    }

    #[test]
    #[serial]
    fn load_cached_zip_archive_reuses_same_archive_for_same_artifact() {
        clear_zip_reader_cache_for_tests();
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("artifact.zip");
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("a.txt", options).unwrap();
        writer.write_all(b"aaa").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        fs::write(&zip_path, bytes).unwrap();

        let cache_a = load_cached_zip_archive(&zip_path).unwrap();
        let cache_b = load_cached_zip_archive(&zip_path).unwrap();
        assert!(Arc::ptr_eq(&cache_a, &cache_b));
        assert_eq!(cached_zip_reader_count_for_tests(), 1);
    }

    #[test]
    fn open_entry_reader_returns_readable_stream_for_ppmd_zip_artifact() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("data.txt");
        fs::write(&source, "hello ppmd zip").unwrap();
        let zip_path = dir.path().join("artifact-ppmd.zip");
        let entry = SourceEntry {
            path: "data.txt".to_string(),
            source_path: Some(source),
            size: 14,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        crate::backup::artifact::zip::write_entries_to_zip(
            &[&entry],
            &zip_path,
            crate::backup::artifact::zip::ZipWriteOptions {
                method: crate::backup::artifact::zip::ZipCompressionMethod::Ppmd,
                level: None,
                sidecar: None,
                sidecar_plan: None,
            },
        )
        .unwrap();

        let artifact_entry = SourceEntry {
            path: "data.txt".to_string(),
            source_path: Some(zip_path),
            size: 14,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::ZipArtifact,
        };

        let mut reader = open_entry_reader(&artifact_entry).unwrap();
        let mut content = String::new();
        reader.read_to_string(&mut content).unwrap();
        assert_eq!(content, "hello ppmd zip");
    }

    #[test]
    fn open_entry_reader_returns_readable_stream_for_7z_artifact() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("a.txt");
        fs::write(&source, "hello 7z").unwrap();
        let archive_path = dir.path().join("artifact.7z");
        let entry = SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(source.clone()),
            size: 8,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        crate::backup::artifact::sevenz::write_entries_to_7z(
            &[&entry],
            &archive_path,
            &crate::backup::artifact::sevenz::SevenZWriteOptions::default(),
        )
        .unwrap();

        let artifact_entry = SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(archive_path),
            size: 8,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::SevenZArtifact,
        };

        let mut reader = open_entry_reader(&artifact_entry).unwrap();
        assert!(matches!(reader, super::EntryReader::Stream(_)));
        let mut content = String::new();
        reader.read_to_string(&mut content).unwrap();
        assert_eq!(content, "hello 7z");
    }

    #[test]
    #[serial]
    fn load_cached_sevenz_reader_reuses_same_reader_for_same_artifact() {
        clear_sevenz_reader_cache_for_tests();
        let dir = tempdir().unwrap();
        let source = dir.path().join("a.txt");
        fs::write(&source, "hello 7z").unwrap();
        let archive_path = dir.path().join("artifact.7z");
        let entry = SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(source),
            size: 8,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        crate::backup::artifact::sevenz::write_entries_to_7z(
            &[&entry],
            &archive_path,
            &crate::backup::artifact::sevenz::SevenZWriteOptions::default(),
        )
        .unwrap();

        let cache_a = load_cached_sevenz_reader(&archive_path).unwrap();
        let cache_b = load_cached_sevenz_reader(&archive_path).unwrap();
        assert!(Arc::ptr_eq(&cache_a, &cache_b));
        assert!(cached_sevenz_reader_count_for_tests() >= 1);
    }

    #[test]
    #[serial]
    fn copy_entry_to_writer_streams_large_7z_artifact_without_vec_buffering() {
        clear_sevenz_reader_cache_for_tests();
        let dir = tempdir().unwrap();
        let source = dir.path().join("large.txt");
        fs::write(
            &source,
            "alpha alpha alpha beta beta beta gamma gamma gamma\n".repeat(300_000),
        )
        .unwrap();
        let archive_path = dir.path().join("artifact.7z");
        let entry = SourceEntry {
            path: "large.txt".to_string(),
            source_path: Some(source.clone()),
            size: fs::metadata(&source).unwrap().len(),
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        crate::backup::artifact::sevenz::write_entries_to_7z(
            &[&entry],
            &archive_path,
            &crate::backup::artifact::sevenz::SevenZWriteOptions {
                solid: false,
                method: crate::backup::artifact::sevenz::SevenZMethod::Copy,
                level: 1,
                sidecar: None,
                sidecar_plan: None,
            },
        )
        .unwrap();

        let artifact_entry = SourceEntry {
            path: "large.txt".to_string(),
            source_path: Some(archive_path),
            size: entry.size,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::SevenZArtifact,
        };

        let mut writer = ChunkLimitedWriter::new(128 * 1024);
        copy_entry_to_writer(&artifact_entry, &mut writer).unwrap();
        assert_eq!(writer.total_bytes as u64, entry.size);
    }

    #[test]
    #[serial]
    fn sort_entries_for_read_locality_prefers_zip_archive_order() {
        clear_zip_reader_cache_for_tests();
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("artifact.zip");
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("z.txt", options).unwrap();
        writer.write_all(b"zzz").unwrap();
        writer.start_file("a.txt", options).unwrap();
        writer.write_all(b"aaa").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        fs::write(&zip_path, bytes).unwrap();

        let mut entries = read_artifact_entries(&zip_path).unwrap();
        let before: Vec<String> = entries.iter().map(|entry| entry.path.clone()).collect();
        assert_eq!(before, vec!["a.txt", "z.txt"]);

        sort_entries_for_read_locality(&mut entries).unwrap();
        let after: Vec<String> = entries.iter().map(|entry| entry.path.clone()).collect();
        assert_eq!(after, vec!["z.txt", "a.txt"]);
    }

    #[test]
    #[serial]
    fn sort_entries_for_read_locality_prefers_sevenz_archive_order() {
        clear_sevenz_reader_cache_for_tests();
        let dir = tempdir().unwrap();
        let z_path = dir.path().join("artifact.7z");
        let src_z = dir.path().join("z.txt");
        let src_a = dir.path().join("a.txt");
        fs::write(&src_z, "zzz").unwrap();
        fs::write(&src_a, "aaa").unwrap();
        let entry_z = SourceEntry {
            path: "z.txt".to_string(),
            source_path: Some(src_z),
            size: 3,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let entry_a = SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(src_a),
            size: 3,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        crate::backup::artifact::sevenz::write_entries_to_7z(
            &[&entry_z, &entry_a],
            &z_path,
            &crate::backup::artifact::sevenz::SevenZWriteOptions {
                solid: false,
                method: crate::backup::artifact::sevenz::SevenZMethod::Copy,
                level: 1,
                sidecar: None,
                sidecar_plan: None,
            },
        )
        .unwrap();

        let mut entries = read_artifact_entries(&z_path).unwrap();
        let before: Vec<String> = entries.iter().map(|entry| entry.path.clone()).collect();
        assert_eq!(before, vec!["a.txt", "z.txt"]);

        sort_entries_for_read_locality(&mut entries).unwrap();
        let after: Vec<String> = entries.iter().map(|entry| entry.path.clone()).collect();
        assert_eq!(after, vec!["z.txt", "a.txt"]);
    }

    #[cfg(feature = "xunbak")]
    #[test]
    #[serial]
    fn open_entry_reader_returns_readable_stream_for_xunbak_artifact() {
        clear_xunbak_reader_cache_for_tests();
        let dir = tempdir().unwrap();
        let source = dir.path().join("a.txt");
        fs::write(&source, "hello xunbak").unwrap();
        let artifact = dir.path().join("artifact.xunbak");
        let entry = SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(source),
            size: 12,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        crate::backup::artifact::xunbak::write_entries_to_xunbak(
            &[&entry],
            &artifact,
            &dir.path().display().to_string(),
            &crate::xunbak::writer::BackupOptions {
                codec: crate::xunbak::constants::Codec::NONE,
                auto_compression: false,
                zstd_level: 1,
                split_size: None,
            },
            crate::backup_formats::OverwriteMode::Fail,
        )
        .unwrap();

        let artifact_entry = SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(artifact),
            size: 12,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::XunbakArtifact,
        };

        let mut reader = open_entry_reader(&artifact_entry).unwrap();
        let mut content = String::new();
        reader.read_to_string(&mut content).unwrap();
        assert_eq!(content, "hello xunbak");
    }

    #[cfg(feature = "xunbak")]
    #[test]
    #[serial]
    fn load_cached_xunbak_reader_and_manifest_reuses_same_reader_for_same_artifact() {
        clear_xunbak_reader_cache_for_tests();
        let dir = tempdir().unwrap();
        let source = dir.path().join("a.txt");
        fs::write(&source, "hello xunbak").unwrap();
        let artifact = dir.path().join("artifact.xunbak");
        let entry = SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(source),
            size: 12,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        crate::backup::artifact::xunbak::write_entries_to_xunbak(
            &[&entry],
            &artifact,
            &dir.path().display().to_string(),
            &crate::xunbak::writer::BackupOptions {
                codec: crate::xunbak::constants::Codec::NONE,
                auto_compression: false,
                zstd_level: 1,
                split_size: None,
            },
            crate::backup_formats::OverwriteMode::Fail,
        )
        .unwrap();

        let (reader_a, manifest_a) = load_cached_xunbak_reader_and_manifest(&artifact).unwrap();
        let (reader_b, manifest_b) = load_cached_xunbak_reader_and_manifest(&artifact).unwrap();

        assert!(Arc::ptr_eq(&reader_a, &reader_b));
        assert!(Arc::ptr_eq(&manifest_a, &manifest_b));
        assert_eq!(cached_xunbak_reader_count_for_tests(), 1);
    }

    #[cfg(feature = "xunbak")]
    #[test]
    #[serial]
    fn open_entry_reader_reuse_does_not_change_multi_file_restore_content() {
        clear_xunbak_reader_cache_for_tests();
        let dir = tempdir().unwrap();
        let source_a = dir.path().join("a.txt");
        let source_b = dir.path().join("nested").join("b.txt");
        fs::create_dir_all(source_b.parent().unwrap()).unwrap();
        fs::write(&source_a, "alpha").unwrap();
        fs::write(&source_b, "beta").unwrap();
        let artifact = dir.path().join("artifact.xunbak");
        let entry_a = SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(source_a),
            size: 5,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let entry_b = SourceEntry {
            path: "nested/b.txt".to_string(),
            source_path: Some(source_b),
            size: 4,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        crate::backup::artifact::xunbak::write_entries_to_xunbak(
            &[&entry_a, &entry_b],
            &artifact,
            &dir.path().display().to_string(),
            &crate::xunbak::writer::BackupOptions {
                codec: crate::xunbak::constants::Codec::NONE,
                auto_compression: false,
                zstd_level: 1,
                split_size: None,
            },
            crate::backup_formats::OverwriteMode::Fail,
        )
        .unwrap();

        let artifact_entry_a = SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(artifact.clone()),
            size: 5,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::XunbakArtifact,
        };
        let artifact_entry_b = SourceEntry {
            path: "nested/b.txt".to_string(),
            source_path: Some(artifact),
            size: 4,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::XunbakArtifact,
        };

        let mut reader_a = open_entry_reader(&artifact_entry_a).unwrap();
        let mut reader_b = open_entry_reader(&artifact_entry_b).unwrap();
        let mut content_a = String::new();
        let mut content_b = String::new();
        reader_a.read_to_string(&mut content_a).unwrap();
        reader_b.read_to_string(&mut content_b).unwrap();

        assert_eq!(content_a, "alpha");
        assert_eq!(content_b, "beta");
        assert_eq!(cached_xunbak_reader_count_for_tests(), 1);
    }
}
