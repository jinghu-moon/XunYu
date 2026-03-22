use std::io::Cursor;

use xun::xunbak::blob::{
    BlobReadResult, BlobRecordError, BlobWriteResult, read_blob_record, write_blob_record,
};
use xun::xunbak::constants::{BLOB_HEADER_SIZE, Codec, RECORD_PREFIX_SIZE, RecordType};
use xun::xunbak::record::RecordPrefix;

fn decode_prefix(bytes: &[u8]) -> RecordPrefix {
    RecordPrefix::from_bytes(&bytes[..RECORD_PREFIX_SIZE]).unwrap()
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
