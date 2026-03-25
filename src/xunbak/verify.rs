use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use crate::xunbak::checkpoint::read_checkpoint_record;
use crate::xunbak::constants::{
    BLOB_HEADER_SIZE, FOOTER_SIZE, HEADER_SIZE, RECORD_PREFIX_SIZE, RecordType,
};
use crate::xunbak::footer::Footer;
use crate::xunbak::header::{DecodedHeader, Header};
use crate::xunbak::reader::{ContainerReader, ReaderError};
use crate::xunbak::record::{RecordPrefix, compute_record_crc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum VerifyLevel {
    #[serde(rename = "quick")]
    Quick,
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "manifest-only")]
    ManifestOnly,
    #[serde(rename = "existence-only")]
    ExistenceOnly,
    #[serde(rename = "paranoid")]
    Paranoid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct VerifyStats {
    pub blob_count: u64,
    pub manifest_entries: usize,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VerifyIssue {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_index: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_type: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VerifyReport {
    pub level: VerifyLevel,
    pub passed: bool,
    pub errors: Vec<VerifyIssue>,
    pub stats: VerifyStats,
}

impl VerifyReport {
    fn success(level: VerifyLevel, stats: VerifyStats) -> Self {
        Self {
            level,
            passed: true,
            errors: vec![],
            stats,
        }
    }

    fn failure(level: VerifyLevel, errors: Vec<VerifyIssue>, stats: VerifyStats) -> Self {
        Self {
            level,
            passed: false,
            errors,
            stats,
        }
    }
}

pub fn verify_quick(reader: &ContainerReader) -> VerifyReport {
    let start = Instant::now();
    let manifest = match reader.load_manifest() {
        Ok(manifest) => manifest,
        Err(err) => {
            return VerifyReport::failure(
                VerifyLevel::Quick,
                vec![issue(err.to_string())],
                VerifyStats {
                    elapsed_ms: start.elapsed().as_millis(),
                    ..VerifyStats::default()
                },
            );
        }
    };
    VerifyReport::success(
        VerifyLevel::Quick,
        VerifyStats {
            blob_count: reader.checkpoint.blob_count,
            manifest_entries: manifest.entries.len(),
            elapsed_ms: start.elapsed().as_millis(),
        },
    )
}

pub fn verify_quick_path(path: &Path) -> VerifyReport {
    let start = Instant::now();
    match ContainerReader::open(path) {
        Ok(reader) => verify_quick(&reader),
        Err(err) => VerifyReport::failure(
            VerifyLevel::Quick,
            vec![issue(err.to_string())],
            VerifyStats {
                elapsed_ms: start.elapsed().as_millis(),
                ..VerifyStats::default()
            },
        ),
    }
}

pub fn verify_manifest_only(reader: &ContainerReader) -> VerifyReport {
    let start = Instant::now();
    let manifest = match reader.load_manifest() {
        Ok(manifest) => manifest,
        Err(err) => {
            return VerifyReport::failure(
                VerifyLevel::ManifestOnly,
                vec![issue(err.to_string())],
                VerifyStats {
                    elapsed_ms: start.elapsed().as_millis(),
                    ..VerifyStats::default()
                },
            );
        }
    };
    VerifyReport::success(
        VerifyLevel::ManifestOnly,
        VerifyStats {
            blob_count: reader.checkpoint.blob_count,
            manifest_entries: manifest.entries.len(),
            elapsed_ms: start.elapsed().as_millis(),
        },
    )
}

pub fn verify_manifest_only_path(path: &Path) -> VerifyReport {
    let start = Instant::now();
    match ContainerReader::open(path) {
        Ok(reader) => verify_manifest_only(&reader),
        Err(err) => VerifyReport::failure(
            VerifyLevel::ManifestOnly,
            vec![issue(err.to_string())],
            VerifyStats {
                elapsed_ms: start.elapsed().as_millis(),
                ..VerifyStats::default()
            },
        ),
    }
}

pub fn verify_full(reader: &ContainerReader) -> VerifyReport {
    let start = Instant::now();
    let manifest = match reader.load_manifest() {
        Ok(manifest) => manifest,
        Err(err) => {
            return VerifyReport::failure(
                VerifyLevel::Full,
                vec![issue(err.to_string())],
                VerifyStats {
                    elapsed_ms: start.elapsed().as_millis(),
                    ..VerifyStats::default()
                },
            );
        }
    };
    for entry in &manifest.entries {
        if let Err(err) = reader.read_and_verify_blob(entry) {
            return VerifyReport::failure(
                VerifyLevel::Full,
                vec![VerifyIssue {
                    message: err.to_string(),
                    path: Some(entry.path.clone()),
                    blob_id: Some(hex_string(&entry.blob_id)),
                    offset: Some(entry.blob_offset),
                    volume_index: Some(entry.volume_index),
                    record_type: Some(RecordType::BLOB.as_u8()),
                }],
                VerifyStats {
                    blob_count: reader.checkpoint.blob_count,
                    manifest_entries: manifest.entries.len(),
                    elapsed_ms: start.elapsed().as_millis(),
                },
            );
        }
    }
    VerifyReport::success(
        VerifyLevel::Full,
        VerifyStats {
            blob_count: reader.checkpoint.blob_count,
            manifest_entries: manifest.entries.len(),
            elapsed_ms: start.elapsed().as_millis(),
        },
    )
}

pub fn verify_full_path(path: &Path) -> VerifyReport {
    let start = Instant::now();
    match ContainerReader::open(path) {
        Ok(reader) => verify_full(&reader),
        Err(err) => VerifyReport::failure(
            VerifyLevel::Full,
            vec![issue(err.to_string())],
            VerifyStats {
                elapsed_ms: start.elapsed().as_millis(),
                ..VerifyStats::default()
            },
        ),
    }
}

pub fn verify_paranoid(reader: &ContainerReader) -> VerifyReport {
    let start = Instant::now();
    let manifest = match reader.load_manifest() {
        Ok(manifest) => manifest,
        Err(err) => {
            return VerifyReport::failure(
                VerifyLevel::Paranoid,
                vec![issue(err.to_string())],
                VerifyStats {
                    elapsed_ms: start.elapsed().as_millis(),
                    ..VerifyStats::default()
                },
            );
        }
    };

    for (volume_index, volume_path) in reader.volume_paths.iter().enumerate() {
        let volume_size = match std::fs::metadata(volume_path) {
            Ok(meta) => meta.len(),
            Err(err) => {
                return VerifyReport::failure(
                    VerifyLevel::Paranoid,
                    vec![issue(err.to_string())],
                    VerifyStats {
                        elapsed_ms: start.elapsed().as_millis(),
                        ..VerifyStats::default()
                    },
                );
            }
        };
        let scan_end = if volume_index + 1 == reader.volume_paths.len() {
            volume_size - FOOTER_SIZE as u64
        } else {
            volume_size
        };
        let bytes = match read_record_region(volume_path, scan_end) {
            Ok(bytes) => bytes,
            Err(err) => {
                return VerifyReport::failure(
                    VerifyLevel::Paranoid,
                    vec![issue(err)],
                    VerifyStats {
                        elapsed_ms: start.elapsed().as_millis(),
                        ..VerifyStats::default()
                    },
                );
            }
        };

        let mut offset = 0usize;
        while offset + RECORD_PREFIX_SIZE <= bytes.len() {
            let prefix = match RecordPrefix::from_bytes(&bytes[offset..offset + RECORD_PREFIX_SIZE])
            {
                Ok(prefix) => prefix,
                Err(err) => {
                    return VerifyReport::failure(
                        VerifyLevel::Paranoid,
                        vec![VerifyIssue {
                            message: format!("{err:?}"),
                            path: None,
                            blob_id: None,
                            offset: Some(HEADER_SIZE as u64 + offset as u64),
                            volume_index: Some(volume_index as u16),
                            record_type: None,
                        }],
                        VerifyStats {
                            blob_count: reader.checkpoint.blob_count,
                            manifest_entries: manifest.entries.len(),
                            elapsed_ms: start.elapsed().as_millis(),
                        },
                    );
                }
            };
            let payload_start = offset + RECORD_PREFIX_SIZE;
            let payload_end = payload_start.saturating_add(prefix.record_len as usize);
            if payload_end > bytes.len() {
                return VerifyReport::failure(
                    VerifyLevel::Paranoid,
                    vec![VerifyIssue {
                        message: "truncated record".to_string(),
                        path: None,
                        blob_id: None,
                        offset: Some(HEADER_SIZE as u64 + offset as u64),
                        volume_index: Some(volume_index as u16),
                        record_type: Some(prefix.record_type.as_u8()),
                    }],
                    VerifyStats {
                        blob_count: reader.checkpoint.blob_count,
                        manifest_entries: manifest.entries.len(),
                        elapsed_ms: start.elapsed().as_millis(),
                    },
                );
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
                return VerifyReport::failure(
                    VerifyLevel::Paranoid,
                    vec![VerifyIssue {
                        message: "record CRC mismatch".to_string(),
                        path: None,
                        blob_id: None,
                        offset: Some(HEADER_SIZE as u64 + offset as u64),
                        volume_index: Some(volume_index as u16),
                        record_type: Some(prefix.record_type.as_u8()),
                    }],
                    VerifyStats {
                        blob_count: reader.checkpoint.blob_count,
                        manifest_entries: manifest.entries.len(),
                        elapsed_ms: start.elapsed().as_millis(),
                    },
                );
            }
            offset = payload_end;
        }

        if HEADER_SIZE as u64 + offset as u64 != scan_end {
            return VerifyReport::failure(
                VerifyLevel::Paranoid,
                vec![VerifyIssue {
                    message: "record boundary discontinuity".to_string(),
                    path: None,
                    blob_id: None,
                    offset: Some(HEADER_SIZE as u64 + offset as u64),
                    volume_index: Some(volume_index as u16),
                    record_type: None,
                }],
                VerifyStats {
                    blob_count: reader.checkpoint.blob_count,
                    manifest_entries: manifest.entries.len(),
                    elapsed_ms: start.elapsed().as_millis(),
                },
            );
        }
    }

    VerifyReport::success(
        VerifyLevel::Paranoid,
        VerifyStats {
            blob_count: reader.checkpoint.blob_count,
            manifest_entries: manifest.entries.len(),
            elapsed_ms: start.elapsed().as_millis(),
        },
    )
}

pub fn verify_paranoid_path(path: &Path) -> VerifyReport {
    let start = Instant::now();
    match ContainerReader::open(path) {
        Ok(reader) => verify_paranoid(&reader),
        Err(err) => VerifyReport::failure(
            VerifyLevel::Paranoid,
            vec![issue(err.to_string())],
            VerifyStats {
                elapsed_ms: start.elapsed().as_millis(),
                ..VerifyStats::default()
            },
        ),
    }
}

pub fn verify_existence_only_path(path: &Path) -> VerifyReport {
    let start = Instant::now();
    match inspect_existing_volume_set(path) {
        Ok(stats) => VerifyReport::success(
            VerifyLevel::ExistenceOnly,
            VerifyStats {
                elapsed_ms: start.elapsed().as_millis(),
                ..stats
            },
        ),
        Err(err) => VerifyReport::failure(
            VerifyLevel::ExistenceOnly,
            vec![err],
            VerifyStats {
                elapsed_ms: start.elapsed().as_millis(),
                ..VerifyStats::default()
            },
        ),
    }
}

fn issue(message: String) -> VerifyIssue {
    VerifyIssue {
        message,
        path: None,
        blob_id: None,
        offset: None,
        volume_index: None,
        record_type: None,
    }
}

fn hex_string(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn read_record_region(path: &PathBuf, scan_end: u64) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|err| err.to_string())?;
    file.seek(SeekFrom::Start(HEADER_SIZE as u64))
        .map_err(|err| err.to_string())?;
    let len = scan_end
        .checked_sub(HEADER_SIZE as u64)
        .ok_or_else(|| "invalid scan range".to_string())? as usize;
    let mut bytes = vec![0u8; len];
    file.read_exact(&mut bytes).map_err(|err| err.to_string())?;
    Ok(bytes)
}

fn inspect_existing_volume_set(path: &Path) -> Result<VerifyStats, VerifyIssue> {
    let primary_path = resolve_primary_path(path).map_err(|err| issue(err.to_string()))?;
    let first_header = read_header(&primary_path).map_err(|err| VerifyIssue {
        message: err.to_string(),
        path: Some(primary_path.display().to_string()),
        blob_id: None,
        offset: Some(0),
        volume_index: None,
        record_type: None,
    })?;

    if first_header.header.split.is_none() {
        return Ok(VerifyStats::default());
    }

    let volume_paths =
        discover_split_volumes(&primary_path, &first_header).map_err(|err| VerifyIssue {
            message: err.to_string(),
            path: Some(primary_path.display().to_string()),
            blob_id: None,
            offset: None,
            volume_index: None,
            record_type: None,
        })?;
    let last_index = volume_paths.len().saturating_sub(1);
    let last_path = volume_paths
        .last()
        .cloned()
        .ok_or_else(|| issue("split volume set is empty".to_string()))?;
    let last_size = std::fs::metadata(&last_path)
        .map_err(|err| VerifyIssue {
            message: err.to_string(),
            path: Some(last_path.display().to_string()),
            blob_id: None,
            offset: None,
            volume_index: Some(last_index as u16),
            record_type: None,
        })?
        .len();
    let footer = read_footer_file(&last_path, last_size).map_err(|message| VerifyIssue {
        message,
        path: Some(last_path.display().to_string()),
        blob_id: None,
        offset: None,
        volume_index: Some(last_index as u16),
        record_type: None,
    })?;
    let checkpoint =
        read_checkpoint_payload_at(&last_path, footer.checkpoint_offset).map_err(|message| {
            VerifyIssue {
                message,
                path: Some(last_path.display().to_string()),
                blob_id: None,
                offset: Some(footer.checkpoint_offset),
                volume_index: Some(last_index as u16),
                record_type: Some(RecordType::CHECKPOINT.as_u8()),
            }
        })?;
    if checkpoint.total_volumes as usize != volume_paths.len() {
        return Err(VerifyIssue {
            message: format!(
                "split volume count mismatch: expected {}, actual {}",
                checkpoint.total_volumes,
                volume_paths.len()
            ),
            path: Some(last_path.display().to_string()),
            blob_id: None,
            offset: Some(footer.checkpoint_offset),
            volume_index: Some(last_index as u16),
            record_type: Some(RecordType::CHECKPOINT.as_u8()),
        });
    }
    Ok(VerifyStats::default())
}

fn read_header(path: &Path) -> Result<DecodedHeader, ReaderError> {
    let mut file = File::open(path).map_err(|err| ReaderError::Io(err.to_string()))?;
    let mut header_bytes = [0u8; HEADER_SIZE];
    file.read_exact(&mut header_bytes)
        .map_err(|err| ReaderError::Io(err.to_string()))?;
    Header::from_bytes(&header_bytes).map_err(ReaderError::Header)
}

fn read_footer_file(path: &Path, file_size: u64) -> Result<Footer, String> {
    let mut file = File::open(path).map_err(|err| err.to_string())?;
    file.seek(SeekFrom::Start(
        file_size.saturating_sub(FOOTER_SIZE as u64),
    ))
    .map_err(|err| err.to_string())?;
    let mut footer_bytes = [0u8; FOOTER_SIZE];
    file.read_exact(&mut footer_bytes)
        .map_err(|err| err.to_string())?;
    Footer::from_bytes(&footer_bytes, file_size).map_err(|err| err.to_string())
}

fn read_checkpoint_payload_at(
    path: &Path,
    checkpoint_offset: u64,
) -> Result<crate::xunbak::checkpoint::CheckpointPayload, String> {
    let mut file = File::open(path).map_err(|err| err.to_string())?;
    file.seek(SeekFrom::Start(checkpoint_offset))
        .map_err(|err| err.to_string())?;
    read_checkpoint_record(&mut file)
        .map(|value| value.payload)
        .map_err(|err| err.to_string())
}

fn resolve_primary_path(input: &Path) -> Result<PathBuf, ReaderError> {
    if input.exists() {
        return Ok(input.to_path_buf());
    }
    let split_first = PathBuf::from(format!("{}.001", input.display()));
    if split_first.exists() {
        return Ok(split_first);
    }
    Err(ReaderError::Io(format!(
        "container not found: {}",
        input.display()
    )))
}

fn discover_split_volumes(
    first_volume: &Path,
    first_header: &DecodedHeader,
) -> Result<Vec<PathBuf>, ReaderError> {
    let base = split_base_path(first_volume);
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let prefix = base
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let expected_set_id = first_header
        .header
        .split
        .as_ref()
        .map(|split| split.set_id)
        .ok_or(ReaderError::UnrecoverableContainer)?;
    let mut volumes = Vec::new();
    for entry in std::fs::read_dir(parent).map_err(|err| ReaderError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| ReaderError::Io(err.to_string()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{prefix}.")) && name.len() == prefix.len() + 4 {
            volumes.push(entry.path());
        }
    }
    volumes.sort();
    if volumes.is_empty() {
        return Err(ReaderError::UnrecoverableContainer);
    }
    for (index, path) in volumes.iter().enumerate() {
        let header = read_header(path)?;
        let split = header
            .header
            .split
            .ok_or(ReaderError::UnrecoverableContainer)?;
        if split.volume_index as usize != index || split.set_id != expected_set_id {
            return Err(ReaderError::UnrecoverableContainer);
        }
    }
    Ok(volumes)
}

fn split_base_path(path: &Path) -> PathBuf {
    if is_split_member_path(path) {
        let raw = path.to_string_lossy();
        PathBuf::from(raw[..raw.len() - 4].to_string())
    } else {
        path.to_path_buf()
    }
}

fn is_split_member_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.len() > 4
                && name.as_bytes()[name.len() - 4] == b'.'
                && name[name.len() - 3..].chars().all(|ch| ch.is_ascii_digit())
        })
}
