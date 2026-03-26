use std::fs;
use std::io::{Cursor, Seek, SeekFrom};

use tempfile::tempdir;
use ulid::Ulid;
use xun::xunbak::checkpoint::read_checkpoint_record;
use xun::xunbak::constants::{Codec, FOOTER_SIZE, HEADER_SIZE, RecordType};
use xun::xunbak::footer::Footer;
use xun::xunbak::manifest::read_manifest_record;
use xun::xunbak::reader::ContainerReader;
use xun::xunbak::record::scan_records;
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

fn large_test_bytes(size: usize) -> Vec<u8> {
    let mut value = 0x1234_5678u32;
    let mut out = Vec::with_capacity(size);
    for _ in 0..size {
        value = value.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        out.push((value >> 24) as u8);
    }
    out
}

#[test]
fn backup_directory_writes_three_blobs_manifest_and_checkpoint() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    fs::write(source.join("b.txt"), "bbb").unwrap();
    fs::write(source.join("c.txt"), "ccc").unwrap();
    let container = dir.path().join("backup.xunbak");

    let result = ContainerWriter::backup(
        &container,
        &source,
        &BackupOptions {
            codec: Codec::NONE,
            auto_compression: false,
            zstd_level: 1,
            split_size: None,
        },
    )
    .unwrap();
    assert_eq!(result.file_count, 3);
    assert_eq!(result.blob_count, 3);

    let bytes = fs::read(&container).unwrap();
    let mut cursor = Cursor::new(&bytes[HEADER_SIZE..bytes.len() - FOOTER_SIZE]);
    let scanned = scan_records(&mut cursor).unwrap();
    assert_eq!(scanned.len(), 5);
    assert_eq!(scanned[0].record_type, RecordType::BLOB);
    assert_eq!(scanned[1].record_type, RecordType::BLOB);
    assert_eq!(scanned[2].record_type, RecordType::BLOB);
    assert_eq!(scanned[3].record_type, RecordType::MANIFEST);
    assert_eq!(scanned[4].record_type, RecordType::CHECKPOINT);
}

#[test]
fn backup_manifest_entries_have_correct_offsets_and_hashes() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    fs::write(source.join("b.txt"), "bbb").unwrap();
    fs::write(source.join("c.txt"), "ccc").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(
        &container,
        &source,
        &BackupOptions {
            codec: Codec::NONE,
            auto_compression: false,
            zstd_level: 1,
            split_size: None,
        },
    )
    .unwrap();
    let bytes = fs::read(&container).unwrap();
    let footer =
        Footer::from_bytes(&bytes[bytes.len() - FOOTER_SIZE..], bytes.len() as u64).unwrap();
    let mut cursor = Cursor::new(bytes.as_slice());
    cursor
        .seek(SeekFrom::Start(footer.checkpoint_offset))
        .unwrap();
    let checkpoint = read_checkpoint_record(&mut cursor).unwrap();
    cursor
        .seek(SeekFrom::Start(checkpoint.payload.manifest_offset))
        .unwrap();
    let manifest = read_manifest_record(&mut cursor).unwrap();

    assert_eq!(manifest.body.entries.len(), 3);
    for entry in &manifest.body.entries {
        let content = fs::read(source.join(entry.path.replace('/', "\\"))).unwrap();
        assert_eq!(entry.content_hash, *blake3::hash(&content).as_bytes());
        assert!(entry.blob_offset >= HEADER_SIZE as u64);
        assert_eq!(entry.blob_len, 13 + (50 + content.len()) as u64);
    }
}

#[test]
fn backup_checkpoint_stats_match_written_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    fs::write(source.join("b.txt"), "bbb").unwrap();
    fs::write(source.join("c.txt"), "ccc").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(
        &container,
        &source,
        &BackupOptions {
            codec: Codec::NONE,
            auto_compression: false,
            zstd_level: 1,
            split_size: None,
        },
    )
    .unwrap();
    let bytes = fs::read(&container).unwrap();
    let footer =
        Footer::from_bytes(&bytes[bytes.len() - FOOTER_SIZE..], bytes.len() as u64).unwrap();
    let mut cursor = Cursor::new(bytes);
    cursor
        .seek(SeekFrom::Start(footer.checkpoint_offset))
        .unwrap();
    let checkpoint = read_checkpoint_record(&mut cursor).unwrap();
    assert_eq!(checkpoint.payload.blob_count, 3);
    assert_eq!(
        checkpoint.payload.total_container_bytes,
        cursor.get_ref().len() as u64
    );
}

#[test]
fn backup_reuses_blob_for_duplicate_content_in_same_run() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("a.txt"), "same content").unwrap();
    fs::write(source.join("nested").join("b.txt"), "same content").unwrap();
    let container = dir.path().join("backup.xunbak");

    let result = ContainerWriter::backup(
        &container,
        &source,
        &BackupOptions {
            codec: Codec::NONE,
            auto_compression: false,
            zstd_level: 1,
            split_size: None,
        },
    )
    .unwrap();
    assert_eq!(result.file_count, 2);
    assert_eq!(result.blob_count, 1);

    let bytes = fs::read(&container).unwrap();
    let footer =
        Footer::from_bytes(&bytes[bytes.len() - FOOTER_SIZE..], bytes.len() as u64).unwrap();
    let mut cursor = Cursor::new(bytes.as_slice());
    cursor
        .seek(SeekFrom::Start(footer.checkpoint_offset))
        .unwrap();
    let checkpoint = read_checkpoint_record(&mut cursor).unwrap();
    assert_eq!(checkpoint.payload.blob_count, 1);
    cursor
        .seek(SeekFrom::Start(checkpoint.payload.manifest_offset))
        .unwrap();
    let manifest = read_manifest_record(&mut cursor).unwrap();
    assert_eq!(manifest.body.entries.len(), 2);
    assert_eq!(
        manifest.body.entries[0].content_hash,
        manifest.body.entries[1].content_hash
    );
    assert_eq!(
        manifest.body.entries[0].blob_offset,
        manifest.body.entries[1].blob_offset
    );
    assert_eq!(
        manifest.body.entries[0].blob_len,
        manifest.body.entries[1].blob_len
    );
}

#[test]
fn backup_manifest_includes_snapshot_context_and_ulid_snapshot_id() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let bytes = fs::read(&container).unwrap();
    let footer =
        Footer::from_bytes(&bytes[bytes.len() - FOOTER_SIZE..], bytes.len() as u64).unwrap();
    let mut cursor = Cursor::new(bytes.as_slice());
    cursor
        .seek(SeekFrom::Start(footer.checkpoint_offset))
        .unwrap();
    let checkpoint = read_checkpoint_record(&mut cursor).unwrap();
    cursor
        .seek(SeekFrom::Start(checkpoint.payload.manifest_offset))
        .unwrap();
    let manifest = read_manifest_record(&mut cursor).unwrap();

    assert!(Ulid::from_string(&manifest.body.snapshot_id).is_ok());

    let context = manifest.body.snapshot_context.as_object().unwrap();
    let expected_hostname =
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown-host".to_string());
    let expected_username =
        std::env::var("USERNAME").unwrap_or_else(|_| "unknown-user".to_string());
    assert_eq!(
        context.get("hostname").and_then(|value| value.as_str()),
        Some(expected_hostname.as_str())
    );
    assert_eq!(
        context.get("username").and_then(|value| value.as_str()),
        Some(expected_username.as_str())
    );
    assert_eq!(
        context.get("os").and_then(|value| value.as_str()),
        Some(std::env::consts::OS)
    );
    assert_eq!(
        context
            .get("xunyu_version")
            .and_then(|value| value.as_str()),
        Some(env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn backup_large_file_uses_parts_and_restores_content() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    let large = large_test_bytes((32 * 1024 * 1024) + 1_048_576);
    fs::write(source.join("large.bin"), &large).unwrap();
    let container = dir.path().join("backup.xunbak");

    let result = ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    assert!(result.blob_count >= 2);

    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let entry = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "large.bin")
        .unwrap();
    let parts = entry
        .parts
        .as_ref()
        .expect("large file should use multipart");
    assert!(parts.len() >= 2);
    assert_eq!(reader.checkpoint.blob_count as usize, parts.len());

    let restored = reader.read_and_verify_blob(entry).unwrap();
    assert_eq!(restored, large);
}

#[test]
fn backup_large_duplicate_reuses_parts_in_same_run() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    let large = large_test_bytes((32 * 1024 * 1024) + 1_048_576);
    fs::write(source.join("a.bin"), &large).unwrap();
    fs::write(source.join("b.bin"), &large).unwrap();
    let container = dir.path().join("backup.xunbak");

    let result = ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let a = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "a.bin")
        .unwrap();
    let b = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "b.bin")
        .unwrap();
    let a_parts = a.parts.as_ref().expect("multipart expected");
    let b_parts = b.parts.as_ref().expect("multipart expected");

    assert_eq!(a_parts.len(), b_parts.len());
    assert_eq!(result.blob_count, a_parts.len());
    assert_eq!(reader.checkpoint.blob_count as usize, a_parts.len());
    assert_eq!(a_parts, b_parts);
}
