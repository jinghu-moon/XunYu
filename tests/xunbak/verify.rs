use std::fs;

use serde_json::Value;
use tempfile::tempdir;
use xun::xunbak::constants::{
    BLOB_HEADER_SIZE, CHECKPOINT_PAYLOAD_SIZE, FOOTER_SIZE, HEADER_SIZE, RECORD_PREFIX_SIZE,
    RecordType,
};
use xun::xunbak::reader::ContainerReader;
use xun::xunbak::record::compute_record_crc;
use xun::xunbak::verify::{
    VerifyLevel, verify_existence_only_path, verify_full_path, verify_manifest_only_path,
    verify_paranoid_path, verify_quick_path,
};
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

#[test]
fn quick_verify_passes_for_valid_container() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let report = verify_quick_path(&container);
    assert!(report.passed);
    assert_eq!(report.level, VerifyLevel::Quick);
}

#[test]
fn quick_verify_passes_via_footer_fallback() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let mut bytes = fs::read(&container).unwrap();
    let footer_index = bytes.len() - FOOTER_SIZE;
    bytes[footer_index] ^= 0xFF;
    fs::write(&container, bytes).unwrap();

    let report = verify_quick_path(&container);
    assert!(report.passed);
}

#[test]
fn quick_verify_fails_when_checkpoint_crc_is_damaged() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    let mut bytes = fs::read(&container).unwrap();
    let payload_crc_index = reader.footer.checkpoint_offset as usize + RECORD_PREFIX_SIZE + 124;
    bytes[payload_crc_index] ^= 0xFF;
    fs::write(&container, bytes).unwrap();

    let report = verify_quick_path(&container);
    assert!(!report.passed);
}

#[test]
fn quick_verify_fails_when_manifest_hash_mismatches() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    let checkpoint_offset = reader.footer.checkpoint_offset as usize;
    let payload_offset = checkpoint_offset + RECORD_PREFIX_SIZE;

    let mut bytes = fs::read(&container).unwrap();
    let payload = &mut bytes[payload_offset..payload_offset + CHECKPOINT_PAYLOAD_SIZE];
    payload[32] ^= 0x01;
    let payload_crc = crc32c::crc32c(&payload[..124]);
    payload[124..128].copy_from_slice(&payload_crc.to_le_bytes());
    let record_crc = compute_record_crc(
        RecordType::CHECKPOINT,
        (CHECKPOINT_PAYLOAD_SIZE as u64).to_le_bytes(),
        payload,
    );
    bytes[checkpoint_offset + 9..checkpoint_offset + 13].copy_from_slice(&record_crc.to_le_bytes());
    fs::write(&container, bytes).unwrap();

    let report = verify_quick_path(&container);
    assert!(!report.passed);
}

#[test]
fn quick_verify_fails_when_manifest_length_mismatches() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    let checkpoint_offset = reader.footer.checkpoint_offset as usize;
    let payload_offset = checkpoint_offset + RECORD_PREFIX_SIZE;

    let mut bytes = fs::read(&container).unwrap();
    let payload = &mut bytes[payload_offset..payload_offset + CHECKPOINT_PAYLOAD_SIZE];
    let manifest_len = reader.checkpoint.manifest_len - 1;
    payload[24..32].copy_from_slice(&manifest_len.to_le_bytes());
    let payload_crc = crc32c::crc32c(&payload[..124]);
    payload[124..128].copy_from_slice(&payload_crc.to_le_bytes());
    let record_crc = compute_record_crc(
        RecordType::CHECKPOINT,
        (CHECKPOINT_PAYLOAD_SIZE as u64).to_le_bytes(),
        payload,
    );
    bytes[checkpoint_offset + 9..checkpoint_offset + 13].copy_from_slice(&record_crc.to_le_bytes());
    fs::write(&container, bytes).unwrap();

    let report = verify_quick_path(&container);
    assert!(!report.passed);
}

#[test]
fn quick_verify_fails_when_manifest_cannot_be_parsed() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    let mut bytes = fs::read(&container).unwrap();
    let manifest_offset = reader.checkpoint.manifest_offset as usize;
    let manifest_len = reader.checkpoint.manifest_len as usize;
    let payload_offset = manifest_offset + RECORD_PREFIX_SIZE;
    let payload_end = manifest_offset + manifest_len;
    let (manifest_hash, manifest_record_crc) = {
        let payload = &mut bytes[payload_offset..payload_end];
        payload[4] = b'!';
        let crc = compute_record_crc(
            RecordType::MANIFEST,
            ((manifest_len - RECORD_PREFIX_SIZE) as u64).to_le_bytes(),
            payload,
        );
        (*blake3::hash(payload).as_bytes(), crc)
    };
    bytes[manifest_offset + 9..manifest_offset + 13]
        .copy_from_slice(&manifest_record_crc.to_le_bytes());

    let checkpoint_offset = reader.footer.checkpoint_offset as usize;
    let checkpoint_payload_offset = checkpoint_offset + RECORD_PREFIX_SIZE;
    let checkpoint_payload =
        &mut bytes[checkpoint_payload_offset..checkpoint_payload_offset + CHECKPOINT_PAYLOAD_SIZE];
    checkpoint_payload[32..64].copy_from_slice(&manifest_hash);
    let payload_crc = crc32c::crc32c(&checkpoint_payload[..124]);
    checkpoint_payload[124..128].copy_from_slice(&payload_crc.to_le_bytes());
    let checkpoint_record_crc = compute_record_crc(
        RecordType::CHECKPOINT,
        (CHECKPOINT_PAYLOAD_SIZE as u64).to_le_bytes(),
        checkpoint_payload,
    );
    bytes[checkpoint_offset + 9..checkpoint_offset + 13]
        .copy_from_slice(&checkpoint_record_crc.to_le_bytes());
    fs::write(&container, bytes).unwrap();

    let report = verify_quick_path(&container);
    assert!(!report.passed);
}

#[test]
fn quick_verify_rejects_too_small_container() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("small.xunbak");
    fs::write(&container, [0u8; 10]).unwrap();
    let report = verify_quick_path(&container);
    assert!(!report.passed);
}

#[test]
fn full_verify_passes_when_all_blobs_are_valid() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    fs::write(source.join("b.txt"), "bbb").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let report = verify_full_path(&container);
    assert!(report.passed);
    assert_eq!(report.level, VerifyLevel::Full);
    assert_eq!(report.stats.manifest_entries, 2);
}

#[test]
fn manifest_only_verify_passes_for_valid_container() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let report = verify_manifest_only_path(&container);
    assert!(report.passed);
    assert_eq!(report.level, VerifyLevel::ManifestOnly);
}

#[test]
fn existence_only_verify_passes_for_valid_split_container() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "a".repeat(1200)).unwrap();
    fs::write(source.join("b.txt"), "b".repeat(1200)).unwrap();
    let container = dir.path().join("backup.xunbak");
    let options = BackupOptions {
        split_size: Some(1900),
        ..BackupOptions::default()
    };
    ContainerWriter::backup(&container, &source, &options).unwrap();

    let report = verify_existence_only_path(&container);
    assert!(report.passed);
    assert_eq!(report.level, VerifyLevel::ExistenceOnly);
}

#[test]
fn existence_only_verify_reports_missing_split_volume() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "a".repeat(1200)).unwrap();
    fs::write(source.join("b.txt"), "b".repeat(1200)).unwrap();
    let container = dir.path().join("backup.xunbak");
    let options = BackupOptions {
        split_size: Some(1900),
        ..BackupOptions::default()
    };
    ContainerWriter::backup(&container, &source, &options).unwrap();
    let last_volume = fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("backup.xunbak."))
        })
        .max()
        .unwrap();
    fs::remove_file(last_volume).unwrap();

    let report = verify_existence_only_path(&container);
    assert!(!report.passed);
    assert_eq!(report.level, VerifyLevel::ExistenceOnly);
    assert!(!report.errors.is_empty());
}

#[test]
fn full_verify_reports_corrupted_blob_with_path_and_offset() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let entry = &manifest.entries[0];

    let mut bytes = fs::read(&container).unwrap();
    let data_start = entry.blob_offset as usize + RECORD_PREFIX_SIZE + BLOB_HEADER_SIZE;
    bytes[data_start] ^= 0xFF;
    fs::write(&container, bytes).unwrap();

    let report = verify_full_path(&container);
    assert!(!report.passed);
    assert_eq!(report.errors[0].path.as_deref(), Some("a.txt"));
    assert_eq!(report.errors[0].offset, Some(entry.blob_offset));
}

#[test]
fn full_verify_reports_corrupted_lz4_blob_with_path_and_offset() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("a.txt"),
        "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512),
    )
    .unwrap();
    let container = dir.path().join("backup.xunbak");
    let options = BackupOptions {
        codec: xun::xunbak::constants::Codec::LZ4,
        auto_compression: false,
        zstd_level: 1,
        split_size: None,
    };
    ContainerWriter::backup(&container, &source, &options).unwrap();
    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let entry = &manifest.entries[0];

    let mut bytes = fs::read(&container).unwrap();
    let data_start = entry.blob_offset as usize + RECORD_PREFIX_SIZE + BLOB_HEADER_SIZE;
    bytes[data_start] ^= 0xFF;
    fs::write(&container, bytes).unwrap();

    let report = verify_full_path(&container);
    assert!(!report.passed);
    assert_eq!(report.errors[0].path.as_deref(), Some("a.txt"));
    assert_eq!(report.errors[0].offset, Some(entry.blob_offset));
}

#[test]
fn full_verify_reports_corrupted_extended_codec_blobs_with_path_and_offset() {
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for codec in [
        xun::xunbak::constants::Codec::DEFLATE,
        xun::xunbak::constants::Codec::BZIP2,
        xun::xunbak::constants::Codec::PPMD,
        xun::xunbak::constants::Codec::LZMA2,
    ] {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), &content).unwrap();
        let container = dir.path().join("backup.xunbak");
        let options = BackupOptions {
            codec,
            auto_compression: false,
            zstd_level: 1,
            split_size: None,
        };
        ContainerWriter::backup(&container, &source, &options).unwrap();
        let reader = ContainerReader::open(&container).unwrap();
        let manifest = reader.load_manifest().unwrap();
        let entry = &manifest.entries[0];

        let mut bytes = fs::read(&container).unwrap();
        let data_start = entry.blob_offset as usize + RECORD_PREFIX_SIZE + BLOB_HEADER_SIZE;
        bytes[data_start] ^= 0xFF;
        fs::write(&container, bytes).unwrap();

        let report = verify_full_path(&container);
        assert!(!report.passed, "codec={:?}", u8::from(codec));
        assert_eq!(report.errors[0].path.as_deref(), Some("a.txt"));
        assert_eq!(report.errors[0].offset, Some(entry.blob_offset));
    }
}

#[test]
fn full_verify_reports_codec_error() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let entry = &manifest.entries[0];

    let mut bytes = fs::read(&container).unwrap();
    let header_start = entry.blob_offset as usize + RECORD_PREFIX_SIZE;
    let codec_index = header_start + 33;
    bytes[codec_index] = 0x80;
    let header = bytes[header_start..header_start + BLOB_HEADER_SIZE].to_vec();
    let record_crc = compute_record_crc(
        RecordType::BLOB,
        (entry.blob_len - RECORD_PREFIX_SIZE as u64).to_le_bytes(),
        &header,
    );
    bytes[entry.blob_offset as usize + 9..entry.blob_offset as usize + 13]
        .copy_from_slice(&record_crc.to_le_bytes());
    fs::write(&container, bytes).unwrap();

    let report = verify_full_path(&container);
    assert!(!report.passed);
    assert_eq!(report.errors[0].path.as_deref(), Some("a.txt"));
}

#[test]
fn paranoid_verify_passes_for_valid_container() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let report = verify_paranoid_path(&container);
    assert!(report.passed);
    assert_eq!(report.level, VerifyLevel::Paranoid);
}

#[test]
fn paranoid_verify_reports_record_crc_mismatch_with_offset_and_type() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let mut bytes = fs::read(&container).unwrap();
    bytes[HEADER_SIZE + 9] ^= 0xFF;
    fs::write(&container, bytes).unwrap();

    let report = verify_paranoid_path(&container);
    assert!(!report.passed);
    assert_eq!(report.errors[0].offset, Some(HEADER_SIZE as u64));
    assert_eq!(report.errors[0].record_type, Some(RecordType::BLOB.as_u8()));
}

#[test]
fn verify_report_is_json_serializable() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let report = verify_quick_path(&container);
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(
        value.get("level").unwrap(),
        &Value::String("quick".to_string())
    );
    assert!(value.get("passed").is_some());
    assert!(value.get("errors").is_some());
    assert!(value.get("stats").is_some());
}
