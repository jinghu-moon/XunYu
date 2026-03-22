use xun::xunbak::constants::{RECORD_PREFIX_SIZE, RecordType};
use xun::xunbak::record::{
    RecordPrefix, RecordScanError, ScannedRecord, compute_record_crc, scan_records,
};

fn make_record(record_type: RecordType, payload: &[u8]) -> Vec<u8> {
    let record_len = payload.len() as u64;
    let prefix = RecordPrefix {
        record_type,
        record_len,
        record_crc: compute_record_crc(record_type, record_len.to_le_bytes(), payload),
    };
    let mut bytes = prefix.to_bytes().to_vec();
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn record_prefix_serializes_to_13_bytes() {
    let prefix = RecordPrefix {
        record_type: RecordType::BLOB,
        record_len: 123,
        record_crc: 0x1122_3344,
    };
    let bytes = prefix.to_bytes();
    assert_eq!(bytes.len(), RECORD_PREFIX_SIZE);
}

#[test]
fn record_prefix_layout_is_little_endian() {
    let prefix = RecordPrefix {
        record_type: RecordType::MANIFEST,
        record_len: 0x0102_0304_0506_0708,
        record_crc: 0x1122_3344,
    };
    let bytes = prefix.to_bytes();
    assert_eq!(bytes[0], 0x02);
    assert_eq!(
        u64::from_le_bytes(bytes[1..9].try_into().unwrap()),
        0x0102_0304_0506_0708
    );
    assert_eq!(
        u32::from_le_bytes(bytes[9..13].try_into().unwrap()),
        0x1122_3344
    );
}

#[test]
fn record_prefix_roundtrip_matches_fields() {
    let prefix = RecordPrefix {
        record_type: RecordType::CHECKPOINT,
        record_len: 42,
        record_crc: 77,
    };
    assert_eq!(
        RecordPrefix::from_bytes(&prefix.to_bytes()).unwrap(),
        prefix
    );
}

#[test]
fn blob_crc_covers_type_len_and_blob_header_only() {
    let payload_for_crc = [0xAB; 50];
    let crc = compute_record_crc(RecordType::BLOB, 50u64.to_le_bytes(), &payload_for_crc);
    let mut changed = payload_for_crc;
    changed[0] ^= 0xFF;
    let changed_crc = compute_record_crc(RecordType::BLOB, 50u64.to_le_bytes(), &changed);
    assert_ne!(crc, changed_crc);
}

#[test]
fn manifest_crc_covers_type_len_and_full_payload() {
    let payload = br#"{"kind":"manifest"}"#;
    let crc = compute_record_crc(
        RecordType::MANIFEST,
        (payload.len() as u64).to_le_bytes(),
        payload,
    );
    let mut changed = payload.to_vec();
    changed[0] ^= 1;
    let changed_crc = compute_record_crc(
        RecordType::MANIFEST,
        (changed.len() as u64).to_le_bytes(),
        &changed,
    );
    assert_ne!(crc, changed_crc);
}

#[test]
fn checkpoint_crc_uses_same_strategy_as_manifest() {
    let payload = [0x7Bu8; 128];
    let crc = compute_record_crc(RecordType::CHECKPOINT, 128u64.to_le_bytes(), &payload);
    let mut changed = payload;
    changed[127] ^= 0x55;
    let changed_crc = compute_record_crc(RecordType::CHECKPOINT, 128u64.to_le_bytes(), &changed);
    assert_ne!(crc, changed_crc);
}

#[test]
fn empty_payload_crc_does_not_panic() {
    let _ = compute_record_crc(RecordType::MANIFEST, 0u64.to_le_bytes(), &[]);
}

#[test]
fn scan_records_returns_offsets_types_and_lengths() {
    let bytes = [
        make_record(RecordType::BLOB, b"abc"),
        make_record(RecordType::MANIFEST, b"hello"),
        make_record(RecordType::CHECKPOINT, b"xyz123"),
    ]
    .concat();
    let mut reader = std::io::Cursor::new(bytes);
    let scanned = scan_records(&mut reader).unwrap();
    assert_eq!(
        scanned,
        vec![
            ScannedRecord {
                offset: 0,
                record_type: RecordType::BLOB,
                record_len: 3
            },
            ScannedRecord {
                offset: 16,
                record_type: RecordType::MANIFEST,
                record_len: 5
            },
            ScannedRecord {
                offset: 34,
                record_type: RecordType::CHECKPOINT,
                record_len: 6
            }
        ]
    );
}

#[test]
fn scan_records_stops_on_crc_mismatch_and_keeps_previous_records() {
    let mut second = make_record(RecordType::MANIFEST, b"hello");
    second[9] ^= 0xFF;
    let bytes = [
        make_record(RecordType::BLOB, b"abc"),
        second,
        make_record(RecordType::CHECKPOINT, b"xyz123"),
    ]
    .concat();
    let mut reader = std::io::Cursor::new(bytes);
    let scanned = scan_records(&mut reader).unwrap();
    assert_eq!(
        scanned,
        vec![ScannedRecord {
            offset: 0,
            record_type: RecordType::BLOB,
            record_len: 3
        }]
    );
}

#[test]
fn scan_records_reports_truncated_record() {
    let mut bytes = make_record(RecordType::BLOB, b"abc");
    bytes.truncate(bytes.len() - 1);
    let mut reader = std::io::Cursor::new(bytes);
    assert_eq!(
        scan_records(&mut reader),
        Err(RecordScanError::TruncatedRecord {
            offset: 0,
            record_len: 3,
        })
    );
}
