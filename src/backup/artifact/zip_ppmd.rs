use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Datelike, TimeZone, Timelike, Utc};
use ppmd_rust::{Ppmd8Decoder, Ppmd8Encoder, RestoreMethod};
use uuid::Uuid;

use crate::backup::artifact::entry::{SourceEntry, SourceKind};
use crate::backup::artifact::reader::open_entry_reader;
use crate::backup::artifact::sidecar::SIDECAR_PATH;
use crate::output::CliError;

const EOCD_SIGNATURE: u32 = 0x0605_4B50;
const EOCD64_SIGNATURE: u32 = 0x0606_4B50;
const EOCD64_LOCATOR_SIGNATURE: u32 = 0x0706_4B50;
const CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0201_4B50;
const LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x0403_4B50;
const ZIP64_EXTRA_FIELD_ID: u16 = 0x0001;
const UTF8_FLAG: u16 = 1 << 11;
const METHOD_STORED: u16 = 0;
const METHOD_PPMD: u16 = 98;
const SYSTEM_DOS: u16 = 0;
const DEFAULT_DATE: u16 = (1 << 5) | 1;
const DEFAULT_TIME: u16 = 0;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
const ZIP64_U16_SENTINEL: u16 = u16::MAX;
const ZIP64_U32_SENTINEL: u32 = u32::MAX;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManualZipWriteSummary {
    pub entry_count: usize,
    pub bytes_in: u64,
    pub dir_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManualZipEntry {
    pub path: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub crc32: u32,
    pub compression_method: u16,
    pub mtime_ns: Option<u64>,
    pub win_attributes: u32,
    pub local_header_offset: u64,
    pub is_dir: bool,
    flags: u16,
    external_attributes: u32,
}

pub(crate) fn write_ppmd_zip<P: AsRef<Path>>(
    entries: &[&SourceEntry],
    destination: P,
    level: Option<u32>,
    sidecar: Option<Vec<u8>>,
) -> Result<ManualZipWriteSummary, CliError> {
    let path = destination.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                1,
                format!("Create zip parent failed {}: {err}", parent.display()),
            )
        })?;
    }

    let directory_entries = collect_directory_entries(entries, sidecar.is_some());
    let mut pending = Vec::new();
    for directory in &directory_entries {
        pending.push(PendingZipRecord::directory(directory));
    }

    let mut bytes_in = 0u64;
    for entry in entries {
        pending.push(PendingZipRecord::ppmd(entry, level)?);
        bytes_in += entry.size;
    }

    if let Some(sidecar) = sidecar {
        pending.push(PendingZipRecord::stored_bytes(
            SIDECAR_PATH,
            sidecar,
            None,
            0,
            false,
        ));
    }

    let mut writer = fs::File::create(path)
        .map_err(|err| CliError::new(1, format!("Create zip failed {}: {err}", path.display())))?;
    let mut central_records = Vec::with_capacity(pending.len());
    let mut offset = 0u64;
    for record in &pending {
        let local_offset = offset;
        let local_zip64_extra =
            build_zip64_local_extra(record.uncompressed_size, record.compressed_size);
        write_u32_le(&mut writer, LOCAL_FILE_HEADER_SIGNATURE)?;
        write_u16_le(&mut writer, record.version_needed)?;
        write_u16_le(&mut writer, record.flags)?;
        write_u16_le(&mut writer, record.compression_method)?;
        write_u16_le(&mut writer, record.mod_time)?;
        write_u16_le(&mut writer, record.mod_date)?;
        write_u32_le(&mut writer, record.crc32)?;
        write_u32_le(&mut writer, zip32_size_field(record.compressed_size))?;
        write_u32_le(&mut writer, zip32_size_field(record.uncompressed_size))?;
        write_u16_le(
            &mut writer,
            record
                .name
                .len()
                .try_into()
                .map_err(|err| CliError::new(1, format!("ZIP file name too long: {err}")))?,
        )?;
        write_u16_le(
            &mut writer,
            local_zip64_extra
                .len()
                .try_into()
                .map_err(|_| CliError::new(1, "ZIP local extra too large"))?,
        )?;
        writer
            .write_all(&record.name)
            .map_err(|err| CliError::new(1, format!("Write zip name failed: {err}")))?;
        writer
            .write_all(&local_zip64_extra)
            .map_err(|err| CliError::new(1, format!("Write zip local extra failed: {err}")))?;
        record.body.write_to(&mut writer)?;

        central_records.push(CentralZipRecord {
            version_made_by: (SYSTEM_DOS << 8) | record.version_needed,
            version_needed: record.version_needed,
            flags: record.flags,
            compression_method: record.compression_method,
            mod_time: record.mod_time,
            mod_date: record.mod_date,
            crc32: record.crc32,
            compressed_size: record.compressed_size,
            uncompressed_size: record.uncompressed_size,
            external_attributes: record.external_attributes,
            local_header_offset: local_offset,
            name: record.name.clone(),
            extra: build_zip64_central_extra(
                record.uncompressed_size,
                record.compressed_size,
                local_offset,
            ),
        });

        offset +=
            30 + record.name.len() as u64 + local_zip64_extra.len() as u64 + record.body.len();
    }

    let central_start = offset;
    for record in &central_records {
        write_u32_le(&mut writer, CENTRAL_DIRECTORY_SIGNATURE)?;
        write_u16_le(&mut writer, record.version_made_by)?;
        write_u16_le(&mut writer, record.version_needed)?;
        write_u16_le(&mut writer, record.flags)?;
        write_u16_le(&mut writer, record.compression_method)?;
        write_u16_le(&mut writer, record.mod_time)?;
        write_u16_le(&mut writer, record.mod_date)?;
        write_u32_le(&mut writer, record.crc32)?;
        write_u32_le(&mut writer, zip32_size_field(record.compressed_size))?;
        write_u32_le(&mut writer, zip32_size_field(record.uncompressed_size))?;
        write_u16_le(
            &mut writer,
            record
                .name
                .len()
                .try_into()
                .map_err(|err| CliError::new(1, format!("ZIP file name too long: {err}")))?,
        )?;
        write_u16_le(
            &mut writer,
            record
                .extra
                .len()
                .try_into()
                .map_err(|_| CliError::new(1, "ZIP central extra too large"))?,
        )?;
        write_u16_le(&mut writer, 0)?;
        write_u16_le(&mut writer, 0)?;
        write_u16_le(&mut writer, 0)?;
        write_u32_le(&mut writer, record.external_attributes)?;
        write_u32_le(&mut writer, zip32_size_field(record.local_header_offset))?;
        writer
            .write_all(&record.name)
            .map_err(|err| CliError::new(1, format!("Write zip central name failed: {err}")))?;
        writer
            .write_all(&record.extra)
            .map_err(|err| CliError::new(1, format!("Write zip central extra failed: {err}")))?;
        offset += 46 + record.name.len() as u64 + record.extra.len() as u64;
    }

    let central_size = offset - central_start;
    let needs_zip64_eocd = central_records.len() >= ZIP64_U16_SENTINEL as usize
        || central_size >= ZIP64_U32_SENTINEL as u64
        || central_start >= ZIP64_U32_SENTINEL as u64;
    if needs_zip64_eocd {
        let eocd64_offset = offset;
        write_u32_le(&mut writer, EOCD64_SIGNATURE)?;
        write_u64_le(&mut writer, 44)?;
        write_u16_le(&mut writer, 45)?;
        write_u16_le(&mut writer, 45)?;
        write_u32_le(&mut writer, 0)?;
        write_u32_le(&mut writer, 0)?;
        write_u64_le(&mut writer, central_records.len() as u64)?;
        write_u64_le(&mut writer, central_records.len() as u64)?;
        write_u64_le(&mut writer, central_size)?;
        write_u64_le(&mut writer, central_start)?;
        write_u32_le(&mut writer, EOCD64_LOCATOR_SIGNATURE)?;
        write_u32_le(&mut writer, 0)?;
        write_u64_le(&mut writer, eocd64_offset)?;
        write_u32_le(&mut writer, 1)?;
    }
    write_u32_le(&mut writer, EOCD_SIGNATURE)?;
    write_u16_le(&mut writer, 0)?;
    write_u16_le(&mut writer, 0)?;
    write_u16_le(&mut writer, zip32_entry_count(central_records.len()))?;
    write_u16_le(&mut writer, zip32_entry_count(central_records.len()))?;
    write_u32_le(&mut writer, zip32_size_field(central_size))?;
    write_u32_le(&mut writer, zip32_size_field(central_start))?;
    write_u16_le(&mut writer, 0)?;
    writer.flush().map_err(|err| {
        CliError::new(1, format!("Finalize zip failed {}: {err}", path.display()))
    })?;

    Ok(ManualZipWriteSummary {
        entry_count: entries.len(),
        bytes_in,
        dir_count: directory_entries.len(),
    })
}

pub(crate) fn list_ppmd_zip_entries(path: &Path) -> Result<Vec<SourceEntry>, CliError> {
    let archive = parse_manual_zip(path)?;
    let mut entries = Vec::new();
    for entry in archive {
        if entry.is_dir || is_backup_internal_name(&entry.path) {
            continue;
        }
        entries.push(SourceEntry {
            path: entry.path,
            source_path: Some(path.to_path_buf()),
            size: entry.uncompressed_size,
            mtime_ns: entry.mtime_ns,
            created_time_ns: None,
            win_attributes: entry.win_attributes,
            content_hash: None,
            kind: SourceKind::ZipArtifact,
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

pub(crate) fn contains_ppmd_entries(path: &Path) -> Result<bool, CliError> {
    Ok(parse_manual_zip(path)?
        .into_iter()
        .any(|entry| entry.compression_method == METHOD_PPMD))
}

#[allow(dead_code)]
pub(crate) fn read_ppmd_zip_entry(path: &Path, wanted: &str) -> Result<Vec<u8>, CliError> {
    let wanted = wanted.replace('\\', "/");
    let archive = parse_manual_zip(path)?;
    let entry = archive
        .into_iter()
        .find(|entry| entry.path == wanted || entry.path.eq_ignore_ascii_case(&wanted))
        .ok_or_else(|| {
            CliError::new(
                1,
                format!("Zip entry not found {}::{}", path.display(), wanted),
            )
        })?;
    let mut content = Vec::new();
    copy_manual_zip_entry_to_writer(path, &entry, &mut content)?;
    Ok(content)
}

pub(crate) fn needs_manual_ppmd_fallback(message: &str) -> bool {
    message.contains("Compression method not supported")
}

fn is_backup_internal_name(path: &str) -> bool {
    path == SIDECAR_PATH || path.starts_with("__xunyu__/")
}

pub(crate) fn copy_ppmd_zip_entry_to_writer(
    path: &Path,
    wanted: &str,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let wanted = wanted.replace('\\', "/");
    let archive = parse_manual_zip(path)?;
    let entry = archive
        .into_iter()
        .find(|entry| entry.path == wanted || entry.path.eq_ignore_ascii_case(&wanted))
        .ok_or_else(|| {
            CliError::new(
                1,
                format!("Zip entry not found {}::{}", path.display(), wanted),
            )
        })?;
    copy_manual_zip_entry_to_writer(path, &entry, writer)
}

pub(crate) fn verify_ppmd_zip_entries(path: &Path) -> Result<(), CliError> {
    for entry in parse_manual_zip(path)? {
        if entry.is_dir || is_backup_internal_name(&entry.path) {
            continue;
        }
        copy_manual_zip_entry_to_writer(path, &entry, &mut std::io::sink())?;
    }
    Ok(())
}

fn copy_manual_zip_entry_to_writer(
    path: &Path,
    entry: &ManualZipEntry,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let mut file = fs::File::open(path)
        .map_err(|err| CliError::new(1, format!("Open zip failed {}: {err}", path.display())))?;
    let data_offset = find_data_offset(&mut file, entry.local_header_offset)?;
    file.seek(SeekFrom::Start(data_offset))
        .map_err(|err| CliError::new(1, format!("Seek zip entry failed: {err}")))?;
    match entry.compression_method {
        METHOD_STORED => {
            let mut remaining = entry.compressed_size;
            let mut crc = crc32fast::Hasher::new();
            let mut buffer = [0u8; 64 * 1024];
            while remaining > 0 {
                let read_len = usize::try_from(remaining.min(buffer.len() as u64))
                    .map_err(|_| CliError::new(1, "Stored ZIP entry chunk length overflow"))?;
                file.read_exact(&mut buffer[..read_len])
                    .map_err(|err| CliError::new(1, format!("Read zip entry failed: {err}")))?;
                crc.update(&buffer[..read_len]);
                writer
                    .write_all(&buffer[..read_len])
                    .map_err(|err| CliError::new(1, format!("Write zip entry failed: {err}")))?;
                remaining -= read_len as u64;
            }
            if crc.finalize() != entry.crc32 {
                return Err(CliError::new(
                    1,
                    format!("ZIP stored entry CRC mismatch: {}", entry.path),
                ));
            }
            Ok(())
        }
        METHOD_PPMD => decode_ppmd_entry_to_writer(&mut file, entry, writer),
        other => Err(CliError::new(
            1,
            format!("Unsupported manual zip method: {other}"),
        )),
    }
}

fn decode_ppmd_entry_to_writer(
    file: &mut fs::File,
    entry: &ManualZipEntry,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    if entry.compressed_size < 2 {
        return Err(CliError::new(
            1,
            "ZIP PPMD entry is missing parameter bytes",
        ));
    }
    let parameter = read_u16_le(file)?;
    let order = u32::from((parameter & 0x0F) + 1);
    let memory_size = 1024 * 1024 * u32::from(((parameter >> 4) & 0xFF) + 1);
    let restore_method = RestoreMethod::from((parameter >> 12) & 0x0F);
    let compressed_remaining = entry.compressed_size - 2;
    let limited = file.take(compressed_remaining);
    let mut decoder = Ppmd8Decoder::new(limited, order, memory_size, restore_method)
        .map_err(|err| CliError::new(1, format!("Create ZIP PPMD decoder failed: {err}")))?;
    let mut crc = crc32fast::Hasher::new();
    let mut written = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = decoder
            .read(&mut buffer)
            .map_err(|err| CliError::new(1, format!("Read ZIP PPMD data failed: {err}")))?;
        if read == 0 {
            break;
        }
        crc.update(&buffer[..read]);
        writer
            .write_all(&buffer[..read])
            .map_err(|err| CliError::new(1, format!("Write ZIP PPMD data failed: {err}")))?;
        written += read as u64;
    }
    if written != entry.uncompressed_size {
        return Err(CliError::new(
            1,
            format!(
                "ZIP PPMD size mismatch for {}: expected {}, got {}",
                entry.path, entry.uncompressed_size, written
            ),
        ));
    }
    if crc.finalize() != entry.crc32 {
        return Err(CliError::new(
            1,
            format!("ZIP PPMD CRC mismatch: {}", entry.path),
        ));
    }
    Ok(())
}

fn parse_manual_zip(path: &Path) -> Result<Vec<ManualZipEntry>, CliError> {
    let mut file = fs::File::open(path)
        .map_err(|err| CliError::new(1, format!("Open zip failed {}: {err}", path.display())))?;
    let file_len = file
        .metadata()
        .map_err(|err| CliError::new(1, format!("Read zip metadata failed: {err}")))?
        .len();
    let tail_len = file_len.min(66_000);
    file.seek(SeekFrom::Start(file_len - tail_len))
        .map_err(|err| CliError::new(1, format!("Seek zip tail failed: {err}")))?;
    let mut tail = vec![0u8; tail_len as usize];
    file.read_exact(&mut tail)
        .map_err(|err| CliError::new(1, format!("Read zip tail failed: {err}")))?;

    let eocd_index = tail
        .windows(4)
        .rposition(|window| window == EOCD_SIGNATURE.to_le_bytes())
        .ok_or_else(|| {
            CliError::new(
                1,
                format!("Read zip failed {}: EOCD not found", path.display()),
            )
        })?;
    let eocd = &tail[eocd_index..];
    let entry_count_32 = u16::from_le_bytes([eocd[10], eocd[11]]) as usize;
    let central_directory_size_32 =
        u32::from_le_bytes([eocd[12], eocd[13], eocd[14], eocd[15]]) as u64;
    let central_directory_offset_32 =
        u32::from_le_bytes([eocd[16], eocd[17], eocd[18], eocd[19]]) as u64;
    let (entry_count, central_directory_size, central_directory_offset) = if entry_count_32
        == ZIP64_U16_SENTINEL as usize
        || central_directory_size_32 == ZIP64_U32_SENTINEL as u64
        || central_directory_offset_32 == ZIP64_U32_SENTINEL as u64
    {
        parse_zip64_eocd(&mut file, file_len, eocd_index, tail_len)?
    } else {
        (
            entry_count_32,
            central_directory_size_32,
            central_directory_offset_32,
        )
    };

    let mut entries = Vec::with_capacity(entry_count);
    file.seek(SeekFrom::Start(central_directory_offset))
        .map_err(|err| CliError::new(1, format!("Seek central directory failed: {err}")))?;
    let mut consumed = 0u64;
    while entries.len() < entry_count && consumed < central_directory_size {
        let signature = read_u32_le(&mut file)?;
        if signature != CENTRAL_DIRECTORY_SIGNATURE {
            return Err(CliError::new(1, "Invalid ZIP central directory signature"));
        }
        let _version_made_by = read_u16_le(&mut file)?;
        let _version_needed = read_u16_le(&mut file)?;
        let flags = read_u16_le(&mut file)?;
        let compression_method = read_u16_le(&mut file)?;
        let mod_time = read_u16_le(&mut file)?;
        let mod_date = read_u16_le(&mut file)?;
        let crc32 = read_u32_le(&mut file)?;
        let compressed_size_32 = read_u32_le(&mut file)? as u64;
        let uncompressed_size_32 = read_u32_le(&mut file)? as u64;
        let name_len = read_u16_le(&mut file)? as usize;
        let extra_len = read_u16_le(&mut file)? as usize;
        let comment_len = read_u16_le(&mut file)? as usize;
        let _disk_number = read_u16_le(&mut file)?;
        let _internal_attributes = read_u16_le(&mut file)?;
        let external_attributes = read_u32_le(&mut file)?;
        let local_header_offset_32 = read_u32_le(&mut file)? as u64;

        let mut name = vec![0u8; name_len];
        file.read_exact(&mut name)
            .map_err(|err| CliError::new(1, format!("Read zip name failed: {err}")))?;
        let mut extra = vec![0u8; extra_len];
        file.read_exact(&mut extra)
            .map_err(|err| CliError::new(1, format!("Read zip extra failed: {err}")))?;
        if comment_len > 0 {
            file.seek(SeekFrom::Current(comment_len as i64))
                .map_err(|err| CliError::new(1, format!("Skip zip comment failed: {err}")))?;
        }

        let (uncompressed_size, compressed_size, local_header_offset) = parse_zip64_central_extra(
            &extra,
            uncompressed_size_32,
            compressed_size_32,
            local_header_offset_32,
        )?;

        let path = String::from_utf8_lossy(&name).replace('\\', "/");
        let is_dir = path.ends_with('/');
        let win_attributes = if external_attributes & FILE_ATTRIBUTE_READONLY != 0
            || ((external_attributes >> 16) & 0o222 == 0 && (external_attributes >> 16) != 0)
        {
            FILE_ATTRIBUTE_READONLY
        } else {
            0
        };
        entries.push(ManualZipEntry {
            path,
            compressed_size,
            uncompressed_size,
            crc32,
            compression_method,
            mtime_ns: dos_datetime_to_unix_ns(mod_date, mod_time),
            win_attributes,
            local_header_offset,
            is_dir,
            flags,
            external_attributes,
        });
        consumed += 46 + name_len as u64 + extra_len as u64 + comment_len as u64;
    }
    Ok(entries)
}

fn find_data_offset(file: &mut fs::File, local_header_offset: u64) -> Result<u64, CliError> {
    file.seek(SeekFrom::Start(local_header_offset))
        .map_err(|err| CliError::new(1, format!("Seek zip local header failed: {err}")))?;
    let signature = read_u32_le(file)?;
    if signature != LOCAL_FILE_HEADER_SIGNATURE {
        return Err(CliError::new(1, "Invalid ZIP local header signature"));
    }
    let _version_needed = read_u16_le(file)?;
    let flags = read_u16_le(file)?;
    if flags & (1 << 3) != 0 {
        return Err(CliError::new(
            1,
            "ZIP ppmd backend does not support data descriptors yet",
        ));
    }
    let _compression_method = read_u16_le(file)?;
    let _mod_time = read_u16_le(file)?;
    let _mod_date = read_u16_le(file)?;
    let _crc32 = read_u32_le(file)?;
    let _compressed_size = read_u32_le(file)?;
    let _uncompressed_size = read_u32_le(file)?;
    let name_len = read_u16_le(file)? as u64;
    let extra_len = read_u16_le(file)? as u64;
    Ok(local_header_offset + 30 + name_len + extra_len)
}

fn read_u16_le<R: Read>(reader: &mut R) -> Result<u16, CliError> {
    let mut bytes = [0u8; 2];
    reader
        .read_exact(&mut bytes)
        .map_err(|err| CliError::new(1, format!("Read u16 failed: {err}")))?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32_le<R: Read>(reader: &mut R) -> Result<u32, CliError> {
    let mut bytes = [0u8; 4];
    reader
        .read_exact(&mut bytes)
        .map_err(|err| CliError::new(1, format!("Read u32 failed: {err}")))?;
    Ok(u32::from_le_bytes(bytes))
}

fn write_u16_le<W: Write>(writer: &mut W, value: u16) -> Result<(), CliError> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(|err| CliError::new(1, format!("Write u16 failed: {err}")))
}

fn write_u32_le<W: Write>(writer: &mut W, value: u32) -> Result<(), CliError> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(|err| CliError::new(1, format!("Write u32 failed: {err}")))
}

fn write_u64_le<W: Write>(writer: &mut W, value: u64) -> Result<(), CliError> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(|err| CliError::new(1, format!("Write u64 failed: {err}")))
}

fn read_u64_le<R: Read>(reader: &mut R) -> Result<u64, CliError> {
    let mut bytes = [0u8; 8];
    reader
        .read_exact(&mut bytes)
        .map_err(|err| CliError::new(1, format!("Read u64 failed: {err}")))?;
    Ok(u64::from_le_bytes(bytes))
}

fn zip32_size_field(value: u64) -> u32 {
    if value >= ZIP64_U32_SENTINEL as u64 {
        ZIP64_U32_SENTINEL
    } else {
        value as u32
    }
}

fn zip32_entry_count(value: usize) -> u16 {
    if value >= ZIP64_U16_SENTINEL as usize {
        ZIP64_U16_SENTINEL
    } else {
        value as u16
    }
}

fn build_zip64_local_extra(uncompressed_size: u64, compressed_size: u64) -> Vec<u8> {
    if uncompressed_size < ZIP64_U32_SENTINEL as u64 && compressed_size < ZIP64_U32_SENTINEL as u64
    {
        return Vec::new();
    }

    let mut extra = Vec::with_capacity(20);
    extra.extend_from_slice(&ZIP64_EXTRA_FIELD_ID.to_le_bytes());
    extra.extend_from_slice(&16u16.to_le_bytes());
    extra.extend_from_slice(&uncompressed_size.to_le_bytes());
    extra.extend_from_slice(&compressed_size.to_le_bytes());
    extra
}

fn build_zip64_central_extra(
    uncompressed_size: u64,
    compressed_size: u64,
    local_header_offset: u64,
) -> Vec<u8> {
    let need_uncompressed = uncompressed_size >= ZIP64_U32_SENTINEL as u64;
    let need_compressed = compressed_size >= ZIP64_U32_SENTINEL as u64;
    let need_offset = local_header_offset >= ZIP64_U32_SENTINEL as u64;
    if !need_uncompressed && !need_compressed && !need_offset {
        return Vec::new();
    }

    let mut payload = Vec::new();
    if need_uncompressed {
        payload.extend_from_slice(&uncompressed_size.to_le_bytes());
    }
    if need_compressed {
        payload.extend_from_slice(&compressed_size.to_le_bytes());
    }
    if need_offset {
        payload.extend_from_slice(&local_header_offset.to_le_bytes());
    }

    let mut extra = Vec::with_capacity(payload.len() + 4);
    extra.extend_from_slice(&ZIP64_EXTRA_FIELD_ID.to_le_bytes());
    extra.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    extra.extend_from_slice(&payload);
    extra
}

fn parse_zip64_central_extra(
    extra: &[u8],
    uncompressed_size_32: u64,
    compressed_size_32: u64,
    local_header_offset_32: u64,
) -> Result<(u64, u64, u64), CliError> {
    let mut uncompressed_size = uncompressed_size_32;
    let mut compressed_size = compressed_size_32;
    let mut local_header_offset = local_header_offset_32;
    if uncompressed_size_32 < ZIP64_U32_SENTINEL as u64
        && compressed_size_32 < ZIP64_U32_SENTINEL as u64
        && local_header_offset_32 < ZIP64_U32_SENTINEL as u64
    {
        return Ok((uncompressed_size, compressed_size, local_header_offset));
    }

    let mut cursor = Cursor::new(extra);
    while (cursor.position() as usize) + 4 <= extra.len() {
        let field_id = read_u16_le(&mut cursor)?;
        let field_len = read_u16_le(&mut cursor)? as usize;
        let start = cursor.position() as usize;
        let end = start + field_len;
        if end > extra.len() {
            return Err(CliError::new(1, "ZIP64 extra field is truncated"));
        }
        if field_id == ZIP64_EXTRA_FIELD_ID {
            if uncompressed_size_32 >= ZIP64_U32_SENTINEL as u64 {
                uncompressed_size = read_u64_le(&mut cursor)?;
            }
            if compressed_size_32 >= ZIP64_U32_SENTINEL as u64 {
                compressed_size = read_u64_le(&mut cursor)?;
            }
            if local_header_offset_32 >= ZIP64_U32_SENTINEL as u64 {
                local_header_offset = read_u64_le(&mut cursor)?;
            }
            return Ok((uncompressed_size, compressed_size, local_header_offset));
        }
        cursor.set_position(end as u64);
    }

    Err(CliError::new(
        1,
        "ZIP64 extra field is missing for saturated ZIP32 header values",
    ))
}

fn parse_zip64_eocd(
    file: &mut fs::File,
    file_len: u64,
    eocd_index: usize,
    tail_len: u64,
) -> Result<(usize, u64, u64), CliError> {
    let eocd_absolute = file_len - tail_len + eocd_index as u64;
    if eocd_absolute < 20 {
        return Err(CliError::new(1, "ZIP64 locator does not fit before EOCD"));
    }
    file.seek(SeekFrom::Start(eocd_absolute - 20))
        .map_err(|err| CliError::new(1, format!("Seek ZIP64 locator failed: {err}")))?;
    if read_u32_le(file)? != EOCD64_LOCATOR_SIGNATURE {
        return Err(CliError::new(1, "ZIP64 locator signature is missing"));
    }
    let _disk_with_central_directory = read_u32_le(file)?;
    let eocd64_offset = read_u64_le(file)?;
    let _number_of_disks = read_u32_le(file)?;

    file.seek(SeekFrom::Start(eocd64_offset))
        .map_err(|err| CliError::new(1, format!("Seek ZIP64 EOCD failed: {err}")))?;
    if read_u32_le(file)? != EOCD64_SIGNATURE {
        return Err(CliError::new(1, "ZIP64 EOCD signature is missing"));
    }
    let _record_size = read_u64_le(file)?;
    let _version_made_by = read_u16_le(file)?;
    let _version_needed = read_u16_le(file)?;
    let _disk_number = read_u32_le(file)?;
    let _disk_with_central_directory = read_u32_le(file)?;
    let _number_of_files_on_this_disk = read_u64_le(file)?;
    let number_of_files = read_u64_le(file)?;
    let central_directory_size = read_u64_le(file)?;
    let central_directory_offset = read_u64_le(file)?;

    let number_of_files: usize = number_of_files
        .try_into()
        .map_err(|_| CliError::new(1, "ZIP64 entry count exceeds current usize"))?;
    Ok((
        number_of_files,
        central_directory_size,
        central_directory_offset,
    ))
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

fn dos_datetime_to_unix_ns(date: u16, time: u16) -> Option<u64> {
    let year = 1980 + i32::from((date >> 9) & 0x7F);
    let month = u32::from((date >> 5) & 0x0F);
    let day = u32::from(date & 0x1F);
    let hour = u32::from((time >> 11) & 0x1F);
    let minute = u32::from((time >> 5) & 0x3F);
    let second = u32::from(time & 0x1F) * 2;
    let dt = Utc
        .with_ymd_and_hms(year, month.max(1), day.max(1), hour, minute, second.min(59))
        .single()?;
    Some(dt.timestamp_nanos_opt()? as u64)
}

fn dos_time_date_from_unix_ns(unix_ns: Option<u64>) -> (u16, u16) {
    let Some(unix_ns) = unix_ns else {
        return (DEFAULT_TIME, DEFAULT_DATE);
    };
    let seconds = (unix_ns / 1_000_000_000) as i64;
    let Some(datetime) = Utc.timestamp_opt(seconds, 0).single() else {
        return (DEFAULT_TIME, DEFAULT_DATE);
    };
    if datetime.year() < 1980 {
        return (DEFAULT_TIME, DEFAULT_DATE);
    }
    let date = (((datetime.year() - 1980) as u16) << 9)
        | ((datetime.month() as u16) << 5)
        | datetime.day() as u16;
    let time = ((datetime.hour() as u16) << 11)
        | ((datetime.minute() as u16) << 5)
        | ((datetime.second() as u16) / 2);
    (time, date)
}

fn normalize_ppmd_level(level: Option<u32>) -> Result<u32, CliError> {
    let level = level.unwrap_or(7);
    if !(1..=9).contains(&level) {
        return Err(CliError::with_details(
            2,
            format!("ZIP ppmd level {level} is invalid"),
            &["Fix: Use `--level 1..9` with `--method ppmd`."],
        ));
    }
    Ok(level)
}

fn create_temp_payload_path(prefix: &str) -> Result<PathBuf, CliError> {
    let mut path = std::env::temp_dir();
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    path.push(format!("{prefix}-{}-{millis}.bin", Uuid::new_v4()));
    Ok(path)
}

struct PendingZipRecord {
    name: Vec<u8>,
    body: PendingBody,
    compression_method: u16,
    version_needed: u16,
    flags: u16,
    mod_time: u16,
    mod_date: u16,
    crc32: u32,
    compressed_size: u64,
    uncompressed_size: u64,
    external_attributes: u32,
}

enum PendingBody {
    Inline(Vec<u8>),
    Temp(TempBody),
}

impl PendingBody {
    fn len(&self) -> u64 {
        match self {
            Self::Inline(data) => data.len() as u64,
            Self::Temp(body) => body.len,
        }
    }

    fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), CliError> {
        match self {
            Self::Inline(data) => writer
                .write_all(data)
                .map_err(|err| CliError::new(1, format!("Write zip payload failed: {err}"))),
            Self::Temp(body) => body.copy_to(writer),
        }
    }
}

struct TempBody {
    path: PathBuf,
    len: u64,
}

impl TempBody {
    fn new(path: PathBuf, len: u64) -> Self {
        Self { path, len }
    }

    fn copy_to<W: Write>(&self, writer: &mut W) -> Result<(), CliError> {
        let mut file = fs::File::open(&self.path).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Open temp zip payload failed {}: {err}",
                    self.path.display()
                ),
            )
        })?;
        std::io::copy(&mut file, writer).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Copy temp zip payload failed {}: {err}",
                    self.path.display()
                ),
            )
        })?;
        Ok(())
    }
}

impl Drop for TempBody {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl PendingZipRecord {
    fn directory(path: &str) -> Self {
        let name = path.replace('\\', "/").into_bytes();
        let flags = if name.is_ascii() { 0 } else { UTF8_FLAG };
        let external_attributes = (0o040755u32 << 16) | FILE_ATTRIBUTE_DIRECTORY;
        Self {
            name,
            body: PendingBody::Inline(Vec::new()),
            compression_method: METHOD_STORED,
            version_needed: 20,
            flags,
            mod_time: DEFAULT_TIME,
            mod_date: DEFAULT_DATE,
            crc32: 0,
            compressed_size: 0,
            uncompressed_size: 0,
            external_attributes,
        }
    }

    fn stored_bytes(
        path: &str,
        data: Vec<u8>,
        mtime_ns: Option<u64>,
        win_attributes: u32,
        readonly: bool,
    ) -> Self {
        let name = path.replace('\\', "/").into_bytes();
        let flags = if name.is_ascii() { 0 } else { UTF8_FLAG };
        let crc32 = crc32fast::hash(&data);
        let (mod_time, mod_date) = dos_time_date_from_unix_ns(mtime_ns);
        let external_attributes = if readonly || win_attributes & FILE_ATTRIBUTE_READONLY != 0 {
            (0o100444u32 << 16) | FILE_ATTRIBUTE_READONLY
        } else {
            0o100644u32 << 16
        };
        Self {
            name,
            compressed_size: data.len() as u64,
            uncompressed_size: data.len() as u64,
            body: PendingBody::Inline(data),
            compression_method: METHOD_STORED,
            version_needed: 10,
            flags,
            mod_time,
            mod_date,
            crc32,
            external_attributes,
        }
    }

    fn ppmd(entry: &SourceEntry, level: Option<u32>) -> Result<Self, CliError> {
        const ORDERS: [u32; 10] = [0, 4, 5, 6, 7, 8, 9, 10, 11, 12];

        let level = normalize_ppmd_level(level)?;
        let order = ORDERS[level as usize];
        let memory_size = 1 << (level + 19);
        let memory_size_mb = memory_size / 1024 / 1024;
        let parameter: u16 = (order as u16 - 1)
            + ((memory_size_mb - 1) << 4) as u16
            + ((RestoreMethod::Restart as u16) << 12);

        let temp_path = create_temp_payload_path("xun-zip-ppmd")?;
        let mut temp = fs::File::create(&temp_path).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Create temp ZIP PPMD payload failed {}: {err}",
                    temp_path.display()
                ),
            )
        })?;
        temp.write_all(&parameter.to_le_bytes()).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Write ZIP PPMD parameter header failed {}: {err}",
                    temp_path.display()
                ),
            )
        })?;
        let mut encoder = Ppmd8Encoder::new(temp, order, memory_size, RestoreMethod::Restart)
            .map_err(|err| CliError::new(1, format!("Create ZIP PPMD encoder failed: {err}")))?;
        let mut reader = open_entry_reader(entry)?;
        let mut crc = crc32fast::Hasher::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut uncompressed_size = 0u64;
        loop {
            let read = reader.read(&mut buffer).map_err(|err| {
                CliError::new(
                    1,
                    format!("Read ZIP PPMD source failed {}: {err}", entry.path),
                )
            })?;
            if read == 0 {
                break;
            }
            crc.update(&buffer[..read]);
            encoder
                .write_all(&buffer[..read])
                .map_err(|err| CliError::new(1, format!("Write ZIP PPMD payload failed: {err}")))?;
            uncompressed_size += read as u64;
        }
        let mut temp = encoder
            .finish(true)
            .map_err(|err| CliError::new(1, format!("Finalize ZIP PPMD payload failed: {err}")))?;
        let compressed_size = temp
            .stream_position()
            .map_err(|err| CliError::new(1, format!("Read ZIP PPMD payload size failed: {err}")))?;
        temp.flush().map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Flush ZIP PPMD payload failed {}: {err}",
                    temp_path.display()
                ),
            )
        })?;
        drop(temp);

        let (mod_time, mod_date) = dos_time_date_from_unix_ns(entry.mtime_ns);
        let mut external_attributes = 0o100644u32 << 16;
        if entry.win_attributes & FILE_ATTRIBUTE_READONLY != 0 {
            external_attributes = (0o100444u32 << 16) | FILE_ATTRIBUTE_READONLY;
        }
        let name = entry.path.replace('\\', "/").into_bytes();
        Ok(Self {
            flags: if name.is_ascii() { 0 } else { UTF8_FLAG },
            name,
            crc32: crc.finalize(),
            compressed_size,
            uncompressed_size,
            body: PendingBody::Temp(TempBody::new(temp_path, compressed_size)),
            compression_method: METHOD_PPMD,
            version_needed: 45,
            mod_time,
            mod_date,
            external_attributes,
        })
    }
}

struct CentralZipRecord {
    version_made_by: u16,
    version_needed: u16,
    flags: u16,
    compression_method: u16,
    mod_time: u16,
    mod_date: u16,
    crc32: u32,
    compressed_size: u64,
    uncompressed_size: u64,
    external_attributes: u32,
    local_header_offset: u64,
    name: Vec<u8>,
    extra: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::{
        EOCD64_LOCATOR_SIGNATURE, EOCD64_SIGNATURE, LOCAL_FILE_HEADER_SIGNATURE,
        ZIP64_EXTRA_FIELD_ID, build_zip64_central_extra, build_zip64_local_extra, find_data_offset,
        parse_zip64_central_extra, parse_zip64_eocd, read_u16_le, write_u16_le, write_u32_le,
        write_u64_le,
    };
    use std::fs;
    use std::io::{Cursor, Write};
    use tempfile::tempdir;

    #[test]
    fn zip64_central_extra_roundtrips_sizes_and_offset() {
        let extra = build_zip64_central_extra(
            u32::MAX as u64 + 10,
            u32::MAX as u64 + 20,
            u32::MAX as u64 + 30,
        );
        assert_eq!(
            read_u16_le(&mut Cursor::new(&extra[0..2])).unwrap(),
            ZIP64_EXTRA_FIELD_ID
        );

        let parsed =
            parse_zip64_central_extra(&extra, u32::MAX as u64, u32::MAX as u64, u32::MAX as u64)
                .unwrap();
        assert_eq!(
            parsed,
            (
                u32::MAX as u64 + 10,
                u32::MAX as u64 + 20,
                u32::MAX as u64 + 30
            )
        );
    }

    #[test]
    fn find_data_offset_skips_zip64_local_extra() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.bin");
        let mut file = fs::File::create(&path).unwrap();
        let extra = build_zip64_local_extra(u32::MAX as u64 + 1, u32::MAX as u64 + 2);
        write_u32_le(&mut file, LOCAL_FILE_HEADER_SIGNATURE).unwrap();
        write_u16_le(&mut file, 45).unwrap();
        write_u16_le(&mut file, 0).unwrap();
        write_u16_le(&mut file, 98).unwrap();
        write_u16_le(&mut file, 0).unwrap();
        write_u16_le(&mut file, 0).unwrap();
        write_u32_le(&mut file, 0).unwrap();
        write_u32_le(&mut file, u32::MAX).unwrap();
        write_u32_le(&mut file, u32::MAX).unwrap();
        write_u16_le(&mut file, 1).unwrap();
        write_u16_le(&mut file, extra.len() as u16).unwrap();
        file.write_all(b"a").unwrap();
        file.write_all(&extra).unwrap();
        file.write_all(b"payload").unwrap();
        file.flush().unwrap();

        let mut file = fs::File::open(&path).unwrap();
        let offset = find_data_offset(&mut file, 0).unwrap();
        assert_eq!(offset, 30 + 1 + extra.len() as u64);
    }

    #[test]
    fn parse_zip64_eocd_reads_synthetic_footer() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("zip64-footer.bin");
        let mut file = fs::File::create(&path).unwrap();

        write_u32_le(&mut file, EOCD64_SIGNATURE).unwrap();
        write_u64_le(&mut file, 44).unwrap();
        write_u16_le(&mut file, 45).unwrap();
        write_u16_le(&mut file, 45).unwrap();
        write_u32_le(&mut file, 0).unwrap();
        write_u32_le(&mut file, 0).unwrap();
        write_u64_le(&mut file, 70_000).unwrap();
        write_u64_le(&mut file, 70_000).unwrap();
        write_u64_le(&mut file, u32::MAX as u64 + 123).unwrap();
        write_u64_le(&mut file, u32::MAX as u64 + 456).unwrap();

        write_u32_le(&mut file, EOCD64_LOCATOR_SIGNATURE).unwrap();
        write_u32_le(&mut file, 0).unwrap();
        write_u64_le(&mut file, 0).unwrap();
        write_u32_le(&mut file, 1).unwrap();

        write_u32_le(&mut file, 0x0605_4B50).unwrap();
        write_u16_le(&mut file, 0).unwrap();
        write_u16_le(&mut file, 0).unwrap();
        write_u16_le(&mut file, u16::MAX).unwrap();
        write_u16_le(&mut file, u16::MAX).unwrap();
        write_u32_le(&mut file, u32::MAX).unwrap();
        write_u32_le(&mut file, u32::MAX).unwrap();
        write_u16_le(&mut file, 0).unwrap();
        file.flush().unwrap();

        let mut file = fs::File::open(&path).unwrap();
        let file_len = file.metadata().unwrap().len();
        let parsed =
            parse_zip64_eocd(&mut file, file_len, (file_len - 22) as usize, file_len).unwrap();
        assert_eq!(
            parsed,
            (70_000, u32::MAX as u64 + 123, u32::MAX as u64 + 456)
        );
    }
}
