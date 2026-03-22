use std::io::Read;

use crate::xunbak::constants::{BLOB_HEADER_SIZE, RECORD_PREFIX_SIZE, RecordType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordPrefix {
    pub record_type: RecordType,
    pub record_len: u64,
    pub record_crc: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScannedRecord {
    pub offset: u64,
    pub record_type: RecordType,
    pub record_len: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RecordScanError {
    Io,
    PrefixTooShort { actual: usize },
    TruncatedRecord { offset: u64, record_len: u64 },
}

impl RecordPrefix {
    pub fn to_bytes(self) -> [u8; RECORD_PREFIX_SIZE] {
        let mut bytes = [0u8; RECORD_PREFIX_SIZE];
        bytes[0] = self.record_type.as_u8();
        bytes[1..9].copy_from_slice(&self.record_len.to_le_bytes());
        bytes[9..13].copy_from_slice(&self.record_crc.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RecordScanError> {
        if bytes.len() < RECORD_PREFIX_SIZE {
            return Err(RecordScanError::PrefixTooShort {
                actual: bytes.len(),
            });
        }
        Ok(Self {
            record_type: RecordType::from_u8(bytes[0]),
            record_len: u64::from_le_bytes(bytes[1..9].try_into().expect("len checked")),
            record_crc: u32::from_le_bytes(bytes[9..13].try_into().expect("len checked")),
        })
    }
}

pub fn compute_record_crc(
    record_type: RecordType,
    record_len_bytes: [u8; 8],
    payload_for_crc: &[u8],
) -> u32 {
    let mut buf = Vec::with_capacity(1 + record_len_bytes.len() + payload_for_crc.len());
    buf.push(record_type.as_u8());
    buf.extend_from_slice(&record_len_bytes);
    buf.extend_from_slice(payload_for_crc);
    crc32c::crc32c(&buf)
}

pub fn scan_records<R: Read>(reader: &mut R) -> Result<Vec<ScannedRecord>, RecordScanError> {
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| RecordScanError::Io)?;

    let mut records = Vec::new();
    let mut offset = 0usize;
    while offset + RECORD_PREFIX_SIZE <= bytes.len() {
        let prefix = RecordPrefix::from_bytes(&bytes[offset..offset + RECORD_PREFIX_SIZE])?;
        let payload_start = offset + RECORD_PREFIX_SIZE;
        let payload_end = payload_start.saturating_add(prefix.record_len as usize);
        if payload_end > bytes.len() {
            return Err(RecordScanError::TruncatedRecord {
                offset: offset as u64,
                record_len: prefix.record_len,
            });
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
            break;
        }

        records.push(ScannedRecord {
            offset: offset as u64,
            record_type: prefix.record_type,
            record_len: prefix.record_len,
        });
        offset = payload_end;
    }

    Ok(records)
}
