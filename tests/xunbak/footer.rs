use xun::xunbak::constants::{FOOTER_MAGIC, FOOTER_SIZE};
use xun::xunbak::footer::{Footer, FooterError};

#[test]
fn footer_serializes_to_24_bytes() {
    let bytes = Footer {
        checkpoint_offset: 1234,
    }
    .to_bytes();
    assert_eq!(bytes.len(), FOOTER_SIZE);
}

#[test]
fn footer_layout_matches_design() {
    let bytes = Footer {
        checkpoint_offset: 0x1122_3344_5566_7788,
    }
    .to_bytes();
    assert_eq!(&bytes[..8], &FOOTER_MAGIC);
    assert_eq!(
        u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        0x1122_3344_5566_7788
    );
    let expected_crc = crc32c::crc32c(&bytes[..16]);
    assert_eq!(
        u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
        expected_crc
    );
    assert!(bytes[20..24].iter().all(|byte| *byte == 0));
}

#[test]
fn footer_crc_covers_magic_and_offset_only() {
    let bytes = Footer {
        checkpoint_offset: 42,
    }
    .to_bytes();
    let expected_crc = crc32c::crc32c(&bytes[..16]);
    assert_eq!(
        u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
        expected_crc
    );
}

#[test]
fn valid_footer_roundtrip_succeeds() {
    let footer = Footer {
        checkpoint_offset: 42,
    };
    let bytes = footer.to_bytes();
    assert_eq!(Footer::from_bytes(&bytes, 100).unwrap(), footer);
}

#[test]
fn invalid_magic_is_rejected() {
    let mut bytes = Footer {
        checkpoint_offset: 42,
    }
    .to_bytes();
    bytes[0] = b'Z';
    assert_eq!(
        Footer::from_bytes(&bytes, 100),
        Err(FooterError::InvalidFooterMagic)
    );
}

#[test]
fn footer_crc_mismatch_is_rejected() {
    let mut bytes = Footer {
        checkpoint_offset: 42,
    }
    .to_bytes();
    bytes[16] ^= 0xFF;
    assert_eq!(
        Footer::from_bytes(&bytes, 100),
        Err(FooterError::FooterCrcMismatch)
    );
}

#[test]
fn checkpoint_offset_out_of_range_is_rejected() {
    let bytes = Footer {
        checkpoint_offset: 100,
    }
    .to_bytes();
    assert_eq!(
        Footer::from_bytes(&bytes, 100),
        Err(FooterError::OffsetOutOfRange {
            checkpoint_offset: 100,
            file_size: 100,
        })
    );
}
