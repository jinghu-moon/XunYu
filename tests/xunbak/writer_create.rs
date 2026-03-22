use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom};

use tempfile::tempdir;
use xun::xunbak::checkpoint::read_checkpoint_record;
use xun::xunbak::constants::{FOOTER_SIZE, HEADER_SIZE};
use xun::xunbak::footer::Footer;
use xun::xunbak::header::Header;
use xun::xunbak::manifest::read_manifest_record;
use xun::xunbak::verify::{VerifyLevel, verify_quick_path};
use xun::xunbak::writer::ContainerWriter;

#[test]
fn create_container_writes_minimal_empty_container() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("empty.xunbak");
    ContainerWriter::create(&container).unwrap();
    let meta = fs::metadata(&container).unwrap();
    assert!(meta.len() >= (HEADER_SIZE + FOOTER_SIZE) as u64);
}

#[test]
fn created_container_has_valid_header() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("empty.xunbak");
    let writer = ContainerWriter::create(&container).unwrap();
    assert_eq!(writer.path(), container.as_path());

    let bytes = fs::read(&container).unwrap();
    let decoded = Header::from_bytes(&bytes[..HEADER_SIZE]).unwrap();
    assert_eq!(decoded.header.write_version, 1);
    assert_eq!(decoded.header.min_reader_version, 1);
}

#[test]
fn footer_points_to_valid_checkpoint() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("empty.xunbak");
    ContainerWriter::create(&container).unwrap();

    let bytes = fs::read(&container).unwrap();
    let footer =
        Footer::from_bytes(&bytes[bytes.len() - FOOTER_SIZE..], bytes.len() as u64).unwrap();
    assert!(footer.checkpoint_offset >= HEADER_SIZE as u64);
}

#[test]
fn empty_container_checkpoint_and_manifest_are_consistent() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("empty.xunbak");
    ContainerWriter::create(&container).unwrap();

    let mut reader = Cursor::new(fs::read(&container).unwrap());
    let footer_pos = reader.get_ref().len() as u64 - FOOTER_SIZE as u64;
    reader.seek(SeekFrom::Start(footer_pos)).unwrap();
    let mut footer_bytes = [0u8; FOOTER_SIZE];
    reader.read_exact(&mut footer_bytes).unwrap();
    let footer = Footer::from_bytes(&footer_bytes, reader.get_ref().len() as u64).unwrap();

    reader
        .seek(SeekFrom::Start(footer.checkpoint_offset))
        .unwrap();
    let checkpoint = read_checkpoint_record(&mut reader).unwrap();
    assert_eq!(checkpoint.payload.blob_count, 0);

    reader
        .seek(SeekFrom::Start(checkpoint.payload.manifest_offset))
        .unwrap();
    let manifest = read_manifest_record(&mut reader).unwrap();
    assert_eq!(manifest.body.file_count, 0);
    assert!(manifest.body.entries.is_empty());
    assert!(manifest.body.removed.is_empty());
}

#[test]
fn empty_container_quick_verify_passes() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("empty.xunbak");
    ContainerWriter::create(&container).unwrap();

    let report = verify_quick_path(&container);
    assert!(report.passed);
    assert_eq!(report.level, VerifyLevel::Quick);
}
