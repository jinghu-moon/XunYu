use std::fs;

use tempfile::tempdir;
use xun::xunbak::constants::{Codec, FLAG_SPLIT};
use xun::xunbak::reader::ContainerReader;
use xun::xunbak::verify::verify_quick_path;
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

fn split_options() -> BackupOptions {
    BackupOptions {
        codec: Codec::NONE,
        zstd_level: 1,
        split_size: Some(1900),
    }
}

fn split_update_options() -> BackupOptions {
    BackupOptions {
        codec: Codec::NONE,
        zstd_level: 1,
        split_size: Some(2800),
    }
}

#[test]
fn split_backup_creates_numbered_volumes() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "a".repeat(80)).unwrap();
    fs::write(source.join("b.txt"), "b".repeat(80)).unwrap();
    fs::write(source.join("c.txt"), "c".repeat(80)).unwrap();
    let base = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&base, &source, &split_options()).unwrap();
    assert!(dir.path().join("backup.xunbak.001").exists());
    assert!(dir.path().join("backup.xunbak.002").exists());
}

#[test]
fn split_reader_open_from_base_path_discovers_volume_set() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "a".repeat(80)).unwrap();
    fs::write(source.join("b.txt"), "b".repeat(80)).unwrap();
    fs::write(source.join("c.txt"), "c".repeat(80)).unwrap();
    let base = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&base, &source, &split_options()).unwrap();
    let reader = ContainerReader::open(&base).unwrap();
    assert!(reader.is_split);
    assert!(reader.header.header.flags & FLAG_SPLIT != 0);
    assert!(reader.volume_paths.len() >= 2);
    assert_eq!(
        reader.checkpoint.total_volumes as usize,
        reader.volume_paths.len()
    );
}

#[test]
fn split_restore_all_restores_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("a.txt"), "a".repeat(80)).unwrap();
    fs::write(source.join("nested").join("b.txt"), "b".repeat(80)).unwrap();
    fs::write(source.join("c.txt"), "c".repeat(80)).unwrap();
    let base = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&base, &source, &split_options()).unwrap();
    let reader = ContainerReader::open(&base).unwrap();
    let target = dir.path().join("restore");
    let result = reader.restore_all(&target).unwrap();
    assert_eq!(result.restored_files, 3);
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).unwrap(),
        "a".repeat(80)
    );
    assert_eq!(
        fs::read_to_string(target.join("nested").join("b.txt")).unwrap(),
        "b".repeat(80)
    );
}

#[test]
fn split_quick_verify_passes() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "a".repeat(80)).unwrap();
    fs::write(source.join("b.txt"), "b".repeat(80)).unwrap();
    fs::write(source.join("c.txt"), "c".repeat(80)).unwrap();
    let base = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&base, &source, &split_options()).unwrap();
    let report = verify_quick_path(&base);
    assert!(report.passed);
}

#[test]
fn split_update_reuses_old_blob_and_restores_new_state() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "a".repeat(180)).unwrap();
    fs::write(source.join("b.txt"), "b".repeat(180)).unwrap();
    fs::write(source.join("c.txt"), "c".repeat(180)).unwrap();
    fs::write(source.join("d.txt"), "d".repeat(180)).unwrap();
    fs::write(source.join("e.txt"), "e".repeat(180)).unwrap();
    let base = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&base, &source, &split_update_options()).unwrap();
    let manifest_before = ContainerReader::open(&base)
        .unwrap()
        .load_manifest()
        .unwrap();
    let old_a = manifest_before
        .entries
        .iter()
        .find(|entry| entry.path == "a.txt")
        .unwrap()
        .clone();

    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(source.join("f.txt"), "f".repeat(180)).unwrap();
    fs::write(source.join("b.txt"), "bb".repeat(120)).unwrap();
    let result = ContainerWriter::update(&base, &source, &split_update_options()).unwrap();
    assert_eq!(result.added_blob_count, 2);

    let reader = ContainerReader::open(&base).unwrap();
    let manifest_after = reader.load_manifest().unwrap();
    let new_a = manifest_after
        .entries
        .iter()
        .find(|entry| entry.path == "a.txt")
        .unwrap();
    assert_eq!(new_a.blob_offset, old_a.blob_offset);
    assert_eq!(new_a.volume_index, old_a.volume_index);
    assert!(
        manifest_after
            .entries
            .iter()
            .any(|entry| entry.path == "f.txt")
    );

    let target = dir.path().join("restore_after_update");
    let restored = reader.restore_all(&target).unwrap();
    assert_eq!(restored.restored_files, 6);
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).unwrap(),
        "a".repeat(180)
    );
    assert_eq!(
        fs::read_to_string(target.join("b.txt")).unwrap(),
        "bb".repeat(120)
    );
    assert_eq!(
        fs::read_to_string(target.join("f.txt")).unwrap(),
        "f".repeat(180)
    );
}
