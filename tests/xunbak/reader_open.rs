use std::fs;
use std::io::{Seek, SeekFrom, Write};

use tempfile::tempdir;
use xun::xunbak::constants::FOOTER_SIZE;
use xun::xunbak::reader::{ContainerReader, ReaderError};
use xun::xunbak::record::compute_record_crc;
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

#[test]
fn open_valid_container_loads_footer_and_checkpoint() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    assert_eq!(reader.checkpoint.blob_count, 1);
}

#[test]
fn too_small_container_is_rejected() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("small.xunbak");
    fs::write(&container, [0u8; 10]).unwrap();
    assert!(matches!(
        ContainerReader::open(&container),
        Err(ReaderError::ContainerTooSmall { .. })
    ));
}

#[test]
fn damaged_footer_magic_uses_fallback_scan() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&container)
        .unwrap();
    file.seek(SeekFrom::End(-(FOOTER_SIZE as i64))).unwrap();
    file.write_all(b"BROKEN!!").unwrap();
    drop(file);

    let reader = ContainerReader::open(&container).unwrap();
    assert_eq!(reader.checkpoint.blob_count, 1);
}

#[test]
fn damaged_footer_crc_uses_fallback_scan() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let mut bytes = fs::read(&container).unwrap();
    let crc_index = bytes.len() - FOOTER_SIZE + 16;
    bytes[crc_index] ^= 0xFF;
    fs::write(&container, bytes).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    assert_eq!(reader.checkpoint.blob_count, 1);
}

#[test]
fn load_manifest_from_checkpoint_succeeds() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    assert_eq!(manifest.entries.len(), 1);
}

#[test]
fn manifest_hash_mismatch_is_reported() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    let mut bytes = fs::read(&container).unwrap();
    let checkpoint_offset = reader.footer.checkpoint_offset as usize;
    let checkpoint_payload_offset = checkpoint_offset + xun::xunbak::constants::RECORD_PREFIX_SIZE;
    let checkpoint_payload = &mut bytes[checkpoint_payload_offset
        ..checkpoint_payload_offset + xun::xunbak::constants::CHECKPOINT_PAYLOAD_SIZE];
    checkpoint_payload[32] ^= 0x01;
    let payload_crc = crc32c::crc32c(&checkpoint_payload[..124]);
    checkpoint_payload[124..128].copy_from_slice(&payload_crc.to_le_bytes());
    let record_len = xun::xunbak::constants::CHECKPOINT_PAYLOAD_SIZE as u64;
    let record_crc = compute_record_crc(
        xun::xunbak::constants::RecordType::CHECKPOINT,
        record_len.to_le_bytes(),
        checkpoint_payload,
    );
    bytes[checkpoint_offset + 9..checkpoint_offset + 13].copy_from_slice(&record_crc.to_le_bytes());
    fs::write(&container, bytes).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    assert!(matches!(
        reader.load_manifest(),
        Err(ReaderError::ManifestHashMismatch)
    ));
}

#[test]
fn unrecoverable_container_is_reported_when_footer_and_records_are_damaged() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let mut bytes = fs::read(&container).unwrap();
    for byte in &mut bytes[xun::xunbak::constants::HEADER_SIZE..] {
        *byte = 0;
    }
    fs::write(&container, bytes).unwrap();

    assert!(matches!(
        ContainerReader::open(&container),
        Err(ReaderError::UnrecoverableContainer)
    ));
}
