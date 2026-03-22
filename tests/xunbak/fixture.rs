use std::collections::BTreeMap;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::tempdir;
use xun::xunbak::constants::Codec;
use xun::xunbak::reader::ContainerReader;
use xun::xunbak::verify::{verify_paranoid_path, verify_quick_path};
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("xunbak_sample")
}

fn set_windows_attributes(path: &Path, attrs: u32) {
    use windows_sys::Win32::Storage::FileSystem::SetFileAttributesW;

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let ok = unsafe { SetFileAttributesW(wide.as_ptr(), attrs) };
    assert_ne!(ok, 0, "failed to set attributes for {}", path.display());
}

fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&src_path, &dst_path);
            continue;
        }

        fs::copy(&src_path, &dst_path).unwrap();
        let attrs = fs::metadata(&src_path).unwrap().file_attributes();
        set_windows_attributes(&dst_path, attrs);
    }
}

fn hash_tree(root: &Path) -> BTreeMap<String, [u8; 32]> {
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, [u8; 32]>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            walk(root, &path, out);
            continue;
        }

        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read(&path).unwrap();
        out.insert(rel, *blake3::hash(&content).as_bytes());
    }
}

fn manifest_entry<'a>(
    manifest: &'a xun::xunbak::manifest::ManifestBody,
    path: &str,
) -> &'a xun::xunbak::manifest::ManifestEntry {
    manifest
        .entries
        .iter()
        .find(|entry| entry.path == path)
        .unwrap_or_else(|| panic!("missing manifest entry: {path}"))
}

fn assert_attribute_flag(source: &Path, restored: &Path, rel: &str, flag: u32) {
    let source_attrs = fs::metadata(source.join(rel)).unwrap().file_attributes();
    let restored_attrs = fs::metadata(restored.join(rel)).unwrap().file_attributes();
    assert_eq!(
        source_attrs & flag,
        flag,
        "source missing attr flag for {rel}"
    );
    assert_eq!(
        restored_attrs & flag,
        flag,
        "restored missing attr flag for {rel}"
    );
}

#[test]
fn fixture_roundtrip_preserves_tree_metadata_and_dedup() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("fixture");
    copy_tree(&fixture_root(), &source);

    let container = dir.path().join("sample.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();

    let quick = verify_quick_path(&container);
    assert!(quick.passed, "quick verify failed: {:?}", quick.errors);

    let reader = ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    assert_eq!(manifest.file_count as usize, hash_tree(&source).len());

    let duplicate_a = manifest_entry(&manifest, "src/duplicate_a.txt");
    let duplicate_b = manifest_entry(&manifest, "docs/duplicate_b.txt");
    assert_eq!(duplicate_a.content_hash, duplicate_b.content_hash);
    assert_eq!(duplicate_a.blob_offset, duplicate_b.blob_offset);
    assert_eq!(duplicate_a.blob_len, duplicate_b.blob_len);
    assert_eq!(duplicate_a.volume_index, duplicate_b.volume_index);

    assert_eq!(
        manifest_entry(&manifest, "docs/sample.log").codec,
        Codec::ZSTD
    );
    assert_eq!(
        manifest_entry(&manifest, "assets/images/photo.jpg").codec,
        Codec::NONE
    );
    assert_eq!(
        manifest_entry(&manifest, "assets/images/icon.png").codec,
        Codec::NONE
    );
    assert_eq!(
        manifest_entry(&manifest, "assets/archives/data.zip").codec,
        Codec::NONE
    );
    assert_eq!(
        manifest_entry(&manifest, "assets/archives/backup.7z").codec,
        Codec::NONE
    );
    assert_eq!(
        manifest_entry(&manifest, "assets/random.bin").codec,
        Codec::NONE
    );

    let restore_dir = dir.path().join("restore");
    let restored = reader.restore_all(&restore_dir).unwrap();
    assert_eq!(restored.restored_files, manifest.file_count as usize);
    assert_eq!(hash_tree(&source), hash_tree(&restore_dir));
    assert_attribute_flag(&source, &restore_dir, "config/readonly_file.txt", 0x01);
    assert_attribute_flag(&source, &restore_dir, "config/hidden_file.txt", 0x02);

    let paranoid = verify_paranoid_path(&container);
    assert!(
        paranoid.passed,
        "paranoid verify failed: {:?}",
        paranoid.errors
    );
}

#[test]
fn fixture_split_incremental_update_restores_latest_state() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("fixture");
    copy_tree(&fixture_root(), &source);

    let base = dir.path().join("sample.xunbak");
    let options = BackupOptions {
        codec: Codec::ZSTD,
        zstd_level: 1,
        split_size: Some(72 * 1024),
    };

    ContainerWriter::backup(&base, &source, &options).unwrap();
    assert!(dir.path().join("sample.xunbak.001").exists());
    assert!(dir.path().join("sample.xunbak.002").exists());

    let manifest_before = ContainerReader::open(&base)
        .unwrap()
        .load_manifest()
        .unwrap();
    let random_before = manifest_entry(&manifest_before, "assets/random.bin").clone();

    std::thread::sleep(Duration::from_millis(20));
    fs::write(
        source.join("config/settings.json"),
        "{\n  \"mode\": \"fixture-update\",\n  \"enabled\": true,\n  \"workers\": 8\n}\n",
    )
    .unwrap();
    let mut log = fs::read_to_string(source.join("docs/sample.log")).unwrap();
    log.push_str("\nfixture incremental append line 1\nfixture incremental append line 2\n");
    fs::write(source.join("docs/sample.log"), log).unwrap();
    fs::remove_file(source.join("src/core/mod.rs")).unwrap();
    let added = source
        .join("new nested")
        .join("更深目录")
        .join("增量 文件.txt");
    fs::create_dir_all(added.parent().unwrap()).unwrap();
    fs::write(
        &added,
        "fixture incremental file for split backup/restore validation\n",
    )
    .unwrap();

    let update = ContainerWriter::update(&base, &source, &options).unwrap();
    assert_eq!(update.added_blob_count, 3);

    let reader = ContainerReader::open(&base).unwrap();
    assert!(reader.is_split);
    let manifest_after = reader.load_manifest().unwrap();
    let random_after = manifest_entry(&manifest_after, "assets/random.bin");
    assert_eq!(random_after.blob_offset, random_before.blob_offset);
    assert_eq!(random_after.blob_len, random_before.blob_len);
    assert_eq!(random_after.volume_index, random_before.volume_index);
    assert!(
        !manifest_after
            .entries
            .iter()
            .any(|entry| entry.path == "src/core/mod.rs")
    );
    assert!(
        manifest_after
            .entries
            .iter()
            .any(|entry| entry.path == "new nested/更深目录/增量 文件.txt")
    );

    let quick = verify_quick_path(&base);
    assert!(
        quick.passed,
        "quick verify failed after split update: {:?}",
        quick.errors
    );

    let restore_dir = dir.path().join("restore");
    let restored = reader.restore_all(&restore_dir).unwrap();
    assert_eq!(restored.restored_files, manifest_after.file_count as usize);
    assert_eq!(hash_tree(&source), hash_tree(&restore_dir));
    assert!(!restore_dir.join("src/core/mod.rs").exists());
    assert_attribute_flag(&source, &restore_dir, "config/readonly_file.txt", 0x01);
    assert_attribute_flag(&source, &restore_dir, "config/hidden_file.txt", 0x02);

    let paranoid = verify_paranoid_path(&base);
    assert!(
        paranoid.passed,
        "paranoid verify failed after split update: {:?}",
        paranoid.errors
    );
}
