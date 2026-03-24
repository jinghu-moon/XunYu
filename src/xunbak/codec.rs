use std::io::{Cursor, Read, Write};

use crate::xunbak::constants::Codec;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CodecError {
    #[error("unsupported codec: {0:#04x}")]
    UnsupportedCodec(u8),
    #[error("zstd encode failed: {0}")]
    ZstdEncode(String),
    #[error("zstd decode failed: {0}")]
    ZstdDecode(String),
    #[error("invalid compression argument: {0}")]
    InvalidCompressionArg(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompressionMode {
    None,
    Zstd { level: i32 },
    Lz4,
    Lzma,
    Auto,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamCompressResult {
    pub hash: [u8; 32],
    pub raw_size: u64,
    pub compressed: Vec<u8>,
    pub peak_buffer_bytes: usize,
}

pub fn compress(codec: Codec, data: &[u8], level: i32) -> Result<Vec<u8>, CodecError> {
    match codec {
        codec if codec == Codec::NONE => Ok(data.to_vec()),
        codec if codec == Codec::ZSTD => zstd::stream::encode_all(Cursor::new(data), level)
            .map_err(|err| CodecError::ZstdEncode(err.to_string())),
        codec => Err(CodecError::UnsupportedCodec(codec.as_u8())),
    }
}

pub fn decompress(codec: Codec, data: &[u8]) -> Result<Vec<u8>, CodecError> {
    match codec {
        codec if codec == Codec::NONE => Ok(data.to_vec()),
        codec if codec == Codec::ZSTD => zstd::stream::decode_all(Cursor::new(data))
            .map_err(|err| CodecError::ZstdDecode(err.to_string())),
        codec => Err(CodecError::UnsupportedCodec(codec.as_u8())),
    }
}

pub fn should_skip_compress(ext: &str) -> bool {
    let ext = ext.trim().trim_start_matches('.').to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "zip"
            | "7z"
            | "rar"
            | "gz"
            | "xz"
            | "zst"
            | "lz4"
            | "bz2"
            | "br"
            | "jpg"
            | "jpeg"
            | "png"
            | "webp"
            | "mp4"
            | "mkv"
    )
}

pub fn compression_is_beneficial(raw_size: u64, stored_size: u64) -> bool {
    if raw_size == 0 {
        return false;
    }
    (stored_size as f64) < (raw_size as f64 * 0.95)
}

pub fn parse_compression_arg(raw: &str) -> Result<CompressionMode, CodecError> {
    let value = raw.trim().to_ascii_lowercase();
    if value.is_empty() {
        return Err(CodecError::InvalidCompressionArg(raw.to_string()));
    }

    match value.as_str() {
        "none" => Ok(CompressionMode::None),
        "zstd" => Ok(CompressionMode::Zstd { level: 1 }),
        "lz4" => Ok(CompressionMode::Lz4),
        "lzma" => Ok(CompressionMode::Lzma),
        "auto" => Ok(CompressionMode::Auto),
        _ => {
            if let Some(level) = value.strip_prefix("zstd:") {
                let level = level
                    .parse::<i32>()
                    .map_err(|_| CodecError::InvalidCompressionArg(raw.to_string()))?;
                return Ok(CompressionMode::Zstd { level });
            }
            Err(CodecError::InvalidCompressionArg(raw.to_string()))
        }
    }
}

pub fn stream_hash_and_compress<R: Read>(
    reader: &mut R,
    codec: Codec,
    level: i32,
    chunk_size: usize,
) -> Result<StreamCompressResult, CodecError> {
    let chunk_size = chunk_size.max(1);
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; chunk_size];
    let mut peak_buffer_bytes = 0usize;
    let mut raw_size = 0u64;

    match codec {
        codec if codec == Codec::NONE => {
            let mut out = Vec::new();
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|err| CodecError::ZstdDecode(err.to_string()))?;
                if n == 0 {
                    break;
                }
                raw_size += n as u64;
                hasher.update(&buf[..n]);
                out.extend_from_slice(&buf[..n]);
                peak_buffer_bytes = peak_buffer_bytes.max(n);
            }
            Ok(StreamCompressResult {
                hash: *hasher.finalize().as_bytes(),
                raw_size,
                compressed: out,
                peak_buffer_bytes,
            })
        }
        codec if codec == Codec::ZSTD => {
            let mut encoder = zstd::stream::Encoder::new(Vec::new(), level)
                .map_err(|err| CodecError::ZstdEncode(err.to_string()))?;
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|err| CodecError::ZstdEncode(err.to_string()))?;
                if n == 0 {
                    break;
                }
                raw_size += n as u64;
                hasher.update(&buf[..n]);
                encoder
                    .write_all(&buf[..n])
                    .map_err(|err| CodecError::ZstdEncode(err.to_string()))?;
                peak_buffer_bytes = peak_buffer_bytes.max(n * 2);
            }
            let compressed = encoder
                .finish()
                .map_err(|err| CodecError::ZstdEncode(err.to_string()))?;
            Ok(StreamCompressResult {
                hash: *hasher.finalize().as_bytes(),
                raw_size,
                compressed,
                peak_buffer_bytes,
            })
        }
        codec => Err(CodecError::UnsupportedCodec(codec.as_u8())),
    }
}
