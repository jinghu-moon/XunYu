use std::fs;
use std::io::{Cursor, Seek, SeekFrom};

use tempfile::tempdir;
use ulid::Ulid;
use xun::xunbak::checkpoint::read_checkpoint_record;
use xun::xunbak::constants::{Codec, FOOTER_SIZE, HEADER_SIZE, RecordType};
use xun::xunbak::footer::Footer;
use xun::xunbak::manifest::read_manifest_record;
use xun::xunbak::record::scan_records;
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

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
