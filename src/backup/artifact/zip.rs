use std::fs;
use std::io::Write;
use std::path::Path;

use crate::backup::artifact::entry::SourceEntry;
use crate::backup::artifact::reader::copy_entry_to_writer;
use crate::output::CliError;
use chrono::{Datelike, TimeZone, Timelike, Utc};

#[derive(Clone, Copy, Debug, Default)]
pub enum ZipCompressionMethod {
    #[default]
    Auto,
    Stored,
    Deflated,
}

impl From<ZipCompressionMethod> for zip::CompressionMethod {
    fn from(method: ZipCompressionMethod) -> Self {
        match method {
            ZipCompressionMethod::Auto => zip::CompressionMethod::Deflated,
            ZipCompressionMethod::Stored => zip::CompressionMethod::Stored,
            ZipCompressionMethod::Deflated => zip::CompressionMethod::Deflated,
        }
    }
}

pub struct ZipWriteOptions {
    pub method: ZipCompressionMethod,
    pub sidecar: Option<Vec<u8>>,
}

impl Default for ZipWriteOptions {
    fn default() -> Self {
        Self {
            method: ZipCompressionMethod::Auto,
            sidecar: None,
        }
    }
}

pub struct ZipWriteSummary {
    pub entry_count: usize,
    pub bytes_in: u64,
    #[allow(dead_code)]
    pub dir_count: usize,
}

pub fn write_entries_to_zip<P: AsRef<Path>>(
    entries: &[&SourceEntry],
    destination: P,
    options: ZipWriteOptions,
) -> Result<ZipWriteSummary, CliError> {
    let path = destination.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                1,
                format!("Create zip parent failed {}: {err}", parent.display()),
            )
        })?;
    }

    let file = fs::File::create(path)
        .map_err(|err| CliError::new(1, format!("Create zip failed {}: {err}", path.display())))?;
    let mut writer = zip::ZipWriter::new(file);
    let directory_entries = collect_directory_entries(entries, options.sidecar.is_some());
    let directory_options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    for directory in &directory_entries {
        writer
            .add_directory(directory, directory_options)
            .map_err(|err| {
                CliError::new(1, format!("Add zip directory failed {directory}: {err}"))
            })?;
    }

    let mut bytes_in = 0u64;
    for entry in entries {
        let file_options = build_file_options(entry, options.method)?;
        writer
            .start_file(entry.path.replace('\\', "/"), file_options)
            .map_err(|err| {
                CliError::new(1, format!("Start zip entry failed {}: {err}", entry.path))
            })?;
        copy_entry_to_writer(entry, &mut writer)?;
        bytes_in += entry.size;
    }

    if let Some(sidecar) = options.sidecar {
        let file_options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o644);
        writer
            .start_file(crate::backup::artifact::sidecar::SIDECAR_PATH, file_options)
            .map_err(|err| CliError::new(1, format!("Start sidecar entry failed: {err}")))?;
        writer
            .write_all(&sidecar)
            .map_err(|err| CliError::new(1, format!("Write sidecar entry failed: {err}")))?;
    }

    writer.finish().map_err(|err| {
        CliError::new(1, format!("Finalize zip failed {}: {err}", path.display()))
    })?;

    Ok(ZipWriteSummary {
        entry_count: entries.len(),
        bytes_in,
        dir_count: directory_entries.len(),
    })
}

fn build_file_options(
    entry: &SourceEntry,
    method: ZipCompressionMethod,
) -> Result<zip::write::SimpleFileOptions, CliError> {
    let effective_method = match method {
        ZipCompressionMethod::Auto => choose_compression_method(entry),
        ZipCompressionMethod::Stored => ZipCompressionMethod::Stored,
        ZipCompressionMethod::Deflated => ZipCompressionMethod::Deflated,
    };
    let mut options = zip::write::SimpleFileOptions::default()
        .compression_method(effective_method.into())
        .unix_permissions(unix_permissions_for_entry(entry))
        .large_file(should_enable_zip64(entry.size));
    if let Some(last_modified_time) = zip_datetime_from_unix_ns(entry.mtime_ns) {
        options = options.last_modified_time(last_modified_time);
    }
    Ok(options)
}

fn should_enable_zip64(entry_size: u64) -> bool {
    entry_size > u32::MAX as u64
}

fn unix_permissions_for_entry(entry: &SourceEntry) -> u32 {
    if is_readonly_entry(entry) {
        0o444
    } else {
        0o644
    }
}

fn is_readonly_entry(entry: &SourceEntry) -> bool {
    #[cfg(windows)]
    {
        const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
        entry.win_attributes & FILE_ATTRIBUTE_READONLY != 0
    }
    #[cfg(not(windows))]
    {
        if let Some(source_path) = &entry.source_path
            && let Ok(metadata) = fs::metadata(source_path)
        {
            return metadata.permissions().readonly();
        }
        false
    }
}

fn choose_compression_method(entry: &SourceEntry) -> ZipCompressionMethod {
    let extension = Path::new(&entry.path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if skip_compress_for_extension(extension) {
        ZipCompressionMethod::Stored
    } else {
        ZipCompressionMethod::Deflated
    }
}

#[cfg(feature = "xunbak")]
fn skip_compress_for_extension(ext: &str) -> bool {
    crate::xunbak::codec::should_skip_compress(ext)
}

#[cfg(not(feature = "xunbak"))]
fn skip_compress_for_extension(ext: &str) -> bool {
    let ext = ext.trim().trim_start_matches('.').to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "zip"
            | "7z"
            | "rar"
            | "gz"
            | "xz"
            | "zst"
            | "lz4"
            | "bz2"
            | "br"
            | "jpg"
            | "jpeg"
            | "png"
            | "webp"
            | "mp4"
            | "mkv"
    )
}

fn collect_directory_entries(entries: &[&SourceEntry], include_sidecar_root: bool) -> Vec<String> {
    let mut directories = std::collections::BTreeSet::new();
    for entry in entries {
        let normalized = entry.path.replace('\\', "/");
        let mut current = String::new();
        for segment in normalized.split('/').filter(|segment| !segment.is_empty()) {
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(segment);
            directories.insert(format!("{current}/"));
        }
        directories.remove(&format!("{normalized}/"));
    }
    if include_sidecar_root {
        directories.insert("__xunyu__/".to_string());
    }
    directories.into_iter().collect()
}

fn zip_datetime_from_unix_ns(unix_ns: Option<u64>) -> Option<zip::DateTime> {
    let unix_ns = unix_ns?;
    let seconds = (unix_ns / 1_000_000_000) as i64;
    let datetime = Utc.timestamp_opt(seconds, 0).single()?;
    zip::DateTime::from_date_and_time(
        datetime.year().try_into().ok()?,
        datetime.month().try_into().ok()?,
        datetime.day().try_into().ok()?,
        datetime.hour().try_into().ok()?,
        datetime.minute().try_into().ok()?,
        datetime.second().try_into().ok()?,
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::{ZipCompressionMethod, ZipWriteOptions, write_entries_to_zip};
    use chrono::TimeZone;
    use std::fs;
    use std::io::Read;
    use tempfile::tempdir;

    use crate::backup::artifact::entry::{SourceEntry, SourceKind};

    #[test]
    fn zip_writer_creates_archive_for_files() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        let file_path = src.join("boxed.txt");
        fs::write(&file_path, "zipme").unwrap();
        let entry = SourceEntry {
            path: "src/boxed.txt".to_string(),
            source_path: Some(file_path.clone()),
            size: 5,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let output = dir.path().join("out.zip");

        let summary = write_entries_to_zip(&[&entry], &output, ZipWriteOptions::default()).unwrap();
        assert_eq!(summary.entry_count, 1);
        assert_eq!(summary.bytes_in, 5);
        assert_eq!(summary.dir_count, 1);

        let file = fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut zipped = archive.by_name("src/boxed.txt").unwrap();
        let mut contents = Vec::new();
        zipped.read_to_end(&mut contents).unwrap();
        assert_eq!(contents, b"zipme");
    }

    #[test]
    fn zip_writer_adds_explicit_directory_entries() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("nested").join("deep");
        fs::create_dir_all(&src).unwrap();
        let file_path = src.join("file.txt");
        fs::write(&file_path, "hello").unwrap();
        let entry = SourceEntry {
            path: "nested/deep/file.txt".to_string(),
            source_path: Some(file_path),
            size: 5,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let output = dir.path().join("dirs.zip");

        let summary = write_entries_to_zip(&[&entry], &output, ZipWriteOptions::default()).unwrap();

        assert_eq!(summary.dir_count, 2);
        let file = fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("nested/").unwrap().is_dir());
        assert!(archive.by_name("nested/deep/").unwrap().is_dir());
    }

    #[test]
    fn zip_writer_auto_uses_stored_for_precompressed_extensions() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("asset.zip");
        fs::write(&file_path, b"pretend zip bytes").unwrap();
        let entry = SourceEntry {
            path: "asset.zip".to_string(),
            source_path: Some(file_path),
            size: 16,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let output = dir.path().join("stored.zip");

        write_entries_to_zip(&[&entry], &output, ZipWriteOptions::default()).unwrap();

        let file = fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let zipped = archive.by_name("asset.zip").unwrap();
        assert_eq!(zipped.compression(), zip::CompressionMethod::Stored);
    }

    #[test]
    fn zip_writer_auto_uses_deflated_for_text_files() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("notes.txt");
        fs::write(&file_path, "hello hello hello hello").unwrap();
        let entry = SourceEntry {
            path: "notes.txt".to_string(),
            source_path: Some(file_path),
            size: 23,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let output = dir.path().join("deflated.zip");

        write_entries_to_zip(&[&entry], &output, ZipWriteOptions::default()).unwrap();

        let file = fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let zipped = archive.by_name("notes.txt").unwrap();
        assert_eq!(zipped.compression(), zip::CompressionMethod::Deflated);
    }

    #[test]
    fn zip_writer_explicit_stored_overrides_auto_strategy() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("notes.txt");
        fs::write(&file_path, "hello hello hello hello").unwrap();
        let entry = SourceEntry {
            path: "notes.txt".to_string(),
            source_path: Some(file_path),
            size: 23,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let output = dir.path().join("stored_override.zip");

        write_entries_to_zip(
            &[&entry],
            &output,
            ZipWriteOptions {
                method: ZipCompressionMethod::Stored,
                sidecar: None,
            },
        )
        .unwrap();

        let file = fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let zipped = archive.by_name("notes.txt").unwrap();
        assert_eq!(zipped.compression(), zip::CompressionMethod::Stored);
    }

    #[test]
    fn zip_writer_writes_even_second_mtime_when_present() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("dated.txt");
        fs::write(&file_path, b"date").unwrap();
        let dt = chrono::Utc
            .with_ymd_and_hms(2024, 1, 2, 3, 4, 6)
            .single()
            .unwrap();
        let entry = SourceEntry {
            path: "dated.txt".to_string(),
            source_path: Some(file_path),
            size: 4,
            mtime_ns: Some(dt.timestamp_nanos_opt().unwrap() as u64),
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let output = dir.path().join("dated.zip");

        write_entries_to_zip(
            &[&entry],
            &output,
            ZipWriteOptions {
                method: ZipCompressionMethod::Stored,
                sidecar: None,
            },
        )
        .unwrap();

        let file = fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let zipped = archive.by_name("dated.txt").unwrap();
        let modified = zipped.last_modified().unwrap();
        assert_eq!(modified.year(), 2024);
        assert_eq!(modified.month(), 1);
        assert_eq!(modified.day(), 2);
        assert_eq!(modified.hour(), 3);
        assert_eq!(modified.minute(), 4);
        assert_eq!(modified.second(), 6);
    }

    #[test]
    fn zip_writer_preserves_utf8_paths() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("说明.txt");
        fs::write(&file_path, "内容").unwrap();
        let entry = SourceEntry {
            path: "中文目录/说明.txt".to_string(),
            source_path: Some(file_path),
            size: 6,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let output = dir.path().join("utf8.zip");

        write_entries_to_zip(&[&entry], &output, ZipWriteOptions::default()).unwrap();

        let file = fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut zipped = archive.by_name("中文目录/说明.txt").unwrap();
        let mut content = String::new();
        zipped.read_to_string(&mut content).unwrap();
        assert_eq!(content, "内容");
    }

    #[test]
    fn zip_writer_marks_readonly_file_without_write_permission() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("readonly.txt");
        fs::write(&file_path, "ro").unwrap();
        let mut permissions = fs::metadata(&file_path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&file_path, permissions).unwrap();
        let entry = SourceEntry {
            path: "readonly.txt".to_string(),
            source_path: Some(file_path),
            size: 2,
            mtime_ns: None,
            created_time_ns: None,
            #[cfg(windows)]
            win_attributes: 0x0000_0001,
            #[cfg(not(windows))]
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let output = dir.path().join("readonly.zip");

        write_entries_to_zip(&[&entry], &output, ZipWriteOptions::default()).unwrap();

        let file = fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let zipped = archive.by_name("readonly.txt").unwrap();
        let mode = zipped.unix_mode().unwrap_or_default();
        assert_eq!(mode & 0o222, 0, "readonly file should not keep write bits");
    }

    #[test]
    fn zip_writer_enables_zip64_for_sizes_above_u32_max() {
        assert!(!super::should_enable_zip64(u32::MAX as u64));
        assert!(super::should_enable_zip64(u32::MAX as u64 + 1));
    }
}
