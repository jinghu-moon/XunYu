use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use xun::xunbak::constants::{Codec, FOOTER_SIZE};
use xun::xunbak::reader::ContainerReader;
use xun::xunbak::verify::verify_quick_path;
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

fn hash_tree(root: &Path) -> Vec<(String, [u8; 32])> {
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, [u8; 32])>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            walk(root, &path, out);
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let content = fs::read(&path).unwrap();
            out.push((rel, *blake3::hash(&content).as_bytes()));
        }
    }
}

fn maybe_long_path(base: &Path) -> Option<PathBuf> {
    let mut current = base.to_path_buf();
    for i in 0..8 {
        current = current.join(format!("segment_{i:02}_abcdefghijklmnopqrstuvwxyz"));
    }
    Some(current.join("long_name_file.txt"))
}

#[test]
fn e2e_roundtrip_preserves_all_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    fs::write(source.join("nested").join("b.txt"), "bbb").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let restore_dir = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    reader.restore_all(&restore_dir).unwrap();

    assert_eq!(hash_tree(&source), hash_tree(&restore_dir));
}

#[test]
fn e2e_supports_unicode_spaces_and_long_paths() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    let unicode_dir = source.join("中文 目录");
    fs::create_dir_all(&unicode_dir).unwrap();
    fs::write(unicode_dir.join("空 格.txt"), "hello").unwrap();

    if let Some(long_path) = maybe_long_path(&source) {
        if let Some(parent) = long_path.parent() {
            if fs::create_dir_all(parent).is_ok() {
                let _ = fs::write(&long_path, "long path");
            }
        }
    }

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
    let restore_dir = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    reader.restore_all(&restore_dir).unwrap();

    assert_eq!(hash_tree(&source), hash_tree(&restore_dir));
}

#[test]
fn e2e_handles_empty_file_and_skips_compressing_zip_and_jpg() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("empty.txt"), "").unwrap();
    fs::write(source.join("archive.zip"), b"PK\x03\x04dummy zip").unwrap();
    fs::write(source.join("photo.jpg"), b"\xFF\xD8\xFFdummy jpg").unwrap();

    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();

    let zip_entry = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "archive.zip")
        .unwrap();
    let jpg_entry = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "photo.jpg")
        .unwrap();
    let empty_entry = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "empty.txt")
        .unwrap();
    assert_eq!(zip_entry.codec, Codec::NONE);
    assert_eq!(jpg_entry.codec, Codec::NONE);
    assert_eq!(empty_entry.size, 0);
}

#[test]
fn e2e_incremental_update_restore_keeps_expected_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    fs::write(source.join("b.txt"), "bbb").unwrap();
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

    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(source.join("b.txt"), "bbbb").unwrap();
    fs::write(source.join("c.txt"), "ccc").unwrap();
    fs::remove_file(source.join("a.txt")).unwrap();
    ContainerWriter::update(
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

    let restore_dir = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    reader.restore_all(&restore_dir).unwrap();
    assert!(!restore_dir.join("a.txt").exists());
    assert_eq!(
        fs::read_to_string(restore_dir.join("b.txt")).unwrap(),
        "bbbb"
    );
    assert_eq!(
        fs::read_to_string(restore_dir.join("c.txt")).unwrap(),
        "ccc"
    );
}

#[test]
fn e2e_truncated_container_recovers_previous_checkpoint() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "v1").unwrap();
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
    let original_size = fs::metadata(&container).unwrap().len();

    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(source.join("a.txt"), "v2").unwrap();
    fs::write(source.join("b.txt"), "new").unwrap();
    ContainerWriter::update(
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

    let mut bytes = fs::read(&container).unwrap();
    let truncate_to = original_size as usize - FOOTER_SIZE + 10;
    bytes.truncate(truncate_to);
    fs::write(&container, bytes).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    let restore_dir = dir.path().join("restore");
    reader.restore_all(&restore_dir).unwrap();
    assert_eq!(fs::read_to_string(restore_dir.join("a.txt")).unwrap(), "v1");
    assert!(!restore_dir.join("b.txt").exists());
}

#[test]
fn e2e_footer_truncated_still_uses_fallback() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let mut bytes = fs::read(&container).unwrap();
    bytes.truncate(bytes.len() - FOOTER_SIZE / 2);
    fs::write(&container, bytes).unwrap();

    let reader = ContainerReader::open(&container).unwrap();
    let restore_dir = dir.path().join("restore");
    reader.restore_all(&restore_dir).unwrap();
    assert_eq!(
        fs::read_to_string(restore_dir.join("a.txt")).unwrap(),
        "aaa"
    );
}

#[test]
fn e2e_handles_single_file_and_empty_directory_boundaries() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("single.txt"), "one").unwrap();
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let restore_dir = dir.path().join("restore");
    ContainerReader::open(&container)
        .unwrap()
        .restore_all(&restore_dir)
        .unwrap();
    assert_eq!(
        fs::read_to_string(restore_dir.join("single.txt")).unwrap(),
        "one"
    );

    let empty_container = dir.path().join("empty.xunbak");
    ContainerWriter::create(&empty_container).unwrap();
    let empty_restore = dir.path().join("empty_restore");
    ContainerReader::open(&empty_container)
        .unwrap()
        .restore_all(&empty_restore)
        .unwrap();
    assert!(empty_restore.exists());
}

#[test]
fn e2e_backup_empty_directory_verify_and_restore() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("empty_src");
    fs::create_dir_all(&source).unwrap();
    let container = dir.path().join("empty_backup.xunbak");

    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let report = verify_quick_path(&container);
    assert!(report.passed);

    let restore_dir = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    let result = reader.restore_all(&restore_dir).unwrap();
    assert_eq!(result.restored_files, 0);
    assert!(restore_dir.exists());
    assert!(fs::read_dir(&restore_dir).unwrap().next().is_none());
}

#[test]
fn e2e_scales_to_ten_thousand_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    for bucket in 0..100 {
        let folder = source.join(format!("d{bucket:03}"));
        fs::create_dir_all(&folder).unwrap();
        for file in 0..100 {
            fs::write(folder.join(format!("f{file:03}.txt")), b"x").unwrap();
        }
    }
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

    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    assert_eq!(manifest.entries.len(), 10_000);

    let restore_dir = dir.path().join("restore");
    let result = reader.restore_all(&restore_dir).unwrap();
    assert_eq!(result.restored_files, 10_000);
}
