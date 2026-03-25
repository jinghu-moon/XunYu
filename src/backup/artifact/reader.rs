use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
#[cfg(feature = "xunbak")]
use std::sync::{Arc, Mutex, OnceLock};

use crate::backup::artifact::entry::{SourceEntry, SourceKind};
use crate::backup::artifact::sevenz::read_7z_file;
use crate::backup::artifact::zip_ppmd::{
    copy_ppmd_zip_entry_to_writer, needs_manual_ppmd_fallback,
};
use crate::output::CliError;
#[cfg(feature = "xunbak")]
use crate::util::normalize_glob_path;
use uuid::Uuid;

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
}

impl Read for EntryReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::File(file) => file.read(buf),
            Self::Temp(file) => file.read(buf),
        }
    }
}

impl Seek for EntryReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match self {
            Self::File(file) => file.seek(pos),
            Self::Temp(file) => file.seek(pos),
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
    let file = fs::File::open(zip_path).map_err(|err| {
        CliError::new(1, format!("Open zip failed {}: {err}", zip_path.display()))
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| {
        CliError::new(1, format!("Read zip failed {}: {err}", zip_path.display()))
    })?;
    let mut zip_entry = open_zip_entry(&mut archive, &entry.path, zip_path)?;
    let mut temp = TempReadableFile::new("xun-entry-zip")?;
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
    let file = fs::File::open(zip_path).map_err(|err| {
        CliError::new(1, format!("Open zip failed {}: {err}", zip_path.display()))
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| {
        CliError::new(1, format!("Read zip failed {}: {err}", zip_path.display()))
    })?;
    let mut zip_entry = open_zip_entry(&mut archive, &entry.path, zip_path)?;
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
    let content = read_7z_file(source, &entry.path)?;
    let mut temp = TempReadableFile::new("xun-entry-7z")?;
    temp.file.write_all(&content).map_err(|err| {
        CliError::new(
            1,
            format!(
                "Write temp 7z entry failed {}::{}: {err}",
                source.display(),
                entry.path
            ),
        )
    })?;
    Ok(EntryReader::Temp(temp.reopen_for_read()?))
}

fn copy_7z_entry_to_writer(entry: &SourceEntry, writer: &mut dyn Write) -> Result<(), CliError> {
    let source = entry.source_path.as_ref().ok_or_else(|| {
        CliError::new(
            1,
            format!("Missing 7z source path for entry: {}", entry.path),
        )
    })?;
    let wanted = entry.path.replace('\\', "/");
    let mut found = false;
    crate::backup::artifact::sevenz::with_archive_reader(source, |reader, _| {
        reader
            .for_each_entries(|archive_entry, data| {
                if archive_entry.is_directory() || !archive_entry.has_stream() {
                    return Ok(true);
                }
                let name = archive_entry.name().replace('\\', "/");
                if !name.eq_ignore_ascii_case(&wanted) {
                    return Ok(true);
                }
                std::io::copy(data, writer)?;
                found = true;
                Ok(false)
            })
            .map_err(|err| CliError::new(1, format!("Read 7z entry failed {}: {err}", wanted)))
    })?;
    if !found {
        return Err(CliError::new(
            1,
            format!("7z entry not found {}::{}", source.display(), entry.path),
        ));
    }
    Ok(())
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

fn open_zip_entry<'a>(
    archive: &'a mut zip::ZipArchive<fs::File>,
    path: &str,
    zip_path: &Path,
) -> Result<zip::read::ZipFile<'a>, CliError> {
    let wanted = path.replace('\\', "/");
    let mut matched_index = None;
    for index in 0..archive.len() {
        let name = {
            let candidate = archive
                .by_index(index)
                .map_err(|err| CliError::new(1, format!("Zip entry error: {err}")))?;
            candidate.name().replace('\\', "/")
        };
        if name == wanted || name.eq_ignore_ascii_case(&wanted) {
            matched_index = Some(index);
            break;
        }
    }

    if let Some(index) = matched_index {
        return archive
            .by_index(index)
            .map_err(|err| CliError::new(1, format!("Zip entry error: {err}")));
    }

    Err(CliError::new(
        1,
        format!("Zip entry not found {}::{}", zip_path.display(), path),
    ))
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
fn normalize_artifact_entry_key(path: &str) -> String {
    normalize_glob_path(path)
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
    #[cfg(feature = "xunbak")]
    use std::sync::Arc;

    use tempfile::tempdir;

    use crate::backup::artifact::entry::{SourceEntry, SourceKind};

    #[cfg(feature = "xunbak")]
    use super::{
        cached_xunbak_reader_count_for_tests, clear_xunbak_reader_cache_for_tests,
        load_cached_xunbak_reader_and_manifest,
    };
    use super::{copy_entry_to_path, open_entry_reader};

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
        let mut content = String::new();
        reader.read_to_string(&mut content).unwrap();
        assert_eq!(content, "hello 7z");
    }

    #[cfg(feature = "xunbak")]
    #[test]
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
            &crate::xunbak::writer::BackupOptions {
                codec: crate::xunbak::constants::Codec::NONE,
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
            &crate::xunbak::writer::BackupOptions {
                codec: crate::xunbak::constants::Codec::NONE,
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
            &crate::xunbak::writer::BackupOptions {
                codec: crate::xunbak::constants::Codec::NONE,
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
