use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Instant;

use serde::Serialize;

use crate::xunbak::constants::{
    BLOB_HEADER_SIZE, FOOTER_SIZE, HEADER_SIZE, RECORD_PREFIX_SIZE, RecordType,
};
use crate::xunbak::reader::ContainerReader;
use crate::xunbak::record::{RecordPrefix, compute_record_crc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VerifyLevel {
    Quick,
    Full,
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

    let mut file = match File::open(&reader.path) {
        Ok(file) => file,
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
    let data_end = reader.file_size - FOOTER_SIZE as u64;
    if file.seek(SeekFrom::Start(HEADER_SIZE as u64)).is_err() {
        return VerifyReport::failure(
            VerifyLevel::Paranoid,
            vec![issue("failed to seek to first record".to_string())],
            VerifyStats {
                elapsed_ms: start.elapsed().as_millis(),
                ..VerifyStats::default()
            },
        );
    }
    let stream_len = (reader.file_size - HEADER_SIZE as u64 - FOOTER_SIZE as u64) as usize;
    let mut bytes = vec![0u8; stream_len];
    if file.read_exact(&mut bytes).is_err() {
        return VerifyReport::failure(
            VerifyLevel::Paranoid,
            vec![issue("failed to read record stream".to_string())],
            VerifyStats {
                elapsed_ms: start.elapsed().as_millis(),
                ..VerifyStats::default()
            },
        );
    }

    let mut offset = 0usize;
    while offset + RECORD_PREFIX_SIZE <= bytes.len() {
        let prefix = match RecordPrefix::from_bytes(&bytes[offset..offset + RECORD_PREFIX_SIZE]) {
            Ok(prefix) => prefix,
            Err(err) => {
                return VerifyReport::failure(
                    VerifyLevel::Paranoid,
                    vec![VerifyIssue {
                        message: format!("{err:?}"),
                        path: None,
                        blob_id: None,
                        offset: Some(HEADER_SIZE as u64 + offset as u64),
                        volume_index: None,
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
                    volume_index: None,
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
                    volume_index: None,
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

    if HEADER_SIZE as u64 + offset as u64 != data_end {
        return VerifyReport::failure(
            VerifyLevel::Paranoid,
            vec![VerifyIssue {
                message: "record boundary discontinuity".to_string(),
                path: None,
                blob_id: None,
                offset: Some(HEADER_SIZE as u64 + offset as u64),
                volume_index: None,
                record_type: None,
            }],
            VerifyStats {
                blob_count: reader.checkpoint.blob_count,
                manifest_entries: manifest.entries.len(),
                elapsed_ms: start.elapsed().as_millis(),
            },
        );
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
