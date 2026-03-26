use std::io::{Cursor, Read};

use xun::xunbak::codec::{
    CodecError, CompressionMode, compress, compression_is_beneficial, copy_decompressed_to_writer,
    decompress, effective_codec_after_compression, parse_compression_arg, should_skip_compress,
};
use xun::xunbak::constants::Codec;

fn sample_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

struct PanickingReader;

impl Read for PanickingReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        panic!("synthetic codec panic");
    }
}

fn assert_codec_roundtrip(codec: Codec, input: &[u8]) {
    let compressed = compress(codec, input, 1).unwrap();
    let decompressed = decompress(codec, &compressed).unwrap();
    assert_eq!(decompressed, input);
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
    assert_codec_roundtrip(Codec::ZSTD, b"hello world hello world hello world");
}

#[test]
fn zstd_codec_roundtrips_one_megabyte() {
    let input = sample_bytes(1024 * 1024);
    assert_codec_roundtrip(Codec::ZSTD, &input);
}

#[test]
fn lz4_codec_roundtrips_small_data() {
    assert_codec_roundtrip(Codec::LZ4, b"hello world hello world hello world");
}

#[test]
fn deflate_codec_roundtrips_small_data() {
    assert_codec_roundtrip(Codec::DEFLATE, b"hello world hello world hello world");
}

#[test]
fn bzip2_codec_roundtrips_small_data() {
    assert_codec_roundtrip(Codec::BZIP2, b"hello world hello world hello world");
}

#[test]
fn ppmd_codec_roundtrips_text_data() {
    assert_codec_roundtrip(
        Codec::PPMD,
        b"alpha alpha alpha beta beta beta gamma gamma gamma",
    );
}

#[test]
fn lzma2_codec_roundtrips_small_data() {
    assert_codec_roundtrip(Codec::LZMA2, b"hello world hello world hello world");
}

#[test]
fn lzma2_codec_roundtrips_one_megabyte() {
    let input = sample_bytes(1024 * 1024);
    assert_codec_roundtrip(Codec::LZMA2, &input);
}

#[test]
fn ppmd_codec_roundtrips_binary_data() {
    let input = sample_bytes(4096);
    assert_codec_roundtrip(Codec::PPMD, &input);
}

#[test]
fn copy_decompressed_to_writer_supports_all_current_codecs() {
    let input = b"hello world hello world hello world";
    for codec in [
        Codec::NONE,
        Codec::ZSTD,
        Codec::LZ4,
        Codec::DEFLATE,
        Codec::BZIP2,
        Codec::PPMD,
        Codec::LZMA2,
    ] {
        let compressed = compress(codec, input, 1).unwrap();
        let mut out = Vec::new();
        let copied = copy_decompressed_to_writer(codec, compressed.as_slice(), &mut out).unwrap();
        assert_eq!(copied, input.len() as u64);
        assert_eq!(out, input);
    }
}

#[test]
fn stream_hash_and_compress_supports_selected_extended_codecs() {
    let input = sample_bytes(1024 * 64);
    for codec in [
        Codec::LZ4,
        Codec::DEFLATE,
        Codec::BZIP2,
        Codec::PPMD,
        Codec::LZMA2,
    ] {
        let mut reader = Cursor::new(&input);
        let result =
            xun::xunbak::codec::stream_hash_and_compress(&mut reader, codec, 1, 8192).unwrap();
        assert_eq!(
            result.raw_size,
            input.len() as u64,
            "codec={:?}",
            u8::from(codec)
        );
        assert_eq!(result.hash, *blake3::hash(&input).as_bytes());
        let decompressed = decompress(codec, &result.compressed).unwrap();
        assert_eq!(decompressed, input, "codec={:?}", u8::from(codec));
    }
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
fn effective_codec_after_compression_reuses_shared_threshold_logic() {
    for codec in [
        Codec::ZSTD,
        Codec::LZ4,
        Codec::DEFLATE,
        Codec::BZIP2,
        Codec::PPMD,
        Codec::LZMA2,
    ] {
        assert_eq!(effective_codec_after_compression(codec, 100, 94), codec);
        assert_eq!(
            effective_codec_after_compression(codec, 100, 95),
            Codec::NONE,
            "codec={:?}",
            u8::from(codec)
        );
    }
    assert_eq!(
        effective_codec_after_compression(Codec::NONE, 100, 200),
        Codec::NONE
    );
}

#[test]
fn ppmd_text_corpus_has_clear_compression_gain() {
    let corpus = r#"
fn main() {
    let config = serde_json::json!({
        "name": "xunbak",
        "mode": "incremental",
        "paths": ["src", "tests", "docs"],
        "retry": 3,
        "enabled": true
    });
    println!("{}", config);
}
"#
    .repeat(1024);

    let ppmd = compress(Codec::PPMD, corpus.as_bytes(), 1).unwrap();

    assert!(
        compression_is_beneficial(corpus.len() as u64, ppmd.len() as u64),
        "ppmd={} raw={}",
        ppmd.len(),
        corpus.len()
    );
}

#[test]
fn ppmd_stream_hash_and_compress_converts_panics_into_encode_failed() {
    let err =
        xun::xunbak::codec::stream_hash_and_compress(&mut PanickingReader, Codec::PPMD, 1, 8192)
            .unwrap_err();

    match err {
        CodecError::EncodeFailed { codec, message } => {
            assert_eq!(codec, "ppmd");
            assert!(message.contains("panicked"));
            assert!(message.contains("synthetic codec panic"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn ppmd_copy_decompressed_to_writer_converts_panics_into_decode_failed() {
    let mut out = Vec::new();
    let err = copy_decompressed_to_writer(Codec::PPMD, PanickingReader, &mut out).unwrap_err();

    match err {
        CodecError::DecodeFailed { codec, message } => {
            assert_eq!(codec, "ppmd");
            assert!(message.contains("panicked"));
            assert!(message.contains("synthetic codec panic"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
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
    assert_eq!(parse_compression_arg("lzma2"), Ok(CompressionMode::Lzma2));
    assert_eq!(parse_compression_arg("lzma"), Ok(CompressionMode::Lzma2));
    assert_eq!(
        parse_compression_arg("deflate"),
        Ok(CompressionMode::Deflate)
    );
    assert_eq!(parse_compression_arg("bzip2"), Ok(CompressionMode::Bzip2));
    assert_eq!(parse_compression_arg("ppmd"), Ok(CompressionMode::Ppmd));
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
