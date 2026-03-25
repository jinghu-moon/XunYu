use std::fs;
use std::io::Cursor;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sevenz_rust2::encoder_options::Lzma2Options;
use sevenz_rust2::{
    ArchiveEntry, ArchiveReader, ArchiveWriter, EncoderConfiguration, EncoderMethod, Password,
    SourceReader,
};

use crate::backup::artifact::entry::SourceEntry;
use crate::backup::artifact::reader::open_entry_reader;
use crate::backup::artifact::sevenz_segmented::{MultiVolumeReader, resolve_multivolume_base};
use crate::output::CliError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SevenZMethod {
    Copy,
    Lzma2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SevenZWriteOptions {
    pub solid: bool,
    pub method: SevenZMethod,
    pub level: u32,
    pub sidecar: Option<Vec<u8>>,
}

impl Default for SevenZWriteOptions {
    fn default() -> Self {
        Self {
            solid: false,
            method: SevenZMethod::Lzma2,
            level: 1,
            sidecar: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SevenZWriteSummary {
    pub entry_count: usize,
    pub bytes_in: u64,
}

pub(crate) fn write_entries_to_7z(
    entries: &[&SourceEntry],
    destination: &Path,
    options: &SevenZWriteOptions,
) -> Result<SevenZWriteSummary, CliError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                1,
                format!("Create 7z parent failed {}: {err}", parent.display()),
            )
        })?;
    }

    let writer = ArchiveWriter::create(destination)
        .map_err(|err| CliError::new(1, format!("Create 7z failed: {err}")))?;
    let writer = write_entries_with_writer(entries, writer, options)?;
    writer
        .finish()
        .map_err(|err| CliError::new(1, format!("Finalize 7z failed: {err}")))?;

    Ok(SevenZWriteSummary {
        entry_count: entries.len(),
        bytes_in: entries.iter().map(|entry| entry.size).sum(),
    })
}

pub(crate) fn write_entries_to_7z_split(
    entries: &[&SourceEntry],
    destination_base: &Path,
    split_size: u64,
    options: &SevenZWriteOptions,
) -> Result<SevenZWriteSummary, CliError> {
    if let Some(parent) = destination_base.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                1,
                format!("Create 7z split parent failed {}: {err}", parent.display()),
            )
        })?;
    }

    let temp_single = destination_base.with_extension("tmp.single.7z");
    if temp_single.exists() {
        let _ = fs::remove_file(&temp_single);
    }
    let summary = write_entries_to_7z(entries, &temp_single, options)?;
    let split_result = split_file_to_volumes(&temp_single, destination_base, split_size);
    let _ = fs::remove_file(&temp_single);
    split_result?;
    Ok(summary)
}

fn write_entries_with_writer<W: Write + Seek>(
    entries: &[&SourceEntry],
    mut writer: ArchiveWriter<W>,
    options: &SevenZWriteOptions,
) -> Result<ArchiveWriter<W>, CliError> {
    writer.set_content_methods(vec![method_config(options.method, options.level)]);

    for directory in collect_directory_entries(entries, options.sidecar.is_some()) {
        writer
            .push_archive_entry::<&[u8]>(ArchiveEntry::new_directory(&directory), None)
            .map_err(|err| {
                CliError::new(1, format!("Write 7z directory failed {directory}: {err}"))
            })?;
    }

    if options.solid {
        let archive_entries: Vec<ArchiveEntry> = entries
            .iter()
            .map(|entry| build_archive_entry(entry))
            .collect();
        let readers: Result<Vec<SourceReader<_>>, CliError> = entries
            .iter()
            .map(|entry| open_entry_reader(entry).map(SourceReader::from))
            .collect();
        writer
            .push_archive_entries(archive_entries, readers?)
            .map_err(|err| CliError::new(1, format!("Write solid 7z failed: {err}")))?;
    } else {
        for entry in entries {
            writer
                .push_archive_entry(build_archive_entry(entry), Some(open_entry_reader(entry)?))
                .map_err(|err| {
                    CliError::new(1, format!("Write 7z entry failed {}: {err}", entry.path))
                })?;
        }
    }

    if let Some(sidecar) = &options.sidecar {
        writer
            .push_archive_entry(
                ArchiveEntry::new_file(crate::backup::artifact::sidecar::SIDECAR_PATH),
                Some(Cursor::new(sidecar.clone())),
            )
            .map_err(|err| CliError::new(1, format!("Write 7z sidecar failed: {err}")))?;
    }
    Ok(writer)
}

pub(crate) fn list_7z_entries(path: &Path) -> Result<Vec<SourceEntry>, CliError> {
    with_archive_reader(path, |reader, logical_path| {
        collect_entries_from_archive(reader.archive(), logical_path)
    })
}

pub(crate) fn read_7z_file(src: &Path, name: &str) -> Result<Vec<u8>, CliError> {
    with_archive_reader(src, |reader, _| {
        reader
            .read_file(name)
            .map_err(|err| CliError::new(1, format!("Read 7z file failed {name}: {err}")))
    })
}

pub(crate) fn restore_7z_entries<F>(
    path: &Path,
    destination: &Path,
    dry_run: bool,
    mut filter: F,
) -> Result<(usize, usize), CliError>
where
    F: FnMut(&str) -> bool,
{
    let mut restored = 0usize;
    with_archive_reader(path, |reader, _| {
        reader
            .for_each_entries(|entry, data| {
                if entry.is_directory() || !entry.has_stream() {
                    return Ok(true);
                }
                let name = entry.name().replace('\\', "/");
                if crate::commands::restore_core::is_backup_internal_name(&name) || !filter(&name) {
                    return Ok(true);
                }
                let dest = destination.join(name.replace('/', "\\"));
                if dry_run {
                    crate::output::ui_println(format_args!(
                        "DRY RUN: would restore {}",
                        dest.strip_prefix(destination).unwrap_or(&dest).display()
                    ));
                    restored += 1;
                    return Ok(true);
                }
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let mut file = fs::File::create(&dest)?;
                std::io::copy(data, &mut file)?;
                if let Err(err) = apply_archive_entry_metadata(&file, &dest, entry) {
                    return Err(std::io::Error::other(err.message).into());
                }
                restored += 1;
                Ok(true)
            })
            .map_err(|err| CliError::new(1, format!("Restore 7z failed: {err}")))?;
        Ok((restored, 0))
    })
}

pub(crate) fn restore_7z_single(
    path: &Path,
    destination: &Path,
    wanted: &str,
    dry_run: bool,
) -> Result<(usize, usize), CliError> {
    let wanted = wanted.replace('\\', "/");
    let mut found = false;
    let result = restore_7z_entries(path, destination, dry_run, |name| {
        let matched = name.eq_ignore_ascii_case(&wanted);
        if matched {
            found = true;
        }
        matched
    })?;
    if !found {
        return Err(CliError::new(
            1,
            format!("Restore failed: file not found in backup: {wanted}"),
        ));
    }
    Ok(result)
}

fn collect_entries_from_archive(
    archive: &sevenz_rust2::Archive,
    path: &Path,
) -> Result<Vec<SourceEntry>, CliError> {
    let mut entries = Vec::new();
    for entry in &archive.files {
        if entry.is_directory() || !entry.has_stream() {
            continue;
        }
        let modified = entry.has_last_modified_date.then(|| {
            crate::backup::artifact::entry::system_time_to_unix_ns(
                entry.last_modified_date().into(),
            )
        });
        let created = entry.has_creation_date.then(|| {
            crate::backup::artifact::entry::system_time_to_unix_ns(entry.creation_date().into())
        });
        let entry_path = entry.name().replace('\\', "/");
        if crate::commands::restore_core::is_backup_internal_name(&entry_path) {
            continue;
        }
        entries.push(SourceEntry {
            path: entry_path,
            source_path: Some(path.to_path_buf()),
            size: entry.size(),
            mtime_ns: modified,
            created_time_ns: created,
            win_attributes: if entry.has_windows_attributes {
                entry.windows_attributes()
            } else {
                0
            },
            content_hash: None,
            kind: crate::backup::artifact::entry::SourceKind::SevenZArtifact,
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

pub(crate) enum SevenZReaderSource {
    File(fs::File),
    Multi(MultiVolumeReader),
}

impl Read for SevenZReaderSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::File(file) => file.read(buf),
            Self::Multi(reader) => reader.read(buf),
        }
    }
}

impl Seek for SevenZReaderSource {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match self {
            Self::File(file) => file.seek(pos),
            Self::Multi(reader) => reader.seek(pos),
        }
    }
}

fn with_open_archive_reader<T>(
    source: SevenZReaderSource,
    logical_path: &Path,
    f: impl FnOnce(&mut ArchiveReader<SevenZReaderSource>, &Path) -> Result<T, CliError>,
) -> Result<T, CliError> {
    let mut reader = ArchiveReader::new(source, Password::empty()).map_err(|err| {
        CliError::new(
            1,
            format!("Open 7z failed {}: {err}", logical_path.display()),
        )
    })?;
    f(&mut reader, logical_path)
}

pub(crate) fn with_archive_reader<T>(
    path: &Path,
    f: impl FnOnce(&mut ArchiveReader<SevenZReaderSource>, &Path) -> Result<T, CliError>,
) -> Result<T, CliError> {
    if resolve_multivolume_base(path).is_some() {
        let reader = MultiVolumeReader::open(path).map_err(|err| {
            CliError::new(
                1,
                format!("Open split 7z volumes failed {}: {err}", path.display()),
            )
        })?;
        return with_open_archive_reader(SevenZReaderSource::Multi(reader), path, f);
    }

    let file = fs::File::open(path)
        .map_err(|err| CliError::new(1, format!("Open 7z failed {}: {err}", path.display())))?;
    with_open_archive_reader(SevenZReaderSource::File(file), path, f)
}

#[cfg(windows)]
fn apply_archive_entry_metadata(
    file: &fs::File,
    path: &Path,
    entry: &ArchiveEntry,
) -> Result<(), CliError> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::FileTimesExt;
    use windows_sys::Win32::Storage::FileSystem::SetFileAttributesW;

    let mut file_times = fs::FileTimes::new();
    if entry.has_last_modified_date {
        file_times = file_times.set_modified(entry.last_modified_date().into());
    }
    #[cfg(any(windows, target_os = "macos"))]
    if entry.has_creation_date {
        file_times = file_times.set_created(entry.creation_date().into());
    }
    let _ = file.set_times(file_times);

    if entry.has_windows_attributes {
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);
        let ok = unsafe { SetFileAttributesW(wide.as_ptr(), entry.windows_attributes()) };
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
fn apply_archive_entry_metadata(
    _file: &fs::File,
    _path: &Path,
    _entry: &ArchiveEntry,
) -> Result<(), CliError> {
    Ok(())
}

fn method_config(method: SevenZMethod, level: u32) -> EncoderConfiguration {
    match method {
        SevenZMethod::Copy => EncoderConfiguration::new(EncoderMethod::COPY),
        SevenZMethod::Lzma2 => Lzma2Options::from_level(level).into(),
    }
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

fn build_archive_entry(entry: &SourceEntry) -> ArchiveEntry {
    let mut archive_entry = ArchiveEntry::new_file(&entry.path.replace('\\', "/"));
    if let Some(created_time_ns) = entry.created_time_ns.and_then(system_time_from_unix_ns)
        && let Ok(value) = created_time_ns.try_into()
    {
        archive_entry.creation_date = value;
        archive_entry.has_creation_date = true;
    }
    if let Some(mtime_ns) = entry.mtime_ns.and_then(system_time_from_unix_ns)
        && let Ok(value) = mtime_ns.try_into()
    {
        archive_entry.last_modified_date = value;
        archive_entry.has_last_modified_date = true;
    }
    if entry.win_attributes != 0 {
        archive_entry.has_windows_attributes = true;
        archive_entry.windows_attributes = entry.win_attributes;
    }
    archive_entry
}

fn system_time_from_unix_ns(value: u64) -> Option<SystemTime> {
    UNIX_EPOCH.checked_add(Duration::from_nanos(value))
}

fn split_file_to_volumes(source: &Path, base_path: &Path, split_size: u64) -> Result<(), CliError> {
    let mut input = fs::File::open(source).map_err(|err| {
        CliError::new(
            1,
            format!("Open temporary 7z failed {}: {err}", source.display()),
        )
    })?;
    let mut index = 1u32;
    let mut buffer = vec![0u8; split_size as usize];

    loop {
        let read = input.read(&mut buffer).map_err(|err| {
            CliError::new(
                1,
                format!("Read temporary 7z failed {}: {err}", source.display()),
            )
        })?;
        if read == 0 {
            break;
        }
        let volume = format!("{}.{index:03}", base_path.display());
        fs::write(&volume, &buffer[..read]).map_err(|err| {
            CliError::new(1, format!("Write split 7z volume failed {volume}: {err}"))
        })?;
        index += 1;
    }
    Ok(())
}
