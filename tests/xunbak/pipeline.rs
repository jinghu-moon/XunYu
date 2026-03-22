use std::fs;
use std::io::Cursor;

use tempfile::tempdir;
use xun::xunbak::codec::stream_hash_and_compress;
use xun::xunbak::constants::Codec;
use xun::xunbak::reader::ContainerReader;
use xun::xunbak::writer::{BackupOptions, ContainerWriter, parallel_compress_pipeline};

#[test]
fn single_thread_baseline_backup_of_100_files_is_correct() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    for i in 0..100 {
        fs::write(source.join(format!("f{i:03}.txt")), format!("file-{i}")).unwrap();
    }
    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(
        &container,
        &source,
        &BackupOptions {
            codec: Codec::NONE,
            zstd_level: 1,
        },
    )
    .unwrap();
    let manifest = ContainerReader::open(&container).unwrap().load_manifest().unwrap();
    assert_eq!(manifest.entries.len(), 100);
}

#[test]
fn parallel_pipeline_matches_single_thread_logical_results() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    let mut files = Vec::new();
    for i in 0..100 {
        let path = source.join(format!("f{i:03}.txt"));
        fs::write(&path, format!("file-{i}")).unwrap();
        let meta = fs::metadata(&path).unwrap();
        files.push(xun::xunbak::writer::ScannedSourceFile {
            rel: format!("f{i:03}.txt"),
            path,
            size: meta.len(),
            mtime_ns: meta.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64,
            created_time_ns: meta.created().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64,
            win_attributes: 0,
        });
    }
    let seq = parallel_compress_pipeline(&files, Codec::NONE, 1, 1).unwrap();
    let par = parallel_compress_pipeline(&files, Codec::NONE, 1, 4).unwrap();
    assert_eq!(seq.len(), par.len());
    for (left, right) in seq.iter().zip(par.iter()) {
        assert_eq!(left.path, right.path);
        assert_eq!(left.header.blob_id, right.header.blob_id);
        assert_eq!(left.header.raw_size, right.header.raw_size);
    }
}

#[test]
fn parallel_pipeline_with_one_thread_matches_sequential_mode() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    let path = source.join("a.txt");
    fs::write(&path, "aaa").unwrap();
    let meta = fs::metadata(&path).unwrap();
    let files = vec![xun::xunbak::writer::ScannedSourceFile {
        rel: "a.txt".to_string(),
        path,
        size: meta.len(),
        mtime_ns: meta.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64,
        created_time_ns: meta.created().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64,
        win_attributes: 0,
    }];
    let seq = parallel_compress_pipeline(&files, Codec::NONE, 1, 1).unwrap();
    let one = parallel_compress_pipeline(&files, Codec::NONE, 1, 1).unwrap();
    assert_eq!(seq, one);
}

#[test]
fn streaming_hash_and_compress_limits_buffer_growth_for_10mb() {
    let input = vec![b'x'; 10 * 1024 * 1024];
    let result = stream_hash_and_compress(&mut Cursor::new(&input), Codec::ZSTD, 1, 64 * 1024).unwrap();
    assert!(result.peak_buffer_bytes <= 2 * 64 * 1024);
}

#[test]
fn streaming_hash_matches_one_shot_blake3() {
    let input = vec![b'y'; 10 * 1024 * 1024];
    let result = stream_hash_and_compress(&mut Cursor::new(&input), Codec::NONE, 1, 64 * 1024).unwrap();
    assert_eq!(result.hash, *blake3::hash(&input).as_bytes());
}
