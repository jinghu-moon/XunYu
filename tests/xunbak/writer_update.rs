use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use tempfile::tempdir;
use xun::xunbak::constants::{Codec, FOOTER_SIZE, HEADER_SIZE};
use xun::xunbak::footer::Footer;
use xun::xunbak::manifest::{ManifestBody, ManifestEntry};
use xun::xunbak::reader::ContainerReader;
use xun::xunbak::record::scan_records;
use xun::xunbak::writer::{
    BackupOptions, ContainerWriter, DiffKind, ScannedSourceFile, build_content_hash_index,
    diff_against_manifest,
};

fn large_test_bytes(size: usize) -> Vec<u8> {
    let mut value = 0x1234_5678u32;
    let mut out = Vec::with_capacity(size);
    for _ in 0..size {
        value = value.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        out.push((value >> 24) as u8);
    }
    out
}

fn none_options() -> BackupOptions {
    BackupOptions {
        codec: Codec::NONE,
        auto_compression: false,
        zstd_level: 1,
        split_size: None,
    }
}

fn read_manifest(container: &std::path::Path) -> ManifestBody {
    let reader = ContainerReader::open(container).unwrap();
    reader.load_manifest().unwrap()
}

fn manifest_by_path(manifest: &ManifestBody) -> HashMap<String, ManifestEntry> {
    manifest
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect()
}

#[test]
fn content_hash_index_maps_entries_to_blob_locators() {
    let manifest = ManifestBody {
        snapshot_id: "01JTESTSNAPSHOTID0000000000".to_string(),
        base_snapshot_id: None,
        created_at: 0,
        source_root: ".".to_string(),
        snapshot_context: serde_json::json!({}),
        file_count: 1,
        total_raw_bytes: 3,
        entries: vec![ManifestEntry {
            path: "a.txt".to_string(),
            blob_id: [1; 32],
            content_hash: [2; 32],
            size: 3,
            mtime_ns: 10,
            created_time_ns: 20,
            win_attributes: 0,
            codec: Codec::NONE,
            blob_offset: 64,
            blob_len: 66,
            volume_index: 0,
            parts: None,
            ext: None,
        }],
        removed: vec![],
    };
    let index = build_content_hash_index(&manifest);
    let locator = index.get(&[2; 32]).unwrap();
    assert_eq!(locator.blob_id, [1; 32]);
    assert_eq!(locator.blob_offset, 64);
    assert_eq!(locator.blob_len, 66);
}

#[test]
fn diff_against_manifest_marks_new_modified_unchanged_deleted() {
    let manifest = ManifestBody {
        snapshot_id: "01JTESTSNAPSHOTID0000000000".to_string(),
        base_snapshot_id: None,
        created_at: 0,
        source_root: ".".to_string(),
        snapshot_context: serde_json::json!({}),
        file_count: 3,
        total_raw_bytes: 9,
        entries: vec![
            ManifestEntry {
                path: "a.txt".to_string(),
                blob_id: [1; 32],
                content_hash: [1; 32],
                size: 3,
                mtime_ns: 10,
                created_time_ns: 20,
                win_attributes: 0,
                codec: Codec::NONE,
                blob_offset: 64,
                blob_len: 66,
                volume_index: 0,
                parts: None,
                ext: None,
            },
            ManifestEntry {
                path: "b.txt".to_string(),
                blob_id: [2; 32],
                content_hash: [2; 32],
                size: 3,
                mtime_ns: 10,
                created_time_ns: 20,
                win_attributes: 0,
                codec: Codec::NONE,
                blob_offset: 130,
                blob_len: 66,
                volume_index: 0,
                parts: None,
                ext: None,
            },
            ManifestEntry {
                path: "c.txt".to_string(),
                blob_id: [3; 32],
                content_hash: [3; 32],
                size: 3,
                mtime_ns: 10,
                created_time_ns: 20,
                win_attributes: 0,
                codec: Codec::NONE,
                blob_offset: 196,
                blob_len: 66,
                volume_index: 0,
                parts: None,
                ext: None,
            },
        ],
        removed: vec![],
    };
    let scan = vec![
        ScannedSourceFile {
            rel: "a.txt".to_string(),
            path: "a.txt".into(),
            size: 3,
            mtime_ns: 10,
            created_time_ns: 20,
            win_attributes: 0,
        },
        ScannedSourceFile {
            rel: "b.txt".to_string(),
            path: "b.txt".into(),
            size: 4,
            mtime_ns: 11,
            created_time_ns: 20,
            win_attributes: 0,
        },
        ScannedSourceFile {
            rel: "d.txt".to_string(),
            path: "d.txt".into(),
            size: 3,
            mtime_ns: 10,
            created_time_ns: 20,
            win_attributes: 0,
        },
    ];
    let diff = diff_against_manifest(&scan, &manifest);
    assert_eq!(
        diff,
        vec![
            xun::xunbak::writer::DiffEntry {
                path: "a.txt".to_string(),
                kind: DiffKind::Unchanged
            },
            xun::xunbak::writer::DiffEntry {
                path: "b.txt".to_string(),
                kind: DiffKind::Modified
            },
            xun::xunbak::writer::DiffEntry {
                path: "c.txt".to_string(),
                kind: DiffKind::Deleted
            },
            xun::xunbak::writer::DiffEntry {
                path: "d.txt".to_string(),
                kind: DiffKind::New
            },
        ]
    );
}

#[test]
fn update_adds_only_new_and_modified_blobs_and_reuses_unchanged_offsets() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    fs::write(source.join("b.txt"), "bbb").unwrap();
    fs::write(source.join("c.txt"), "ccc").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&container, &source, &none_options()).unwrap();
    let manifest_before = read_manifest(&container);
    let before_map = manifest_by_path(&manifest_before);

    std::thread::sleep(Duration::from_millis(20));
    fs::write(source.join("b.txt"), "bbbb").unwrap();
    fs::write(source.join("d.txt"), "ddd").unwrap();

    let update = ContainerWriter::update(&container, &source, &none_options()).unwrap();
    assert_eq!(update.added_blob_count, 2);

    let manifest_after = read_manifest(&container);
    let after_map = manifest_by_path(&manifest_after);
    assert_eq!(
        after_map["a.txt"].blob_offset,
        before_map["a.txt"].blob_offset
    );
    assert_eq!(after_map["a.txt"].blob_len, before_map["a.txt"].blob_len);

    let bytes = fs::read(&container).unwrap();
    let mut cursor = std::io::Cursor::new(&bytes[HEADER_SIZE..bytes.len() - FOOTER_SIZE]);
    let scanned = scan_records(&mut cursor).unwrap();
    assert_eq!(scanned.len(), 9);
}

#[test]
fn rename_without_content_change_reuses_blob_and_writes_no_new_blob() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("old.txt"), "same").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&container, &source, &none_options()).unwrap();
    let manifest_before = read_manifest(&container);
    let old_entry = manifest_before.entries.first().unwrap().clone();

    std::thread::sleep(Duration::from_millis(20));
    fs::rename(source.join("old.txt"), source.join("new.txt")).unwrap();

    let update = ContainerWriter::update(&container, &source, &none_options()).unwrap();
    assert_eq!(update.added_blob_count, 0);

    let manifest_after = read_manifest(&container);
    assert_eq!(manifest_after.entries.len(), 1);
    let new_entry = &manifest_after.entries[0];
    assert_eq!(new_entry.path, "new.txt");
    assert_eq!(new_entry.blob_id, old_entry.blob_id);
    assert_eq!(new_entry.blob_offset, old_entry.blob_offset);
}

#[test]
fn update_overwrites_old_footer_and_appends_new_snapshot() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&container, &source, &none_options()).unwrap();
    let original_size = fs::metadata(&container).unwrap().len();
    let original_footer = Footer::from_bytes(
        &fs::read(&container).unwrap()[original_size as usize - FOOTER_SIZE..],
        original_size,
    )
    .unwrap();

    std::thread::sleep(Duration::from_millis(20));
    fs::write(source.join("b.txt"), "bbb").unwrap();
    ContainerWriter::update(&container, &source, &none_options()).unwrap();

    let updated_bytes = fs::read(&container).unwrap();
    let updated_footer = Footer::from_bytes(
        &updated_bytes[updated_bytes.len() - FOOTER_SIZE..],
        updated_bytes.len() as u64,
    )
    .unwrap();
    assert!(updated_footer.checkpoint_offset > original_footer.checkpoint_offset);
    assert_ne!(
        &updated_bytes[original_size as usize - FOOTER_SIZE..original_size as usize],
        &updated_bytes[updated_bytes.len() - FOOTER_SIZE..]
    );
}

#[test]
fn phase_one_flush_crash_recovers_previous_checkpoint() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "v1").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&container, &source, &none_options()).unwrap();
    let before = ContainerReader::open(&container).unwrap();
    let previous_checkpoint_offset = before.footer.checkpoint_offset;
    let previous_snapshot_id = before.checkpoint.snapshot_id;

    std::thread::sleep(Duration::from_millis(20));
    fs::write(source.join("a.txt"), "v2").unwrap();
    fs::write(source.join("b.txt"), "new").unwrap();
    ContainerWriter::update(&container, &source, &none_options()).unwrap();

    let after = ContainerReader::open(&container).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&container)
        .unwrap()
        .set_len(after.footer.checkpoint_offset)
        .unwrap();

    let recovered = ContainerReader::open(&container).unwrap();
    assert_eq!(
        recovered.footer.checkpoint_offset,
        previous_checkpoint_offset
    );
    assert_eq!(recovered.checkpoint.snapshot_id, previous_snapshot_id);

    let restore_dir = dir.path().join("restore_after_phase_one");
    let result = recovered.restore_all(&restore_dir).unwrap();
    assert_eq!(result.restored_files, 1);
    assert_eq!(fs::read_to_string(restore_dir.join("a.txt")).unwrap(), "v1");
    assert!(!restore_dir.join("b.txt").exists());
}

#[test]
fn rename_large_multipart_file_reuses_parts_and_writes_no_new_blob() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    let large = large_test_bytes((32 * 1024 * 1024) + 1_048_576);
    fs::write(source.join("old.bin"), &large).unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let manifest_before = read_manifest(&container);
    let old_entry = manifest_before.entries.first().unwrap().clone();
    let old_parts = old_entry.parts.clone().expect("multipart expected");

    std::thread::sleep(Duration::from_millis(20));
    fs::rename(source.join("old.bin"), source.join("new.bin")).unwrap();

    let update = ContainerWriter::update(&container, &source, &BackupOptions::default()).unwrap();
    assert_eq!(update.added_blob_count, 0);

    let manifest_after = read_manifest(&container);
    let new_entry = manifest_after
        .entries
        .iter()
        .find(|entry| entry.path == "new.bin")
        .unwrap();
    assert_eq!(new_entry.content_hash, old_entry.content_hash);
    assert_eq!(new_entry.parts.as_ref().unwrap(), &old_parts);
}
