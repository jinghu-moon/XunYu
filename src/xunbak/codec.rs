use std::io::{Cursor, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::xunbak::constants::Codec;
use crate::xunbak::memory::reserve_buffer_capacity;

const DEFLATE_LEVEL_DEFAULT: u32 = 6;
const BZIP2_LEVEL_DEFAULT: u32 = 6;
const LZMA2_PRESET_DEFAULT: u32 = 6;
const PPMD_ORDER_DEFAULT: u32 = 8;
const PPMD_MEM_SIZE_DEFAULT: u32 = 16 << 20;
pub const XUNBAK_COMPRESSION_PROFILE_FIX_HINT: &str = "Fix: Use one of `none`, `zstd`, `zstd:N`, `lz4`, `lzma2`, `deflate`, `bzip2`, `ppmd`, or `auto`.";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CodecError {
    #[error("unsupported codec: {0:#04x}")]
    UnsupportedCodec(u8),
    #[error("{codec} encode failed: {message}")]
    EncodeFailed {
        codec: &'static str,
        message: String,
    },
    #[error("{codec} decode failed: {message}")]
    DecodeFailed {
        codec: &'static str,
        message: String,
    },
    #[error("invalid compression argument: {0}")]
    InvalidCompressionArg(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompressionMode {
    None,
    Zstd { level: i32 },
    Lz4,
    Lzma2,
    Deflate,
    Bzip2,
    Ppmd,
    Auto,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamCompressResult {
    pub hash: [u8; 32],
    pub raw_size: u64,
    pub compressed: Vec<u8>,
    pub peak_buffer_bytes: usize,
}

pub fn codec_name(codec: Codec) -> &'static str {
    match codec {
        codec if codec == Codec::NONE => "none",
        codec if codec == Codec::ZSTD => "zstd",
        codec if codec == Codec::LZ4 => "lz4",
        codec if codec == Codec::LZMA2 => "lzma2",
        codec if codec == Codec::DEFLATE => "deflate",
        codec if codec == Codec::BZIP2 => "bzip2",
        codec if codec == Codec::PPMD => "ppmd",
        _ => "unknown",
    }
}

pub fn compress(codec: Codec, data: &[u8], level: i32) -> Result<Vec<u8>, CodecError> {
    match codec {
        codec if codec == Codec::NONE => Ok(data.to_vec()),
        codec if codec == Codec::ZSTD => zstd::stream::encode_all(Cursor::new(data), level)
            .map_err(|err| encode_error(codec, err)),
        codec if codec == Codec::LZ4 => {
            let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
            encoder
                .write_all(data)
                .map_err(|err| encode_error(codec, err))?;
            encoder.finish().map_err(|err| encode_error(codec, err))
        }
        codec if codec == Codec::DEFLATE => {
            let mut encoder = flate2::write::DeflateEncoder::new(
                Vec::new(),
                flate2::Compression::new(DEFLATE_LEVEL_DEFAULT),
            );
            encoder
                .write_all(data)
                .map_err(|err| encode_error(codec, err))?;
            encoder.finish().map_err(|err| encode_error(codec, err))
        }
        codec if codec == Codec::BZIP2 => {
            let mut encoder = bzip2::write::BzEncoder::new(
                Vec::new(),
                bzip2::Compression::new(BZIP2_LEVEL_DEFAULT),
            );
            encoder
                .write_all(data)
                .map_err(|err| encode_error(codec, err))?;
            encoder.finish().map_err(|err| encode_error(codec, err))
        }
        codec if codec == Codec::PPMD => catch_codec_unwind(codec, "encode", || {
            let mut encoder = ppmd_rust::Ppmd8Encoder::new(
                Vec::new(),
                PPMD_ORDER_DEFAULT,
                PPMD_MEM_SIZE_DEFAULT,
                ppmd_rust::RestoreMethod::Restart,
            )
            .map_err(|err| encode_error(codec, err))?;
            encoder
                .write_all(data)
                .map_err(|err| encode_error(codec, err))?;
            encoder.finish(true).map_err(|err| encode_error(codec, err))
        }),
        codec if codec == Codec::LZMA2 => {
            let options = lzma_rust2::Lzma2Options::with_preset(LZMA2_PRESET_DEFAULT);
            let mut encoder = lzma_rust2::Lzma2Writer::new(Vec::new(), options);
            encoder
                .write_all(data)
                .map_err(|err| encode_error(codec, err))?;
            encoder.finish().map_err(|err| encode_error(codec, err))
        }
        codec => Err(CodecError::UnsupportedCodec(codec.as_u8())),
    }
}

pub fn decompressed_reader<'a, R: Read + 'a>(
    codec: Codec,
    reader: R,
) -> Result<Box<dyn Read + 'a>, CodecError> {
    match codec {
        codec if codec == Codec::NONE => Ok(Box::new(reader)),
        codec if codec == Codec::ZSTD => {
            let decoder =
                zstd::stream::Decoder::new(reader).map_err(|err| decode_error(codec, err))?;
            Ok(Box::new(decoder))
        }
        codec if codec == Codec::LZ4 => Ok(Box::new(lz4_flex::frame::FrameDecoder::new(reader))),
        codec if codec == Codec::DEFLATE => Ok(Box::new(flate2::read::DeflateDecoder::new(reader))),
        codec if codec == Codec::BZIP2 => Ok(Box::new(bzip2::read::BzDecoder::new(reader))),
        codec if codec == Codec::PPMD => {
            let decoder = catch_codec_unwind(codec, "decode", || {
                ppmd_rust::Ppmd8Decoder::new(
                    reader,
                    PPMD_ORDER_DEFAULT,
                    PPMD_MEM_SIZE_DEFAULT,
                    ppmd_rust::RestoreMethod::Restart,
                )
                .map_err(|err| decode_error(codec, err))
            })?;
            Ok(Box::new(PanicGuardedRead::new(codec, "decode", decoder)))
        }
        codec if codec == Codec::LZMA2 => Ok(Box::new(lzma_rust2::Lzma2Reader::new(
            reader,
            lzma2_dict_size(),
            None,
        ))),
        codec => Err(CodecError::UnsupportedCodec(codec.as_u8())),
    }
}

pub fn copy_decompressed_to_writer<R: Read, W: Write>(
    codec: Codec,
    reader: R,
    writer: &mut W,
) -> Result<u64, CodecError> {
    let mut decoder = decompressed_reader(codec, reader)?;
    std::io::copy(&mut decoder, writer).map_err(|err| decode_error(codec, err))
}

pub fn decompress(codec: Codec, data: &[u8]) -> Result<Vec<u8>, CodecError> {
    let mut decoder = decompressed_reader(codec, Cursor::new(data))?;
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|err| decode_error(codec, err))?;
    Ok(out)
}

pub fn decompress_bounded(
    codec: Codec,
    data: &[u8],
    max_output_bytes: u64,
) -> Result<Vec<u8>, CodecError> {
    let limit = max_output_bytes
        .checked_add(1)
        .ok_or_else(|| decode_error(codec, "max output size overflow"))?;
    let decoder = decompressed_reader(codec, Cursor::new(data))?;
    let mut limited = decoder.take(limit);
    let mut out = Vec::new();
    reserve_buffer_capacity(&mut out, limit, "decompressed content")
        .map_err(|err| decode_error(codec, err))?;
    limited
        .read_to_end(&mut out)
        .map_err(|err| decode_error(codec, err))?;
    Ok(out)
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

pub fn effective_codec_after_compression(
    requested: Codec,
    raw_size: u64,
    stored_size: u64,
) -> Codec {
    if requested == Codec::NONE || compression_is_beneficial(raw_size, stored_size) {
        requested
    } else {
        Codec::NONE
    }
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
        "lzma2" | "lzma" => Ok(CompressionMode::Lzma2),
        "deflate" => Ok(CompressionMode::Deflate),
        "bzip2" => Ok(CompressionMode::Bzip2),
        "ppmd" => Ok(CompressionMode::Ppmd),
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
                    .map_err(|err| decode_error(codec, err))?;
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
                .map_err(|err| encode_error(codec, err))?;
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|err| encode_error(codec, err))?;
                if n == 0 {
                    break;
                }
                raw_size += n as u64;
                hasher.update(&buf[..n]);
                encoder
                    .write_all(&buf[..n])
                    .map_err(|err| encode_error(codec, err))?;
                peak_buffer_bytes = peak_buffer_bytes.max(n * 2);
            }
            let compressed = encoder.finish().map_err(|err| encode_error(codec, err))?;
            Ok(StreamCompressResult {
                hash: *hasher.finalize().as_bytes(),
                raw_size,
                compressed,
                peak_buffer_bytes,
            })
        }
        codec if codec == Codec::LZ4 => {
            let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|err| encode_error(codec, err))?;
                if n == 0 {
                    break;
                }
                raw_size += n as u64;
                hasher.update(&buf[..n]);
                encoder
                    .write_all(&buf[..n])
                    .map_err(|err| encode_error(codec, err))?;
                peak_buffer_bytes = peak_buffer_bytes.max(n * 2);
            }
            let compressed = encoder.finish().map_err(|err| encode_error(codec, err))?;
            Ok(StreamCompressResult {
                hash: *hasher.finalize().as_bytes(),
                raw_size,
                compressed,
                peak_buffer_bytes,
            })
        }
        codec if codec == Codec::DEFLATE => {
            let mut encoder = flate2::write::DeflateEncoder::new(
                Vec::new(),
                flate2::Compression::new(DEFLATE_LEVEL_DEFAULT),
            );
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|err| encode_error(codec, err))?;
                if n == 0 {
                    break;
                }
                raw_size += n as u64;
                hasher.update(&buf[..n]);
                encoder
                    .write_all(&buf[..n])
                    .map_err(|err| encode_error(codec, err))?;
                peak_buffer_bytes = peak_buffer_bytes.max(n * 2);
            }
            let compressed = encoder.finish().map_err(|err| encode_error(codec, err))?;
            Ok(StreamCompressResult {
                hash: *hasher.finalize().as_bytes(),
                raw_size,
                compressed,
                peak_buffer_bytes,
            })
        }
        codec if codec == Codec::BZIP2 => {
            let mut encoder = bzip2::write::BzEncoder::new(
                Vec::new(),
                bzip2::Compression::new(BZIP2_LEVEL_DEFAULT),
            );
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|err| encode_error(codec, err))?;
                if n == 0 {
                    break;
                }
                raw_size += n as u64;
                hasher.update(&buf[..n]);
                encoder
                    .write_all(&buf[..n])
                    .map_err(|err| encode_error(codec, err))?;
                peak_buffer_bytes = peak_buffer_bytes.max(n * 2);
            }
            let compressed = encoder.finish().map_err(|err| encode_error(codec, err))?;
            Ok(StreamCompressResult {
                hash: *hasher.finalize().as_bytes(),
                raw_size,
                compressed,
                peak_buffer_bytes,
            })
        }
        codec if codec == Codec::PPMD => catch_codec_unwind(codec, "encode", || {
            let mut encoder = ppmd_rust::Ppmd8Encoder::new(
                Vec::new(),
                PPMD_ORDER_DEFAULT,
                PPMD_MEM_SIZE_DEFAULT,
                ppmd_rust::RestoreMethod::Restart,
            )
            .map_err(|err| encode_error(codec, err))?;
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|err| encode_error(codec, err))?;
                if n == 0 {
                    break;
                }
                raw_size += n as u64;
                hasher.update(&buf[..n]);
                encoder
                    .write_all(&buf[..n])
                    .map_err(|err| encode_error(codec, err))?;
                peak_buffer_bytes = peak_buffer_bytes.max(n * 2);
            }
            let compressed = encoder
                .finish(true)
                .map_err(|err| encode_error(codec, err))?;
            Ok(StreamCompressResult {
                hash: *hasher.finalize().as_bytes(),
                raw_size,
                compressed,
                peak_buffer_bytes,
            })
        }),
        codec if codec == Codec::LZMA2 => {
            let mut encoder = lzma_rust2::Lzma2Writer::new(Vec::new(), lzma2_options());
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|err| encode_error(codec, err))?;
                if n == 0 {
                    break;
                }
                raw_size += n as u64;
                hasher.update(&buf[..n]);
                encoder
                    .write_all(&buf[..n])
                    .map_err(|err| encode_error(codec, err))?;
                peak_buffer_bytes = peak_buffer_bytes.max(n * 2);
            }
            let compressed = encoder.finish().map_err(|err| encode_error(codec, err))?;
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

fn lzma2_options() -> lzma_rust2::Lzma2Options {
    lzma_rust2::Lzma2Options::with_preset(LZMA2_PRESET_DEFAULT)
}

fn lzma2_dict_size() -> u32 {
    lzma2_options().lzma_options.dict_size
}

fn encode_error(codec: Codec, err: impl ToString) -> CodecError {
    CodecError::EncodeFailed {
        codec: codec_name(codec),
        message: err.to_string(),
    }
}

fn decode_error(codec: Codec, err: impl ToString) -> CodecError {
    CodecError::DecodeFailed {
        codec: codec_name(codec),
        message: err.to_string(),
    }
}

fn catch_codec_unwind<T>(
    codec: Codec,
    phase: &'static str,
    f: impl FnOnce() -> Result<T, CodecError>,
) -> Result<T, CodecError> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => Err(codec_panic_error(codec, phase, payload)),
    }
}

fn catch_codec_io_unwind<T>(
    codec: Codec,
    phase: &'static str,
    f: impl FnOnce() -> std::io::Result<T>,
) -> std::io::Result<T> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => Err(std::io::Error::other(format!(
            "{} {} panicked: {}",
            codec_name(codec),
            phase,
            panic_payload_to_string(payload)
        ))),
    }
}

fn codec_panic_error(
    codec: Codec,
    phase: &'static str,
    payload: Box<dyn std::any::Any + Send>,
) -> CodecError {
    let message = format!("{phase} panicked: {}", panic_payload_to_string(payload));
    if phase == "encode" {
        encode_error(codec, message)
    } else {
        decode_error(codec, message)
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

struct PanicGuardedRead<R> {
    codec: Codec,
    phase: &'static str,
    inner: R,
}

impl<R> PanicGuardedRead<R> {
    fn new(codec: Codec, phase: &'static str, inner: R) -> Self {
        Self {
            codec,
            phase,
            inner,
        }
    }
}

impl<R: Read> Read for PanicGuardedRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        catch_codec_io_unwind(self.codec, self.phase, || self.inner.read(buf))
    }
}
