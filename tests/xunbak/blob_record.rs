use std::io::{Cursor, Write};

use xun::xunbak::blob::{
    BlobReadResult, BlobRecordError, BlobWriteResult, copy_blob_record_content_to_writer,
    read_blob_record, write_blob_record,
};
use xun::xunbak::constants::{BLOB_HEADER_SIZE, Codec, RECORD_PREFIX_SIZE, RecordType};
use xun::xunbak::record::RecordPrefix;

fn decode_prefix(bytes: &[u8]) -> RecordPrefix {
    RecordPrefix::from_bytes(&bytes[..RECORD_PREFIX_SIZE]).unwrap()
}

fn incompressible_bytes(len: usize) -> Vec<u8> {
    let mut state = 0x1234_5678u32;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        out.push((state & 0xFF) as u8);
    }
    out
}

struct ChunkLimitedWriter {
    max_write_len: usize,
    chunks: usize,
    data: Vec<u8>,
}

impl ChunkLimitedWriter {
    fn new(max_write_len: usize) -> Self {
        Self {
            max_write_len,
            chunks: 0,
            data: Vec::new(),
        }
    }
}

impl Write for ChunkLimitedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() > self.max_write_len {
            return Err(std::io::Error::other(format!(
                "chunk too large: {} > {}",
                buf.len(),
                self.max_write_len
            )));
        }
        self.chunks += 1;
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn write_blob_record_for_small_plaintext_matches_layout() {
    let mut out = Vec::new();
    let result = write_blob_record(&mut out, b"hello world", Codec::NONE, 1).unwrap();
    let prefix = decode_prefix(&out);
    assert_eq!(prefix.record_type, RecordType::BLOB);
    assert_eq!(prefix.record_len, (BLOB_HEADER_SIZE + 11) as u64);
    assert_eq!(
        result.header.blob_id,
        *blake3::hash(b"hello world").as_bytes()
    );
}

#[test]
fn none_codec_keeps_stored_size_equal_to_raw_size() {
    let mut out = Vec::new();
    let result = write_blob_record(&mut out, b"hello world", Codec::NONE, 1).unwrap();
    assert_eq!(result.header.raw_size, 11);
    assert_eq!(result.header.stored_size, 11);
    assert_eq!(
        &out[(RECORD_PREFIX_SIZE + BLOB_HEADER_SIZE)..],
        b"hello world"
    );
}

#[test]
fn zstd_codec_roundtrip_matches_original_content() {
    let mut out = Vec::new();
    write_blob_record(
        &mut out,
        b"hello world hello world hello world",
        Codec::ZSTD,
        1,
    )
    .unwrap();
    let read = read_blob_record(&mut Cursor::new(out)).unwrap();
    assert_eq!(read.content, b"hello world hello world hello world");
}

#[test]
fn lz4_codec_roundtrip_matches_original_content() {
    let mut out = Vec::new();
    write_blob_record(
        &mut out,
        b"hello world hello world hello world",
        Codec::LZ4,
        1,
    )
    .unwrap();
    let read = read_blob_record(&mut Cursor::new(out)).unwrap();
    assert_eq!(read.content, b"hello world hello world hello world");
}

#[test]
fn deflate_codec_roundtrip_matches_original_content() {
    let mut out = Vec::new();
    write_blob_record(
        &mut out,
        b"hello world hello world hello world",
        Codec::DEFLATE,
        1,
    )
    .unwrap();
    let read = read_blob_record(&mut Cursor::new(out)).unwrap();
    assert_eq!(read.content, b"hello world hello world hello world");
}

#[test]
fn bzip2_codec_roundtrip_matches_original_content() {
    let mut out = Vec::new();
    write_blob_record(
        &mut out,
        b"hello world hello world hello world",
        Codec::BZIP2,
        1,
    )
    .unwrap();
    let read = read_blob_record(&mut Cursor::new(out)).unwrap();
    assert_eq!(read.content, b"hello world hello world hello world");
}

#[test]
fn ppmd_codec_roundtrip_matches_original_content() {
    let mut out = Vec::new();
    write_blob_record(
        &mut out,
        b"alpha alpha alpha beta beta beta gamma gamma gamma",
        Codec::PPMD,
        1,
    )
    .unwrap();
    let read = read_blob_record(&mut Cursor::new(out)).unwrap();
    assert_eq!(
        read.content,
        b"alpha alpha alpha beta beta beta gamma gamma gamma"
    );
}

#[test]
fn lzma2_codec_roundtrip_matches_original_content() {
    let mut out = Vec::new();
    write_blob_record(
        &mut out,
        b"hello world hello world hello world",
        Codec::LZMA2,
        1,
    )
    .unwrap();
    let read = read_blob_record(&mut Cursor::new(out)).unwrap();
    assert_eq!(read.content, b"hello world hello world hello world");
}

#[test]
fn copy_blob_record_content_to_writer_streams_lz4_content() {
    let mut out = Vec::new();
    write_blob_record(
        &mut out,
        b"hello world hello world hello world",
        Codec::LZ4,
        1,
    )
    .unwrap();
    let mut copied = Vec::new();
    let result = copy_blob_record_content_to_writer(&mut Cursor::new(out), &mut copied).unwrap();
    assert_eq!(result.copied_bytes, 35);
    assert_eq!(copied, b"hello world hello world hello world");
}

#[test]
fn copy_blob_record_content_to_writer_streams_deflate_content() {
    let input = b"hello world hello world hello world";
    let mut out = Vec::new();
    write_blob_record(&mut out, input, Codec::DEFLATE, 1).unwrap();
    let mut copied = Vec::new();
    let result = copy_blob_record_content_to_writer(&mut Cursor::new(out), &mut copied).unwrap();
    assert_eq!(result.copied_bytes, input.len() as u64);
    assert_eq!(copied, input);
}

#[test]
fn copy_blob_record_content_to_writer_streams_bzip2_content() {
    let input = b"hello world hello world hello world";
    let mut out = Vec::new();
    write_blob_record(&mut out, input, Codec::BZIP2, 1).unwrap();
    let mut copied = Vec::new();
    let result = copy_blob_record_content_to_writer(&mut Cursor::new(out), &mut copied).unwrap();
    assert_eq!(result.copied_bytes, input.len() as u64);
    assert_eq!(copied, input);
}

#[test]
fn copy_blob_record_content_to_writer_streams_ppmd_content() {
    let input = b"alpha alpha alpha beta beta beta gamma gamma gamma";
    let mut out = Vec::new();
    write_blob_record(&mut out, input, Codec::PPMD, 1).unwrap();
    let mut copied = Vec::new();
    let result = copy_blob_record_content_to_writer(&mut Cursor::new(out), &mut copied).unwrap();
    assert_eq!(result.copied_bytes, input.len() as u64);
    assert_eq!(copied, input);
}

#[test]
fn copy_blob_record_content_to_writer_streams_ppmd_in_multiple_chunks() {
    let input = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda\n"
        .repeat(8192)
        .into_bytes();
    let mut out = Vec::new();
    write_blob_record(&mut out, &input, Codec::PPMD, 1).unwrap();

    let mut copied = ChunkLimitedWriter::new(16 * 1024);
    let result = copy_blob_record_content_to_writer(&mut Cursor::new(out), &mut copied).unwrap();

    assert_eq!(result.copied_bytes, input.len() as u64);
    assert_eq!(copied.data, input);
    assert!(copied.chunks > 1);
}

#[test]
fn copy_blob_record_content_to_writer_streams_lzma2_content() {
    let input = b"hello world hello world hello world";
    let mut out = Vec::new();
    write_blob_record(&mut out, input, Codec::LZMA2, 1).unwrap();
    let mut copied = Vec::new();
    let result = copy_blob_record_content_to_writer(&mut Cursor::new(out), &mut copied).unwrap();
    assert_eq!(result.copied_bytes, input.len() as u64);
    assert_eq!(copied, input);
}

#[test]
fn write_blob_record_falls_back_to_none_when_codec_is_not_beneficial() {
    let input = incompressible_bytes(4096);
    for codec in [
        Codec::LZ4,
        Codec::DEFLATE,
        Codec::BZIP2,
        Codec::PPMD,
        Codec::LZMA2,
    ] {
        let mut out = Vec::new();
        let result = write_blob_record(&mut out, &input, codec, 1).unwrap();
        assert_eq!(
            result.header.codec,
            Codec::NONE,
            "codec={:?}",
            u8::from(codec)
        );
    }
}

#[test]
fn mutating_data_payload_does_not_change_record_crc_but_breaks_hash_check() {
    let mut out = Vec::new();
    write_blob_record(
        &mut out,
        b"hello world hello world hello world",
        Codec::ZSTD,
        1,
    )
    .unwrap();
    let original_crc = decode_prefix(&out).record_crc;
    let last = out.len() - 1;
    out[last] ^= 0x01;
    let tampered_crc = decode_prefix(&out).record_crc;
    assert_eq!(original_crc, tampered_crc);
    assert!(matches!(
        read_blob_record(&mut Cursor::new(out)),
        Err(BlobRecordError::Codec(_) | BlobRecordError::BlobHashMismatch)
    ));
}

#[test]
fn read_blob_record_returns_original_plaintext() {
    let mut out = Vec::new();
    let write = write_blob_record(&mut out, b"plain", Codec::NONE, 1).unwrap();
    let read = read_blob_record(&mut Cursor::new(out)).unwrap();
    assert_eq!(
        read,
        BlobReadResult {
            header: write.header,
            record_len: write.record_len,
            content: b"plain".to_vec(),
        }
    );
}

#[test]
fn record_crc_mismatch_is_rejected() {
    let mut out = Vec::new();
    write_blob_record(&mut out, b"plain", Codec::NONE, 1).unwrap();
    out[9] ^= 0xFF;
    assert!(matches!(
        read_blob_record(&mut Cursor::new(out)),
        Err(BlobRecordError::BlobCrcMismatch)
    ));
}

#[test]
fn blob_hash_mismatch_is_rejected() {
    let mut out = Vec::new();
    write_blob_record(&mut out, b"plain", Codec::NONE, 1).unwrap();
    let payload_start = RECORD_PREFIX_SIZE + BLOB_HEADER_SIZE;
    out[payload_start] ^= 0xFF;
    assert!(matches!(
        read_blob_record(&mut Cursor::new(out)),
        Err(BlobRecordError::BlobHashMismatch)
    ));
}

#[test]
fn unknown_codec_is_rejected_during_read() {
    let mut out = Vec::new();
    let BlobWriteResult { record_len, .. } =
        write_blob_record(&mut out, b"plain", Codec::NONE, 1).unwrap();
    out[RECORD_PREFIX_SIZE + 33] = 0x80;
    let prefix = RecordPrefix {
        record_type: RecordType::BLOB,
        record_len,
        record_crc: xun::xunbak::record::compute_record_crc(
            RecordType::BLOB,
            record_len.to_le_bytes(),
            &out[RECORD_PREFIX_SIZE..RECORD_PREFIX_SIZE + BLOB_HEADER_SIZE],
        ),
    };
    out[..RECORD_PREFIX_SIZE].copy_from_slice(&prefix.to_bytes());
    assert!(matches!(
        read_blob_record(&mut Cursor::new(out)),
        Err(BlobRecordError::Codec(_))
    ));
}
