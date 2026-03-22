use std::io::{Read, Write};

use crate::xunbak::constants::{CHECKPOINT_PAYLOAD_SIZE, RecordType};
use crate::xunbak::record::{RecordPrefix, RecordScanError, compute_record_crc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointPayload {
    pub snapshot_id: [u8; 16],
    pub manifest_offset: u64,
    pub manifest_len: u64,
    pub manifest_hash: [u8; 32],
    pub container_end: u64,
    pub blob_count: u64,
    pub referenced_blob_bytes: u64,
    pub total_container_bytes: u64,
    pub prev_checkpoint_offset: u64,
    pub total_volumes: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointWriteResult {
    pub record_len: u64,
    pub payload: CheckpointPayload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointReadResult {
    pub record_len: u64,
    pub payload: CheckpointPayload,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CheckpointError {
    #[error("checkpoint payload too short: {actual}")]
    PayloadTooShort { actual: usize },
    #[error("record prefix error: {0:?}")]
    RecordPrefix(RecordScanError),
    #[error("unexpected record type: {0:#04x}")]
    UnexpectedRecordType(u8),
    #[error("checkpoint record CRC mismatch")]
    RecordCrcMismatch,
    #[error("checkpoint payload CRC mismatch")]
    CheckpointCrcMismatch,
    #[error(
        "manifest offset out of range: offset={manifest_offset}, len={manifest_len}, container_end={container_end}"
    )]
    ManifestOffsetOutOfRange {
        manifest_offset: u64,
        manifest_len: u64,
        container_end: u64,
    },
    #[error("I/O error: {0}")]
    Io(String),
}

impl CheckpointPayload {
    pub fn to_bytes(&self) -> [u8; CHECKPOINT_PAYLOAD_SIZE] {
        let mut bytes = [0u8; CHECKPOINT_PAYLOAD_SIZE];
        bytes[..16].copy_from_slice(&self.snapshot_id);
        bytes[16..24].copy_from_slice(&self.manifest_offset.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.manifest_len.to_le_bytes());
        bytes[32..64].copy_from_slice(&self.manifest_hash);
        bytes[64..72].copy_from_slice(&self.container_end.to_le_bytes());
        bytes[72..80].copy_from_slice(&self.blob_count.to_le_bytes());
        bytes[80..88].copy_from_slice(&self.referenced_blob_bytes.to_le_bytes());
        bytes[88..96].copy_from_slice(&self.total_container_bytes.to_le_bytes());
        bytes[96..104].copy_from_slice(&self.prev_checkpoint_offset.to_le_bytes());
        bytes[104..106].copy_from_slice(&self.total_volumes.to_le_bytes());
        let crc = crc32c::crc32c(&bytes[..124]);
        bytes[124..128].copy_from_slice(&crc.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CheckpointError> {
        if bytes.len() < CHECKPOINT_PAYLOAD_SIZE {
            return Err(CheckpointError::PayloadTooShort {
                actual: bytes.len(),
            });
        }
        let expected_crc = u32::from_le_bytes(bytes[124..128].try_into().expect("len checked"));
        let actual_crc = crc32c::crc32c(&bytes[..124]);
        if expected_crc != actual_crc {
            return Err(CheckpointError::CheckpointCrcMismatch);
        }
        let mut snapshot_id = [0u8; 16];
        snapshot_id.copy_from_slice(&bytes[..16]);
        let mut manifest_hash = [0u8; 32];
        manifest_hash.copy_from_slice(&bytes[32..64]);
        let payload = Self {
            snapshot_id,
            manifest_offset: u64::from_le_bytes(bytes[16..24].try_into().expect("len checked")),
            manifest_len: u64::from_le_bytes(bytes[24..32].try_into().expect("len checked")),
            manifest_hash,
            container_end: u64::from_le_bytes(bytes[64..72].try_into().expect("len checked")),
            blob_count: u64::from_le_bytes(bytes[72..80].try_into().expect("len checked")),
            referenced_blob_bytes: u64::from_le_bytes(
                bytes[80..88].try_into().expect("len checked"),
            ),
            total_container_bytes: u64::from_le_bytes(
                bytes[88..96].try_into().expect("len checked"),
            ),
            prev_checkpoint_offset: u64::from_le_bytes(
                bytes[96..104].try_into().expect("len checked"),
            ),
            total_volumes: u16::from_le_bytes(bytes[104..106].try_into().expect("len checked")),
        };
        validate_manifest_range(&payload)?;
        Ok(payload)
    }
}

pub fn write_checkpoint_record<W: Write>(
    writer: &mut W,
    payload: &CheckpointPayload,
) -> Result<CheckpointWriteResult, CheckpointError> {
    let payload_bytes = payload.to_bytes();
    let record_len = CHECKPOINT_PAYLOAD_SIZE as u64;
    let prefix = RecordPrefix {
        record_type: RecordType::CHECKPOINT,
        record_len,
        record_crc: compute_record_crc(
            RecordType::CHECKPOINT,
            record_len.to_le_bytes(),
            &payload_bytes,
        ),
    };
    writer
        .write_all(&prefix.to_bytes())
        .map_err(|err| CheckpointError::Io(err.to_string()))?;
    writer
        .write_all(&payload_bytes)
        .map_err(|err| CheckpointError::Io(err.to_string()))?;
    Ok(CheckpointWriteResult {
        record_len,
        payload: payload.clone(),
    })
}

pub fn read_checkpoint_record<R: Read>(
    reader: &mut R,
) -> Result<CheckpointReadResult, CheckpointError> {
    let mut prefix_bytes = [0u8; crate::xunbak::constants::RECORD_PREFIX_SIZE];
    reader
        .read_exact(&mut prefix_bytes)
        .map_err(|err| CheckpointError::Io(err.to_string()))?;
    let prefix = RecordPrefix::from_bytes(&prefix_bytes).map_err(CheckpointError::RecordPrefix)?;
    if prefix.record_type != RecordType::CHECKPOINT {
        return Err(CheckpointError::UnexpectedRecordType(
            prefix.record_type.as_u8(),
        ));
    }
    let mut payload_bytes = vec![0u8; prefix.record_len as usize];
    reader
        .read_exact(&mut payload_bytes)
        .map_err(|err| CheckpointError::Io(err.to_string()))?;
    let actual_crc = compute_record_crc(
        RecordType::CHECKPOINT,
        prefix.record_len.to_le_bytes(),
        &payload_bytes,
    );
    if actual_crc != prefix.record_crc {
        return Err(CheckpointError::RecordCrcMismatch);
    }
    let payload = CheckpointPayload::from_bytes(&payload_bytes)?;
    Ok(CheckpointReadResult {
        record_len: prefix.record_len,
        payload,
    })
}

pub fn compute_manifest_hash(manifest_payload: &[u8]) -> [u8; 32] {
    *blake3::hash(manifest_payload).as_bytes()
}

fn validate_manifest_range(payload: &CheckpointPayload) -> Result<(), CheckpointError> {
    if payload.manifest_offset.saturating_add(payload.manifest_len) > payload.container_end {
        return Err(CheckpointError::ManifestOffsetOutOfRange {
            manifest_offset: payload.manifest_offset,
            manifest_len: payload.manifest_len,
            container_end: payload.container_end,
        });
    }
    Ok(())
}
