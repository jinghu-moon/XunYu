use std::io::{Read, Write};

use crate::xunbak::codec::{self, CodecError};
use crate::xunbak::constants::{BLOB_HEADER_SIZE, Codec, RecordType};
use crate::xunbak::memory::allocate_zeroed_buffer;
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
    #[error("resource limit: {0}")]
    ResourceLimit(String),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobEncodedRecord {
    pub header: BlobHeader,
    pub record_len: u64,
    pub compressed: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobCopyResult {
    pub header: BlobHeader,
    pub record_len: u64,
    pub copied_bytes: u64,
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
    let raw_size = content.len() as u64;
    let mut compressed = codec::compress(codec, content, level)?;
    let effective_codec =
        codec::effective_codec_after_compression(codec, raw_size, compressed.len() as u64);
    if effective_codec != codec {
        compressed = content.to_vec();
    }
    let header = BlobHeader {
        blob_id: *blake3::hash(content).as_bytes(),
        blob_flags: 0,
        codec: effective_codec,
        raw_size,
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

pub fn write_blob_record_from_precompressed<W: Write>(
    writer: &mut W,
    blob_id: [u8; 32],
    raw_size: u64,
    codec: Codec,
    compressed: &[u8],
) -> Result<BlobWriteResult, BlobRecordError> {
    let header = BlobHeader {
        blob_id,
        blob_flags: 0,
        codec,
        raw_size,
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
        .write_all(compressed)
        .map_err(|err| BlobRecordError::Io(err.to_string()))?;

    Ok(BlobWriteResult { header, record_len })
}

pub fn read_blob_record<R: Read>(reader: &mut R) -> Result<BlobReadResult, BlobRecordError> {
    let encoded = read_blob_record_encoded(reader)?;
    decode_blob_record(encoded)
}

pub fn read_blob_record_encoded<R: Read>(
    reader: &mut R,
) -> Result<BlobEncodedRecord, BlobRecordError> {
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

    let mut compressed = allocate_zeroed_buffer(header.stored_size, "blob payload")
        .map_err(BlobRecordError::ResourceLimit)?;
    reader
        .read_exact(&mut compressed)
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::UnexpectedEof => BlobRecordError::BlobPayloadTruncated,
            _ => BlobRecordError::Io(err.to_string()),
        })?;
    Ok(BlobEncodedRecord {
        header,
        record_len: prefix.record_len,
        compressed,
    })
}

pub fn decode_blob_record(encoded: BlobEncodedRecord) -> Result<BlobReadResult, BlobRecordError> {
    let content = codec::decompress_bounded(
        encoded.header.codec,
        &encoded.compressed,
        encoded.header.raw_size,
    )?;
    if content.len() as u64 != encoded.header.raw_size {
        return Err(BlobRecordError::BlobLengthMismatch {
            record_len: content.len() as u64,
            expected_len: encoded.header.raw_size,
        });
    }
    if *blake3::hash(&content).as_bytes() != encoded.header.blob_id {
        return Err(BlobRecordError::BlobHashMismatch);
    }

    Ok(BlobReadResult {
        header: encoded.header,
        record_len: encoded.record_len,
        content,
    })
}

pub fn copy_blob_record_content_to_writer<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
) -> Result<BlobCopyResult, BlobRecordError> {
    let encoded = read_blob_record_encoded(reader)?;
    copy_encoded_blob_record_content_to_writer(encoded, writer)
}

pub fn copy_encoded_blob_record_content_to_writer<W: Write>(
    encoded: BlobEncodedRecord,
    writer: &mut W,
) -> Result<BlobCopyResult, BlobRecordError> {
    let mut hashing_writer = BoundedHashingWriter::new(writer, encoded.header.raw_size);
    let limited = std::io::Cursor::new(encoded.compressed);
    let mut decoder = codec::decompressed_reader(encoded.header.codec, limited)
        .map_err(BlobRecordError::Codec)?;
    let copied_bytes = match std::io::copy(&mut decoder, &mut hashing_writer) {
        Ok(copied_bytes) => copied_bytes,
        Err(err) if hashing_writer.limit_exceeded() => {
            return Err(BlobRecordError::BlobLengthMismatch {
                record_len: hashing_writer.written().saturating_add(1),
                expected_len: encoded.header.raw_size,
            });
        }
        Err(err) => {
            return Err(BlobRecordError::Codec(CodecError::DecodeFailed {
                codec: codec::codec_name(encoded.header.codec),
                message: err.to_string(),
            }));
        }
    };

    if copied_bytes != encoded.header.raw_size {
        return Err(BlobRecordError::BlobLengthMismatch {
            record_len: copied_bytes,
            expected_len: encoded.header.raw_size,
        });
    }
    if hashing_writer.finalize() != encoded.header.blob_id {
        return Err(BlobRecordError::BlobHashMismatch);
    }

    Ok(BlobCopyResult {
        header: encoded.header,
        record_len: encoded.record_len,
        copied_bytes,
    })
}

struct BoundedHashingWriter<'a, W> {
    inner: &'a mut W,
    hasher: blake3::Hasher,
    limit: u64,
    written: u64,
    limit_exceeded: bool,
}

impl<'a, W> BoundedHashingWriter<'a, W> {
    fn new(inner: &'a mut W, limit: u64) -> Self {
        Self {
            inner,
            hasher: blake3::Hasher::new(),
            limit,
            written: 0,
            limit_exceeded: false,
        }
    }

    fn finalize(self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }

    fn written(&self) -> u64 {
        self.written
    }

    fn limit_exceeded(&self) -> bool {
        self.limit_exceeded
    }
}

impl<W: Write> Write for BoundedHashingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let remaining = self.limit.saturating_sub(self.written) as usize;
        if remaining == 0 && !buf.is_empty() {
            self.limit_exceeded = true;
            return Err(std::io::Error::other("blob exceeds expected raw size"));
        }

        let to_write = remaining.min(buf.len());
        if to_write > 0 {
            self.inner.write_all(&buf[..to_write])?;
            self.hasher.update(&buf[..to_write]);
            self.written += to_write as u64;
        }
        if to_write < buf.len() {
            self.limit_exceeded = true;
            return Err(std::io::Error::other("blob exceeds expected raw size"));
        }
        Ok(to_write)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
