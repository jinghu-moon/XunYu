use std::io::Cursor;

use xun::xunbak::checkpoint::{
    CheckpointError, CheckpointPayload, CheckpointReadResult, compute_manifest_hash,
    read_checkpoint_record, write_checkpoint_record,
};
use xun::xunbak::constants::{CHECKPOINT_PAYLOAD_SIZE, RecordType};
use xun::xunbak::record::RecordPrefix;

fn sample_payload() -> CheckpointPayload {
    CheckpointPayload {
        snapshot_id: [0xAB; 16],
        manifest_offset: 200,
        manifest_len: 80,
        manifest_hash: [0xCD; 32],
        container_end: 400,
        blob_count: 3,
        referenced_blob_bytes: 1024,
        total_container_bytes: 2048,
        prev_checkpoint_offset: 0,
        total_volumes: 1,
    }
}

#[test]
fn checkpoint_payload_serialization_matches_layout() {
    let bytes = sample_payload().to_bytes();
    assert_eq!(bytes.len(), CHECKPOINT_PAYLOAD_SIZE);
    assert_eq!(&bytes[..16], &[0xAB; 16]);
    assert_eq!(u64::from_le_bytes(bytes[16..24].try_into().unwrap()), 200);
    assert_eq!(u64::from_le_bytes(bytes[24..32].try_into().unwrap()), 80);
    assert_eq!(&bytes[32..64], &[0xCD; 32]);
    assert_eq!(u64::from_le_bytes(bytes[64..72].try_into().unwrap()), 400);
    assert_eq!(u64::from_le_bytes(bytes[72..80].try_into().unwrap()), 3);
    assert_eq!(u64::from_le_bytes(bytes[80..88].try_into().unwrap()), 1024);
    assert_eq!(u64::from_le_bytes(bytes[88..96].try_into().unwrap()), 2048);
    assert_eq!(u64::from_le_bytes(bytes[96..104].try_into().unwrap()), 0);
    assert_eq!(u16::from_le_bytes(bytes[104..106].try_into().unwrap()), 1);
    assert!(bytes[106..124].iter().all(|b| *b == 0));
}

#[test]
fn checkpoint_payload_roundtrip_succeeds() {
    let payload = sample_payload();
    assert_eq!(
        CheckpointPayload::from_bytes(&payload.to_bytes()).unwrap(),
        payload
    );
}

#[test]
fn checkpoint_crc_sits_at_expected_offset() {
    let bytes = sample_payload().to_bytes();
    let expected = crc32c::crc32c(&bytes[..124]);
    assert_eq!(
        u32::from_le_bytes(bytes[124..128].try_into().unwrap()),
        expected
    );
}

#[test]
fn checkpoint_record_layout_matches_design() {
    let mut out = Vec::new();
    let payload = sample_payload();
    let write = write_checkpoint_record(&mut out, &payload).unwrap();
    assert_eq!(out.len(), 13 + CHECKPOINT_PAYLOAD_SIZE);
    let prefix = RecordPrefix::from_bytes(&out[..13]).unwrap();
    assert_eq!(prefix.record_type, RecordType::CHECKPOINT);
    assert_eq!(prefix.record_len, CHECKPOINT_PAYLOAD_SIZE as u64);
    assert_eq!(write.record_len, CHECKPOINT_PAYLOAD_SIZE as u64);
}

#[test]
fn checkpoint_record_read_roundtrips() {
    let mut out = Vec::new();
    let payload = sample_payload();
    write_checkpoint_record(&mut out, &payload).unwrap();
    let read = read_checkpoint_record(&mut Cursor::new(out)).unwrap();
    assert_eq!(
        read,
        CheckpointReadResult {
            record_len: CHECKPOINT_PAYLOAD_SIZE as u64,
            payload,
        }
    );
}

#[test]
fn checkpoint_record_crc_and_payload_crc_are_independent() {
    let mut out = Vec::new();
    write_checkpoint_record(&mut out, &sample_payload()).unwrap();
    out[9] ^= 0x01;
    assert!(matches!(
        read_checkpoint_record(&mut Cursor::new(out)),
        Err(CheckpointError::RecordCrcMismatch)
    ));
}

#[test]
fn checkpoint_payload_crc_mismatch_is_rejected() {
    let mut bytes = sample_payload().to_bytes();
    bytes[124] ^= 0x01;
    assert!(matches!(
        CheckpointPayload::from_bytes(&bytes),
        Err(CheckpointError::CheckpointCrcMismatch)
    ));
}

#[test]
fn manifest_offset_out_of_range_is_rejected() {
    let payload = CheckpointPayload {
        manifest_offset: 390,
        manifest_len: 20,
        ..sample_payload()
    };
    assert!(matches!(
        CheckpointPayload::from_bytes(&payload.to_bytes()),
        Err(CheckpointError::ManifestOffsetOutOfRange { .. })
    ));
}

#[test]
fn manifest_hash_matches_payload_hash() {
    let manifest_payload = br#"{"manifest":"payload"}"#;
    let hash = compute_manifest_hash(manifest_payload);
    assert_eq!(hash, *blake3::hash(manifest_payload).as_bytes());
}
