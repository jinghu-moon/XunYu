use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::backup_export::sevenz_io::read_7z_file;
use crate::backup_export::source::{SourceEntry, SourceKind};
use crate::output::CliError;
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
    let mut reader = open_entry_reader(entry)?;
    std::io::copy(&mut reader, writer).map_err(|err| {
        CliError::new(1, format!("Copy entry failed {}: {err}", entry.path))
    })?;
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
                format!("Reopen temp entry file failed {}: {err}", self.path.display()),
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
    let file = fs::File::open(zip_path).map_err(|err| {
        CliError::new(1, format!("Open zip failed {}: {err}", zip_path.display()))
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| {
        CliError::new(1, format!("Read zip failed {}: {err}", zip_path.display()))
    })?;
    let mut zip_entry = open_zip_entry(&mut archive, &entry.path, zip_path)?;
    let mut temp = TempReadableFile::new("xun-entry-zip")?;
    std::io::copy(&mut zip_entry, &mut temp).map_err(|err| {
        CliError::new(1, format!("Copy zip entry failed {}::{}: {err}", zip_path.display(), entry.path))
    })?;
    Ok(EntryReader::Temp(temp.reopen_for_read()?))
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
        CliError::new(1, format!("Write temp 7z entry failed {}::{}: {err}", source.display(), entry.path))
    })?;
    Ok(EntryReader::Temp(temp.reopen_for_read()?))
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
fn open_xunbak_reader(entry: &SourceEntry) -> Result<EntryReader, CliError> {
    use crate::xunbak::reader::ContainerReader;

    let path = entry.source_path.as_ref().ok_or_else(|| {
        CliError::new(
            1,
            format!("Missing xunbak source path for entry: {}", entry.path),
        )
    })?;
    let reader = ContainerReader::open(path).map_err(|err| CliError::new(2, err.to_string()))?;
    let manifest = reader
        .load_manifest()
        .map_err(|err| CliError::new(2, err.to_string()))?;
    let manifest_entry = manifest
        .entries
        .iter()
        .find(|candidate| candidate.path.eq_ignore_ascii_case(&entry.path))
        .ok_or_else(|| CliError::new(1, format!("xunbak entry not found: {}", entry.path)))?;
    let content = reader
        .read_and_verify_blob(manifest_entry)
        .map_err(|err| CliError::new(2, err.to_string()))?;
    let mut temp = TempReadableFile::new("xun-entry-xunbak")?;
    temp.file.write_all(&content).map_err(|err| {
        CliError::new(1, format!("Write temp xunbak entry failed {}: {err}", entry.path))
    })?;
    Ok(EntryReader::Temp(temp.reopen_for_read()?))
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

    use tempfile::tempdir;

    use crate::backup_export::source::{SourceEntry, SourceKind};

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
        crate::backup_export::sevenz_io::write_entries_to_7z(
            &[&entry],
            &archive_path,
            &crate::backup_export::sevenz_io::SevenZWriteOptions::default(),
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
        crate::backup_export::xunbak_writer::write_entries_to_xunbak(
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
}
