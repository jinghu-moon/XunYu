use std::io::{Read, Write};

use crate::xunbak::codec::{self, CodecError, compression_is_beneficial};
use crate::xunbak::constants::{BLOB_HEADER_SIZE, Codec, RecordType};
use crate::xunbak::record::{RecordPrefix, RecordScanError, compute_record_crc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlobHeader {
    pub blob_id: [u8; 32],
    pub blob_flags: u8,
    pub codec: Codec,
    pub raw_size: u64,
    pub stored_size: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlobHeaderError {
    HeaderTooShort { actual: usize },
}

#[derive(Debug, thiserror::Error)]
pub enum BlobRecordError {
    #[error("blob header error: {0:?}")]
    Header(BlobHeaderError),
    #[error("record prefix error: {0:?}")]
    Prefix(RecordScanError),
    #[error("unexpected record type: {0:#04x}")]
    UnexpectedRecordType(u8),
    #[error("blob record CRC mismatch")]
    BlobCrcMismatch,
    #[error("blob length mismatch: record_len={record_len}, expected={expected_len}")]
    BlobLengthMismatch { record_len: u64, expected_len: u64 },
    #[error("blob payload truncated")]
    BlobPayloadTruncated,
    #[error("blob content hash mismatch")]
    BlobHashMismatch,
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error("I/O error: {0}")]
    Io(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobWriteResult {
    pub header: BlobHeader,
    pub record_len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobReadResult {
    pub header: BlobHeader,
    pub record_len: u64,
    pub content: Vec<u8>,
}

impl BlobHeader {
    pub fn to_bytes(self) -> [u8; BLOB_HEADER_SIZE] {
        let mut bytes = [0u8; BLOB_HEADER_SIZE];
        bytes[..32].copy_from_slice(&self.blob_id);
        bytes[32] = self.blob_flags;
        bytes[33] = self.codec.as_u8();
        bytes[34..42].copy_from_slice(&self.raw_size.to_le_bytes());
        bytes[42..50].copy_from_slice(&self.stored_size.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BlobHeaderError> {
        if bytes.len() < BLOB_HEADER_SIZE {
            return Err(BlobHeaderError::HeaderTooShort {
                actual: bytes.len(),
            });
        }
        let mut blob_id = [0u8; 32];
        blob_id.copy_from_slice(&bytes[..32]);
        Ok(Self {
            blob_id,
            blob_flags: bytes[32],
            codec: Codec::from_u8(bytes[33]),
            raw_size: u64::from_le_bytes(bytes[34..42].try_into().expect("len checked")),
            stored_size: u64::from_le_bytes(bytes[42..50].try_into().expect("len checked")),
        })
    }
}

pub fn write_blob_record<W: Write>(
    writer: &mut W,
    content: &[u8],
    codec: Codec,
    level: i32,
) -> Result<BlobWriteResult, BlobRecordError> {
    let mut effective_codec = codec;
    let mut compressed = codec::compress(codec, content, level)?;
    if codec != Codec::NONE
        && !compression_is_beneficial(content.len() as u64, compressed.len() as u64)
    {
        effective_codec = Codec::NONE;
        compressed = content.to_vec();
    }
    let header = BlobHeader {
        blob_id: *blake3::hash(content).as_bytes(),
        blob_flags: 0,
        codec: effective_codec,
        raw_size: content.len() as u64,
        stored_size: compressed.len() as u64,
    };
    let header_bytes = header.to_bytes();
    let record_len = (BLOB_HEADER_SIZE + compressed.len()) as u64;
    let prefix = RecordPrefix {
        record_type: RecordType::BLOB,
        record_len,
        record_crc: compute_record_crc(RecordType::BLOB, record_len.to_le_bytes(), &header_bytes),
    };

    writer
        .write_all(&prefix.to_bytes())
        .map_err(|err| BlobRecordError::Io(err.to_string()))?;
    writer
        .write_all(&header_bytes)
        .map_err(|err| BlobRecordError::Io(err.to_string()))?;
    writer
        .write_all(&compressed)
        .map_err(|err| BlobRecordError::Io(err.to_string()))?;

    Ok(BlobWriteResult { header, record_len })
}

pub fn read_blob_record<R: Read>(reader: &mut R) -> Result<BlobReadResult, BlobRecordError> {
    let mut prefix_bytes = [0u8; crate::xunbak::constants::RECORD_PREFIX_SIZE];
    reader
        .read_exact(&mut prefix_bytes)
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::UnexpectedEof => {
                BlobRecordError::Prefix(RecordScanError::PrefixTooShort { actual: 0 })
            }
            _ => BlobRecordError::Io(err.to_string()),
        })?;
    let prefix = RecordPrefix::from_bytes(&prefix_bytes).map_err(BlobRecordError::Prefix)?;
    if prefix.record_type != RecordType::BLOB {
        return Err(BlobRecordError::UnexpectedRecordType(
            prefix.record_type.as_u8(),
        ));
    }

    let mut header_bytes = [0u8; BLOB_HEADER_SIZE];
    reader
        .read_exact(&mut header_bytes)
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::UnexpectedEof => BlobRecordError::BlobPayloadTruncated,
            _ => BlobRecordError::Io(err.to_string()),
        })?;
    let header = BlobHeader::from_bytes(&header_bytes).map_err(BlobRecordError::Header)?;

    let expected_record_len = (BLOB_HEADER_SIZE as u64) + header.stored_size;
    if prefix.record_len != expected_record_len {
        return Err(BlobRecordError::BlobLengthMismatch {
            record_len: prefix.record_len,
            expected_len: expected_record_len,
        });
    }

    let actual_crc = compute_record_crc(
        RecordType::BLOB,
        prefix.record_len.to_le_bytes(),
        &header_bytes,
    );
    if actual_crc != prefix.record_crc {
        return Err(BlobRecordError::BlobCrcMismatch);
    }

    let mut compressed = vec![0u8; header.stored_size as usize];
    reader
        .read_exact(&mut compressed)
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::UnexpectedEof => BlobRecordError::BlobPayloadTruncated,
            _ => BlobRecordError::Io(err.to_string()),
        })?;
    let content = codec::decompress(header.codec, &compressed)?;
    if *blake3::hash(&content).as_bytes() != header.blob_id {
        return Err(BlobRecordError::BlobHashMismatch);
    }

    Ok(BlobReadResult {
        header,
        record_len: prefix.record_len,
        content,
    })
}
