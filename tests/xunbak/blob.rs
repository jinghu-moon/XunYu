use xun::xunbak::blob::{BlobHeader, BlobHeaderError};
use xun::xunbak::constants::{BLOB_HEADER_SIZE, Codec};

fn sample_blob_header() -> BlobHeader {
    BlobHeader {
        blob_id: [0xAB; 32],
        blob_flags: 0x05,
        codec: Codec::ZSTD,
        raw_size: 1234,
        stored_size: 567,
    }
}

#[test]
fn blob_header_serialization_matches_offsets() {
    let bytes = sample_blob_header().to_bytes();
    assert_eq!(bytes.len(), BLOB_HEADER_SIZE);
    assert_eq!(&bytes[..32], &[0xAB; 32]);
    assert_eq!(bytes[32], 0x05);
    assert_eq!(bytes[33], 0x01);
    assert_eq!(u64::from_le_bytes(bytes[34..42].try_into().unwrap()), 1234);
    assert_eq!(u64::from_le_bytes(bytes[42..50].try_into().unwrap()), 567);
}

#[test]
fn blob_header_roundtrip_preserves_fields() {
    let header = sample_blob_header();
    assert_eq!(BlobHeader::from_bytes(&header.to_bytes()).unwrap(), header);
}

#[test]
fn short_blob_header_is_rejected() {
    let bytes = [0u8; BLOB_HEADER_SIZE - 1];
    assert_eq!(
        BlobHeader::from_bytes(&bytes),
        Err(BlobHeaderError::HeaderTooShort {
            actual: BLOB_HEADER_SIZE - 1,
        })
    );
}
