use xun::xunbak::codec::{
    CodecError, CompressionMode, compress, compression_is_beneficial, decompress,
    parse_compression_arg, should_skip_compress,
};
use xun::xunbak::constants::Codec;

fn sample_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

#[test]
fn none_codec_roundtrips_without_changes() {
    let input = b"hello xunbak";
    let compressed = compress(Codec::NONE, input, 1).unwrap();
    assert_eq!(compressed, input);
    let decompressed = decompress(Codec::NONE, &compressed).unwrap();
    assert_eq!(decompressed, input);
}

#[test]
fn zstd_codec_roundtrips_empty_data() {
    let compressed = compress(Codec::ZSTD, &[], 1).unwrap();
    let decompressed = decompress(Codec::ZSTD, &compressed).unwrap();
    assert_eq!(decompressed, Vec::<u8>::new());
}

#[test]
fn zstd_codec_roundtrips_small_data() {
    let input = b"hello world hello world hello world";
    let compressed = compress(Codec::ZSTD, input, 1).unwrap();
    let decompressed = decompress(Codec::ZSTD, &compressed).unwrap();
    assert_eq!(decompressed, input);
}

#[test]
fn zstd_codec_roundtrips_one_megabyte() {
    let input = sample_bytes(1024 * 1024);
    let compressed = compress(Codec::ZSTD, &input, 1).unwrap();
    let decompressed = decompress(Codec::ZSTD, &compressed).unwrap();
    assert_eq!(decompressed, input);
}

#[test]
fn lz4_and_lzma_are_explicitly_unsupported_for_now() {
    let input = b"abc";
    assert_eq!(
        compress(Codec::LZ4, input, 1),
        Err(CodecError::UnsupportedCodec(Codec::LZ4.as_u8()))
    );
    assert_eq!(
        decompress(Codec::LZMA, input),
        Err(CodecError::UnsupportedCodec(Codec::LZMA.as_u8()))
    );
}

#[test]
fn skip_compress_matches_known_extensions() {
    for ext in ["jpg", ".zip", "mkv", ".mp4", ".zst", ".br", ".lz4", ".bz2"] {
        assert!(should_skip_compress(ext), "{ext} should be skipped");
    }
    for ext in ["rs", "txt", "json", ".pdf"] {
        assert!(
            !should_skip_compress(ext),
            "{ext} should not be force-skipped"
        );
    }
}

#[test]
fn compression_benefit_uses_ninety_five_percent_threshold() {
    assert!(compression_is_beneficial(100, 94));
    assert!(!compression_is_beneficial(100, 95));
    assert!(!compression_is_beneficial(0, 0));
}

#[test]
fn parse_compression_arg_supports_expected_forms() {
    assert_eq!(parse_compression_arg("none"), Ok(CompressionMode::None));
    assert_eq!(
        parse_compression_arg("zstd"),
        Ok(CompressionMode::Zstd { level: 1 })
    );
    assert_eq!(
        parse_compression_arg("zstd:9"),
        Ok(CompressionMode::Zstd { level: 9 })
    );
    assert_eq!(parse_compression_arg("lz4"), Ok(CompressionMode::Lz4));
    assert_eq!(parse_compression_arg("lzma"), Ok(CompressionMode::Lzma));
    assert_eq!(parse_compression_arg("auto"), Ok(CompressionMode::Auto));
}

#[test]
fn parse_compression_arg_rejects_invalid_values() {
    assert!(matches!(
        parse_compression_arg(""),
        Err(CodecError::InvalidCompressionArg(_))
    ));
    assert!(matches!(
        parse_compression_arg("zstd:abc"),
        Err(CodecError::InvalidCompressionArg(_))
    ));
    assert!(matches!(
        parse_compression_arg("brotli"),
        Err(CodecError::InvalidCompressionArg(_))
    ));
}
