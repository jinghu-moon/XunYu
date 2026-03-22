use std::collections::HashSet;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use crate::xunbak::constants::{Codec, RecordType};
use crate::xunbak::record::{RecordPrefix, RecordScanError, compute_record_crc};

const MANIFEST_PREFIX_SIZE: usize = 4;
const WINDOWS_EPOCH_DIFF_100NS: i128 = 116_444_736_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestCodec(u8);

impl ManifestCodec {
    pub const JSON: Self = Self(0x00);
    pub const MSGPACK: Self = Self(0x01);
    pub const BINCODE: Self = Self(0x02);

    pub const fn from_u8(value: u8) -> Self {
        Self(value)
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestType(u8);

impl ManifestType {
    pub const FULL: Self = Self(0x00);
    pub const DELTA: Self = Self(0x01);

    pub const fn from_u8(value: u8) -> Self {
        Self(value)
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestPrefix {
    pub manifest_codec: ManifestCodec,
    pub manifest_type: ManifestType,
    pub manifest_version: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestPart {
    pub part_index: u32,
    #[serde(
        serialize_with = "serialize_hash32",
        deserialize_with = "deserialize_hash32"
    )]
    pub blob_id: [u8; 32],
    #[serde(
        serialize_with = "serialize_codec",
        deserialize_with = "deserialize_codec"
    )]
    pub codec: Codec,
    pub raw_size: u64,
    pub stored_size: u64,
    pub blob_offset: u64,
    pub blob_len: u64,
    pub volume_index: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    #[serde(
        serialize_with = "serialize_hash32",
        deserialize_with = "deserialize_hash32"
    )]
    pub blob_id: [u8; 32],
    #[serde(
        serialize_with = "serialize_hash32",
        deserialize_with = "deserialize_hash32"
    )]
    pub content_hash: [u8; 32],
    pub size: u64,
    pub mtime_ns: u64,
    pub created_time_ns: u64,
    pub win_attributes: u32,
    #[serde(
        serialize_with = "serialize_codec",
        deserialize_with = "deserialize_codec"
    )]
    pub codec: Codec,
    pub blob_offset: u64,
    pub blob_len: u64,
    pub volume_index: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<ManifestPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestBody {
    pub snapshot_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_snapshot_id: Option<String>,
    pub created_at: u64,
    pub source_root: String,
    pub snapshot_context: serde_json::Value,
    pub file_count: u64,
    pub total_raw_bytes: u64,
    pub entries: Vec<ManifestEntry>,
    pub removed: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ManifestWriteResult {
    pub prefix: ManifestPrefix,
    pub record_len: u64,
    pub body_len: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ManifestReadResult {
    pub prefix: ManifestPrefix,
    pub body: ManifestBody,
    pub record_len: u64,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ManifestError {
    #[error("manifest prefix too short: {actual}")]
    PrefixTooShort { actual: usize },
    #[error("record prefix error: {0:?}")]
    RecordPrefix(RecordScanError),
    #[error("unexpected record type: {0:#04x}")]
    UnexpectedRecordType(u8),
    #[error("manifest CRC mismatch")]
    ManifestCrcMismatch,
    #[error("unsupported manifest codec: {0:#04x}")]
    UnsupportedManifestCodec(u8),
    #[error("manifest parse error: {0}")]
    ManifestParseError(String),
    #[error("empty path")]
    EmptyPath,
    #[error("path case conflict: {0}")]
    PathCaseConflict(String),
    #[error("I/O error: {0}")]
    Io(String),
}

impl ManifestPrefix {
    pub fn to_bytes(self) -> [u8; MANIFEST_PREFIX_SIZE] {
        [
            self.manifest_codec.as_u8(),
            self.manifest_type.as_u8(),
            self.manifest_version.to_le_bytes()[0],
            self.manifest_version.to_le_bytes()[1],
        ]
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ManifestError> {
        if bytes.len() < MANIFEST_PREFIX_SIZE {
            return Err(ManifestError::PrefixTooShort {
                actual: bytes.len(),
            });
        }
        Ok(Self {
            manifest_codec: ManifestCodec::from_u8(bytes[0]),
            manifest_type: ManifestType::from_u8(bytes[1]),
            manifest_version: u16::from_le_bytes(bytes[2..4].try_into().expect("len checked")),
        })
    }
}

pub fn write_manifest_record<W: Write>(
    writer: &mut W,
    prefix: ManifestPrefix,
    body: &ManifestBody,
) -> Result<ManifestWriteResult, ManifestError> {
    let prefix_bytes = prefix.to_bytes();
    let body_bytes = match prefix.manifest_codec {
        codec if codec == ManifestCodec::JSON => serde_json::to_vec(body)
            .map_err(|err| ManifestError::ManifestParseError(err.to_string()))?,
        codec => return Err(ManifestError::UnsupportedManifestCodec(codec.as_u8())),
    };

    let mut payload = prefix_bytes.to_vec();
    payload.extend_from_slice(&body_bytes);
    let record_len = payload.len() as u64;
    let record_prefix = RecordPrefix {
        record_type: RecordType::MANIFEST,
        record_len,
        record_crc: compute_record_crc(RecordType::MANIFEST, record_len.to_le_bytes(), &payload),
    };

    writer
        .write_all(&record_prefix.to_bytes())
        .map_err(|err| ManifestError::Io(err.to_string()))?;
    writer
        .write_all(&payload)
        .map_err(|err| ManifestError::Io(err.to_string()))?;

    Ok(ManifestWriteResult {
        prefix,
        record_len,
        body_len: body_bytes.len(),
    })
}

pub fn read_manifest_record<R: Read>(reader: &mut R) -> Result<ManifestReadResult, ManifestError> {
    let mut prefix_bytes = [0u8; crate::xunbak::constants::RECORD_PREFIX_SIZE];
    reader
        .read_exact(&mut prefix_bytes)
        .map_err(|err| ManifestError::Io(err.to_string()))?;
    let record_prefix =
        RecordPrefix::from_bytes(&prefix_bytes).map_err(ManifestError::RecordPrefix)?;
    if record_prefix.record_type != RecordType::MANIFEST {
        return Err(ManifestError::UnexpectedRecordType(
            record_prefix.record_type.as_u8(),
        ));
    }

    let mut payload = vec![0u8; record_prefix.record_len as usize];
    reader
        .read_exact(&mut payload)
        .map_err(|err| ManifestError::Io(err.to_string()))?;
    let crc = compute_record_crc(
        RecordType::MANIFEST,
        record_prefix.record_len.to_le_bytes(),
        &payload,
    );
    if crc != record_prefix.record_crc {
        return Err(ManifestError::ManifestCrcMismatch);
    }

    let prefix = ManifestPrefix::from_bytes(&payload[..MANIFEST_PREFIX_SIZE])?;
    let body = match prefix.manifest_codec {
        codec if codec == ManifestCodec::JSON => {
            serde_json::from_slice(&payload[MANIFEST_PREFIX_SIZE..])
                .map_err(|err| ManifestError::ManifestParseError(err.to_string()))?
        }
        codec => return Err(ManifestError::UnsupportedManifestCodec(codec.as_u8())),
    };

    Ok(ManifestReadResult {
        prefix,
        body,
        record_len: record_prefix.record_len,
    })
}

pub fn normalize_path(input: &str) -> Result<String, ManifestError> {
    let mut normalized = input.trim().replace('\\', "/");
    if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        normalized = normalized[2..].to_string();
    }
    normalized = normalized.trim_start_matches('/').to_string();
    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }
    if normalized.is_empty() {
        return Err(ManifestError::EmptyPath);
    }
    Ok(normalized)
}

pub fn detect_case_conflicts(paths: &[String]) -> Result<(), ManifestError> {
    let mut seen = HashSet::new();
    for path in paths {
        let key = path.to_ascii_lowercase();
        if !seen.insert(key) {
            return Err(ManifestError::PathCaseConflict(path.clone()));
        }
    }
    Ok(())
}

pub fn filetime_to_unix_ns(filetime_100ns: u64) -> i128 {
    (filetime_100ns as i128 - WINDOWS_EPOCH_DIFF_100NS) * 100
}

pub fn unix_ns_to_filetime(unix_ns: i128) -> u64 {
    ((unix_ns / 100) + WINDOWS_EPOCH_DIFF_100NS) as u64
}

fn serialize_hash32<S>(value: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&encode_hex(value))
}

fn deserialize_hash32<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let bytes = decode_hex(&value).map_err(serde::de::Error::custom)?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| serde::de::Error::custom("expected 32-byte hash"))?;
    Ok(array)
}

fn serialize_codec<S>(value: &Codec, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u8(value.as_u8())
}

fn deserialize_codec<'de, D>(deserializer: D) -> Result<Codec, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Codec::from_u8(u8::deserialize(deserializer)?))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0F) as usize] as char);
    }
    out
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("hex string must have even length".to_string());
    }
    let mut out = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let high = decode_hex_nibble(pair[0])?;
        let low = decode_hex_nibble(pair[1])?;
        out.push((high << 4) | low);
    }
    Ok(out)
}

fn decode_hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex digit: {}", byte as char)),
    }
}
