use std::fs;

use tempfile::tempdir;
use xun::xunbak::reader::{ContainerReader, ReaderError};
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

#[test]
fn read_and_verify_blob_returns_original_content() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let content = reader.read_and_verify_blob(&manifest.entries[0]).unwrap();
    assert_eq!(content, b"aaa");
}

#[test]
fn copy_and_verify_blob_streams_original_content() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let mut out = Vec::new();
    reader.copy_and_verify_blob(&manifest.entries[0], &mut out).unwrap();
    assert_eq!(out, b"aaa");
}

#[test]
fn restore_file_restores_only_selected_path() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    fs::write(source.join("nested").join("b.txt"), "bbb").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let target = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    let result = reader.restore_file("nested/b.txt", &target).unwrap();
    assert_eq!(result.restored_files, 1);
    assert!(!target.join("a.txt").exists());
    assert_eq!(
        fs::read_to_string(target.join("nested").join("b.txt")).unwrap(),
        "bbb"
    );
}

#[test]
fn restore_file_reports_missing_path() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let target = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    assert!(matches!(
        reader.restore_file("missing.txt", &target),
        Err(ReaderError::PathNotFound(_))
    ));
}

#[test]
fn restore_glob_restores_only_matching_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("a.rs"), "aaa").unwrap();
    fs::write(source.join("nested").join("b.rs"), "bbb").unwrap();
    fs::write(source.join("c.txt"), "ccc").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let target = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    let result = reader.restore_glob("nested/**/*.rs", &target).unwrap();
    assert_eq!(result.restored_files, 1);
    assert!(!target.join("a.rs").exists());
    assert_eq!(
        fs::read_to_string(target.join("nested").join("b.rs")).unwrap(),
        "bbb"
    );
    assert!(!target.join("c.txt").exists());
}

#[test]
fn restore_glob_returns_zero_when_nothing_matches() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let target = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    let result = reader.restore_glob("**/*.rs", &target).unwrap();
    assert_eq!(result.restored_files, 0);
}
