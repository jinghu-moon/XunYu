use xun::xunbak::constants::{
    Codec, FOOTER_MAGIC, FOOTER_SIZE, HEADER_MAGIC, HEADER_SIZE, RECORD_PREFIX_SIZE, RecordType,
};

#[test]
fn magic_constants_match_design() {
    assert_eq!(&HEADER_MAGIC, b"XUNBAK\0\0");
    assert_eq!(&FOOTER_MAGIC, b"XBKFTR\0\0");
}

#[test]
fn fixed_sizes_match_design() {
    assert_eq!(HEADER_SIZE, 64);
    assert_eq!(FOOTER_SIZE, 24);
    assert_eq!(RECORD_PREFIX_SIZE, 13);
}

#[test]
fn record_type_u8_values_match_design() {
    assert_eq!(u8::from(RecordType::BLOB), 0x01);
    assert_eq!(u8::from(RecordType::MANIFEST), 0x02);
    assert_eq!(u8::from(RecordType::CHECKPOINT), 0x03);
}

#[test]
fn codec_u8_values_match_design() {
    assert_eq!(u8::from(Codec::NONE), 0x00);
    assert_eq!(u8::from(Codec::ZSTD), 0x01);
    assert_eq!(u8::from(Codec::LZ4), 0x02);
    assert_eq!(u8::from(Codec::LZMA2), 0x03);
    assert_eq!(u8::from(Codec::DEFLATE), 0x04);
    assert_eq!(u8::from(Codec::BZIP2), 0x05);
    assert_eq!(u8::from(Codec::PPMD), 0x06);
}

#[test]
fn unknown_record_type_is_safe() {
    let unknown = RecordType::from_u8(0xFF);
    assert_eq!(unknown.as_u8(), 0xFF);
    assert!(!unknown.is_known());
}

#[test]
fn unknown_codec_is_safe() {
    let unknown = Codec::from_u8(0x80);
    assert_eq!(unknown.as_u8(), 0x80);
    assert!(!unknown.is_known());
}
