use chrono::TimeZone;
use std::fs;
use std::path::Path;
#[cfg(feature = "xunbak")]
use std::path::PathBuf;

use crate::backup::artifact::entry::{
    SourceEntry, SourceKind, file_attributes, metadata_created_time_ns, system_time_to_unix_ns,
};
use crate::backup::artifact::sevenz::list_7z_entries;
use crate::commands::restore_core::{is_backup_internal_name, is_backup_internal_rel_path};
use crate::output::CliError;

pub(crate) fn read_artifact_entries(path: &Path) -> Result<Vec<SourceEntry>, CliError> {
    if path.is_dir() {
        return read_dir_artifact_entries(path);
    }
    if is_zip_artifact(path) {
        return read_zip_artifact_entries(path);
    }
    if is_xunbak_artifact(path) {
        return read_xunbak_artifact_entries(path);
    }
    if is_7z_artifact(path) {
        return list_7z_entries(path);
    }
    Err(CliError::with_details(
        2,
        format!("Unsupported backup artifact: {}", path.display()),
        &[
            "Fix: Pass a directory backup, .zip, .xunbak, or .xunbak.001 path.",
            "Fix: Continue implementing 7z artifact support.",
        ],
    ))
}

fn read_dir_artifact_entries(root: &Path) -> Result<Vec<SourceEntry>, CliError> {
    let mut entries = Vec::new();
    collect_dir_entries(root, root, &mut entries)?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

fn collect_dir_entries(
    root: &Path,
    dir: &Path,
    entries: &mut Vec<SourceEntry>,
) -> Result<(), CliError> {
    let read_dir = fs::read_dir(dir)
        .map_err(|err| CliError::new(1, format!("Read dir failed {}: {err}", dir.display())))?;
    for item in read_dir {
        let item = item.map_err(|err| {
            CliError::new(1, format!("Read dir entry failed {}: {err}", dir.display()))
        })?;
        let path = item.path();
        let file_type = item.file_type().map_err(|err| {
            CliError::new(
                1,
                format!("Read file type failed {}: {err}", path.display()),
            )
        })?;
        if file_type.is_dir() {
            collect_dir_entries(root, &path, entries)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let rel = path.strip_prefix(root).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Failed to strip artifact prefix {} from {}: {err}",
                    root.display(),
                    path.display()
                ),
            )
        })?;
        if is_backup_internal_rel_path(rel) {
            continue;
        }
        let metadata = item.metadata().map_err(|err| {
            CliError::new(1, format!("Read metadata failed {}: {err}", path.display()))
        })?;
        entries.push(SourceEntry {
            path: rel.to_string_lossy().replace('\\', "/"),
            source_path: Some(path),
            size: metadata.len(),
            mtime_ns: metadata.modified().ok().map(system_time_to_unix_ns),
            created_time_ns: metadata_created_time_ns(&metadata),
            win_attributes: file_attributes(&metadata),
            content_hash: None,
            kind: SourceKind::DirArtifact,
        });
    }
    Ok(())
}

fn read_zip_artifact_entries(zip_path: &Path) -> Result<Vec<SourceEntry>, CliError> {
    let file = fs::File::open(zip_path).map_err(|err| {
        CliError::new(1, format!("Open zip failed {}: {err}", zip_path.display()))
    })?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|err| CliError::new(1, format!("Read zip failed: {err}")))?;
    let mut entries = Vec::new();

    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|err| CliError::new(1, format!("Zip entry error: {err}")))?;
        if entry.is_dir() {
            continue;
        }

        let path = entry.name().replace('\\', "/");
        if is_backup_internal_name(&path) {
            continue;
        }
        entries.push(SourceEntry {
            path,
            source_path: Some(zip_path.to_path_buf()),
            size: entry.size(),
            mtime_ns: zip_datetime_to_unix_ns(entry.last_modified()),
            created_time_ns: None,
            win_attributes: zip_entry_win_attributes(&entry),
            content_hash: None,
            kind: SourceKind::ZipArtifact,
        });
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

#[cfg(feature = "xunbak")]
fn read_xunbak_artifact_entries(path: &Path) -> Result<Vec<SourceEntry>, CliError> {
    use crate::xunbak::reader::ContainerReader;

    let reader = ContainerReader::open(path).map_err(|err| CliError::new(2, err.to_string()))?;
    let manifest = reader
        .load_manifest()
        .map_err(|err| CliError::new(2, err.to_string()))?;
    let mut entries: Vec<SourceEntry> = manifest
        .entries
        .into_iter()
        .map(|entry| SourceEntry {
            path: entry.path,
            source_path: Some(primary_xunbak_path(path)),
            size: entry.size,
            mtime_ns: Some(entry.mtime_ns),
            created_time_ns: Some(entry.created_time_ns),
            win_attributes: entry.win_attributes,
            content_hash: Some(entry.content_hash),
            kind: SourceKind::XunbakArtifact,
        })
        .collect();
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

#[cfg(not(feature = "xunbak"))]
fn read_xunbak_artifact_entries(path: &Path) -> Result<Vec<SourceEntry>, CliError> {
    Err(CliError::with_details(
        2,
        format!(
            "xunbak artifact support is not enabled in this build: {}",
            path.display()
        ),
        &["Fix: Rebuild with `--features xunbak`."],
    ))
}

#[cfg(feature = "xunbak")]
fn primary_xunbak_path(path: &Path) -> PathBuf {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".xunbak.001"))
    {
        return PathBuf::from(path.to_string_lossy().trim_end_matches(".001"));
    }
    path.to_path_buf()
}

fn is_zip_artifact(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}

fn is_7z_artifact(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("7z"))
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".7z.001"))
}

fn is_xunbak_artifact(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xunbak"))
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".xunbak.001"))
}

fn zip_datetime_to_unix_ns(datetime: Option<zip::DateTime>) -> Option<u64> {
    let datetime = datetime?;
    let dt = chrono::Utc
        .with_ymd_and_hms(
            datetime.year().into(),
            datetime.month().into(),
            datetime.day().into(),
            datetime.hour().into(),
            datetime.minute().into(),
            datetime.second().into(),
        )
        .single()?;
    Some(dt.timestamp_nanos_opt()? as u64)
}

fn zip_entry_win_attributes(entry: &zip::read::ZipFile<'_>) -> u32 {
    const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
    if entry
        .unix_mode()
        .is_some_and(|mode| mode & 0o222 == 0 && mode != 0)
    {
        FILE_ATTRIBUTE_READONLY
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use tempfile::tempdir;

    use super::read_artifact_entries;

    #[test]
    fn read_artifact_entries_lists_directory_backup_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("nested").join("keep.txt"), "keep").unwrap();
        fs::write(root.join(".bak-meta.json"), "{}").unwrap();

        let entries = read_artifact_entries(root).unwrap();
        let paths: Vec<&str> = entries.iter().map(|entry| entry.path.as_str()).collect();
        assert_eq!(paths, vec!["nested/keep.txt"]);
    }

    #[test]
    fn read_artifact_entries_lists_zip_files() {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("backup.zip");
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("a.txt", options).unwrap();
        writer.write_all(b"aaa").unwrap();
        writer.start_file(".bak-meta.json", options).unwrap();
        writer.write_all(b"{}").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        fs::write(&zip_path, bytes).unwrap();

        let entries = read_artifact_entries(&zip_path).unwrap();
        let paths: Vec<&str> = entries.iter().map(|entry| entry.path.as_str()).collect();
        assert_eq!(paths, vec!["a.txt"]);
    }

    #[cfg(feature = "xunbak")]
    #[test]
    fn read_artifact_entries_lists_xunbak_files() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("a.txt");
        fs::write(&source, "aaa").unwrap();
        let entry = crate::backup::artifact::entry::SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(source),
            size: 3,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: crate::backup::artifact::entry::SourceKind::DirArtifact,
        };
        let artifact = dir.path().join("artifact.xunbak");
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

        let entries = read_artifact_entries(&artifact).unwrap();
        let paths: Vec<&str> = entries.iter().map(|item| item.path.as_str()).collect();
        assert_eq!(paths, vec!["a.txt"]);
    }
}
