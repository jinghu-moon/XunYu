use xun::xunbak::constants::{
    FLAG_ALIGNED, FLAG_SPLIT, HEADER_MAGIC, HEADER_SIZE, XUNBAK_READER_VERSION,
};
use xun::xunbak::header::{DecodedHeader, Header, HeaderError, SplitHeader};

fn sample_header() -> Header {
    Header {
        write_version: 7,
        min_reader_version: 1,
        flags: FLAG_SPLIT | FLAG_ALIGNED,
        created_at_unix: 1_700_000_000,
        split: Some(SplitHeader {
            volume_index: 3,
            split_size: 2 * 1024 * 1024,
            set_id: 0x1122_3344_5566_7788,
        }),
    }
}

#[test]
fn header_serialization_matches_layout() {
    let bytes = sample_header().to_bytes();
    assert_eq!(bytes.len(), HEADER_SIZE);
    assert_eq!(&bytes[..8], &HEADER_MAGIC);
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 7);
    assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 1);
    assert_eq!(
        u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        FLAG_SPLIT | FLAG_ALIGNED
    );
    assert_eq!(
        u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        1_700_000_000
    );
    assert_eq!(u16::from_le_bytes(bytes[32..34].try_into().unwrap()), 3);
    assert_eq!(
        u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
        2 * 1024 * 1024
    );
    assert_eq!(
        u64::from_le_bytes(bytes[48..56].try_into().unwrap()),
        0x1122_3344_5566_7788
    );
}

#[test]
fn header_roundtrip_preserves_fields() {
    let header = sample_header();
    let decoded = Header::from_bytes(&header.to_bytes()).unwrap();
    assert_eq!(
        decoded,
        DecodedHeader {
            header,
            unknown_flags: 0,
        }
    );
}

#[test]
fn header_write_and_reader_versions_are_at_expected_offsets() {
    let bytes = sample_header().to_bytes();
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 7);
    assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 1);
}

#[test]
fn header_flags_and_created_time_are_at_expected_offsets() {
    let bytes = sample_header().to_bytes();
    assert_eq!(
        u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        FLAG_SPLIT | FLAG_ALIGNED
    );
    assert_eq!(
        u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        1_700_000_000
    );
}

#[test]
fn nonsplit_header_zeroes_reserved_region() {
    let header = Header {
        write_version: 1,
        min_reader_version: 1,
        flags: 0,
        created_at_unix: 42,
        split: None,
    };
    let bytes = header.to_bytes();
    assert!(bytes[32..64].iter().all(|byte| *byte == 0));
}

#[test]
fn invalid_magic_is_rejected() {
    let mut bytes = sample_header().to_bytes();
    bytes[0] = b'Z';
    assert_eq!(Header::from_bytes(&bytes), Err(HeaderError::InvalidMagic));
}

#[test]
fn short_header_is_rejected() {
    let bytes = [0u8; HEADER_SIZE - 1];
    assert_eq!(
        Header::from_bytes(&bytes),
        Err(HeaderError::HeaderTooShort {
            actual: HEADER_SIZE - 1,
        })
    );
}

#[test]
fn too_new_reader_version_is_rejected() {
    let mut bytes = sample_header().to_bytes();
    bytes[12..16].copy_from_slice(&(XUNBAK_READER_VERSION + 1).to_le_bytes());
    assert_eq!(
        Header::from_bytes(&bytes),
        Err(HeaderError::VersionTooNew {
            min_reader_version: XUNBAK_READER_VERSION + 1,
            current: XUNBAK_READER_VERSION,
        })
    );
}

#[test]
fn unknown_flags_return_warning_bits_without_panicking() {
    let mut header = sample_header();
    header.flags |= 0x80;
    let decoded = Header::from_bytes(&header.to_bytes()).unwrap();
    assert_eq!(decoded.header.flags, FLAG_SPLIT | FLAG_ALIGNED | 0x80);
    assert_eq!(decoded.unknown_flags, 0x80);
}

#[test]
fn split_fields_roundtrip_when_split_flag_is_enabled() {
    let decoded = Header::from_bytes(&sample_header().to_bytes()).unwrap();
    assert_eq!(
        decoded.header.split,
        Some(SplitHeader {
            volume_index: 3,
            split_size: 2 * 1024 * 1024,
            set_id: 0x1122_3344_5566_7788,
        })
    );
}

#[test]
fn split_fields_stay_zeroed_when_split_flag_is_disabled() {
    let header = Header {
        write_version: 1,
        min_reader_version: 1,
        flags: 0,
        created_at_unix: 42,
        split: Some(SplitHeader {
            volume_index: 9,
            split_size: 123,
            set_id: 456,
        }),
    };
    let bytes = header.to_bytes();
    assert!(bytes[32..64].iter().all(|byte| *byte == 0));
    let decoded = Header::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.header.split, None);
}
