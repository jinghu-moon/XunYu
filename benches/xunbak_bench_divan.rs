use std::fs;
use std::path::PathBuf;

use divan::{AllocProfiler, Bencher};
use tempfile::tempdir;
use xun::xunbak::blob::write_blob_record;
use xun::xunbak::codec::compress;
use xun::xunbak::constants::Codec;
use xun::xunbak::header::Header;
use xun::xunbak::reader::ContainerReader;
use xun::xunbak::verify::{verify_full_path, verify_quick_path};
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

fn bytes(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

fn text_corpus(len: usize) -> Vec<u8> {
    let chunk = b"alpha alpha alpha beta beta beta gamma gamma gamma delta delta delta ";
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        let remaining = len - out.len();
        if remaining >= chunk.len() {
            out.extend_from_slice(chunk);
        } else {
            out.extend_from_slice(&chunk[..remaining]);
        }
    }
    out
}

fn create_large_text_source(root: &PathBuf) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("large.txt"),
        text_corpus((32 * 1024 * 1024) + 1_048_576),
    )
    .unwrap();
}

fn create_source_files(root: &PathBuf, count: usize) {
    for i in 0..count {
        let dir = root.join(format!("d{:03}", i / 20));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("f{i:03}.txt")), bytes(128)).unwrap();
    }
}

#[divan::bench]
fn header_roundtrip(bencher: Bencher) {
    let header = Header {
        write_version: 1,
        min_reader_version: 1,
        flags: 0,
        created_at_unix: 1_700_000_000,
        split: None,
    };
    bencher.bench_local(|| {
        let bytes = header.to_bytes();
        let _ = Header::from_bytes(&bytes).unwrap();
    });
}

fn bench_blob_write(bencher: Bencher, size: usize) {
    let input = bytes(size);
    bencher.bench_local(|| {
        let mut out = Vec::new();
        let _ = write_blob_record(&mut out, &input, Codec::NONE, 1).unwrap();
    });
}

#[divan::bench]
fn blob_write_1kb(bencher: Bencher) {
    bench_blob_write(bencher, 1024);
}

#[divan::bench]
fn blob_write_1mb(bencher: Bencher) {
    bench_blob_write(bencher, 1024 * 1024);
}

#[divan::bench]
fn blob_write_10mb(bencher: Bencher) {
    bench_blob_write(bencher, 10 * 1024 * 1024);
}

#[divan::bench]
fn compress_zstd_1mb(bencher: Bencher) {
    let input = bytes(1024 * 1024);
    bencher.bench_local(|| {
        let _ = compress(Codec::ZSTD, &input, 1).unwrap();
    });
}

#[divan::bench]
fn compress_lz4_1mb(bencher: Bencher) {
    let input = bytes(1024 * 1024);
    bencher.bench_local(|| {
        let _ = compress(Codec::LZ4, &input, 1).unwrap();
    });
}

#[divan::bench]
fn compress_lzma2_1mb(bencher: Bencher) {
    let input = bytes(1024 * 1024);
    bencher.bench_local(|| {
        let _ = compress(Codec::LZMA2, &input, 1).unwrap();
    });
}

#[divan::bench]
fn compress_deflate_1mb(bencher: Bencher) {
    let input = bytes(1024 * 1024);
    bencher.bench_local(|| {
        let _ = compress(Codec::DEFLATE, &input, 1).unwrap();
    });
}

#[divan::bench]
fn compress_bzip2_1mb(bencher: Bencher) {
    let input = bytes(1024 * 1024);
    bencher.bench_local(|| {
        let _ = compress(Codec::BZIP2, &input, 1).unwrap();
    });
}

#[divan::bench]
fn compress_ppmd_text_corpus(bencher: Bencher) {
    let input = text_corpus(1024 * 1024);
    bencher.bench_local(|| {
        let _ = compress(Codec::PPMD, &input, 1).unwrap();
    });
}

#[divan::bench]
fn backup_100_files(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    fs::create_dir_all(&source).unwrap();
    create_source_files(&source, 100);
    let container = tmp.path().join("backup.xunbak");
    bencher.bench_local(|| {
        let _ = fs::remove_file(&container);
        let _ = ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    });
}

#[divan::bench]
fn backup_100_files_lz4(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    fs::create_dir_all(&source).unwrap();
    create_source_files(&source, 100);
    let container = tmp.path().join("backup-lz4.xunbak");
    let options = BackupOptions {
        codec: Codec::LZ4,
        auto_compression: false,
        zstd_level: 1,
        split_size: None,
    };
    bencher.bench_local(|| {
        let _ = fs::remove_file(&container);
        let _ = ContainerWriter::backup(&container, &source, &options).unwrap();
    });
}

#[divan::bench]
fn backup_large_text_lz4(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    create_large_text_source(&source);
    let container = tmp.path().join("backup-large-lz4.xunbak");
    let options = BackupOptions {
        codec: Codec::LZ4,
        auto_compression: false,
        zstd_level: 1,
        split_size: None,
    };
    bencher.bench_local(|| {
        let _ = fs::remove_file(&container);
        let _ = ContainerWriter::backup(&container, &source, &options).unwrap();
    });
}

#[divan::bench]
fn backup_large_text_ppmd(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    create_large_text_source(&source);
    let container = tmp.path().join("backup-large-ppmd.xunbak");
    let options = BackupOptions {
        codec: Codec::PPMD,
        auto_compression: false,
        zstd_level: 1,
        split_size: None,
    };
    bencher.bench_local(|| {
        let _ = fs::remove_file(&container);
        let _ = ContainerWriter::backup(&container, &source, &options).unwrap();
    });
}

#[divan::bench]
fn backup_large_text_lzma2(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    create_large_text_source(&source);
    let container = tmp.path().join("backup-large-lzma2.xunbak");
    let options = BackupOptions {
        codec: Codec::LZMA2,
        auto_compression: false,
        zstd_level: 1,
        split_size: None,
    };
    bencher.bench_local(|| {
        let _ = fs::remove_file(&container);
        let _ = ContainerWriter::backup(&container, &source, &options).unwrap();
    });
}

#[divan::bench]
fn backup_incremental_10pct(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    fs::create_dir_all(&source).unwrap();
    create_source_files(&source, 100);
    let container = tmp.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    for i in 0..10 {
        let dir = source.join(format!("d{:03}", i / 20));
        fs::write(dir.join(format!("f{i:03}.txt")), bytes(256)).unwrap();
    }

    bencher.bench_local(|| {
        let _ = ContainerReader::open(&container).unwrap();
        let _ = ContainerWriter::update(&container, &source, &BackupOptions::default()).unwrap();
    });
}

#[divan::bench]
fn restore_100_files(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    fs::create_dir_all(&source).unwrap();
    create_source_files(&source, 100);
    let container = tmp.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    bencher.bench_local(|| {
        let target = tmp.path().join("restore");
        let _ = fs::remove_dir_all(&target);
        let reader = ContainerReader::open(&container).unwrap();
        let _ = reader.restore_all(&target).unwrap();
    });
}

#[divan::bench]
fn restore_100_files_lz4(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    fs::create_dir_all(&source).unwrap();
    create_source_files(&source, 100);
    let container = tmp.path().join("backup-lz4.xunbak");
    let options = BackupOptions {
        codec: Codec::LZ4,
        auto_compression: false,
        zstd_level: 1,
        split_size: None,
    };
    ContainerWriter::backup(&container, &source, &options).unwrap();
    bencher.bench_local(|| {
        let target = tmp.path().join("restore-lz4");
        let _ = fs::remove_dir_all(&target);
        let reader = ContainerReader::open(&container).unwrap();
        let _ = reader.restore_all(&target).unwrap();
    });
}

#[divan::bench]
fn restore_large_text_lz4(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    create_large_text_source(&source);
    let container = tmp.path().join("backup-large-lz4.xunbak");
    let options = BackupOptions {
        codec: Codec::LZ4,
        auto_compression: false,
        zstd_level: 1,
        split_size: None,
    };
    ContainerWriter::backup(&container, &source, &options).unwrap();
    bencher.bench_local(|| {
        let target = tmp.path().join("restore-large-lz4");
        let _ = fs::remove_dir_all(&target);
        let reader = ContainerReader::open(&container).unwrap();
        let _ = reader.restore_all(&target).unwrap();
    });
}

#[divan::bench]
fn restore_large_text_ppmd(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    create_large_text_source(&source);
    let container = tmp.path().join("backup-large-ppmd.xunbak");
    let options = BackupOptions {
        codec: Codec::PPMD,
        auto_compression: false,
        zstd_level: 1,
        split_size: None,
    };
    ContainerWriter::backup(&container, &source, &options).unwrap();
    bencher.bench_local(|| {
        let target = tmp.path().join("restore-large-ppmd");
        let _ = fs::remove_dir_all(&target);
        let reader = ContainerReader::open(&container).unwrap();
        let _ = reader.restore_all(&target).unwrap();
    });
}

#[divan::bench]
fn restore_large_text_lzma2(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    create_large_text_source(&source);
    let container = tmp.path().join("backup-large-lzma2.xunbak");
    let options = BackupOptions {
        codec: Codec::LZMA2,
        auto_compression: false,
        zstd_level: 1,
        split_size: None,
    };
    ContainerWriter::backup(&container, &source, &options).unwrap();
    bencher.bench_local(|| {
        let target = tmp.path().join("restore-large-lzma2");
        let _ = fs::remove_dir_all(&target);
        let reader = ContainerReader::open(&container).unwrap();
        let _ = reader.restore_all(&target).unwrap();
    });
}

#[divan::bench]
fn verify_quick(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    fs::create_dir_all(&source).unwrap();
    create_source_files(&source, 100);
    let container = tmp.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    bencher.bench_local(|| {
        let _ = verify_quick_path(&container);
    });
}

#[divan::bench]
fn verify_full(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("src");
    fs::create_dir_all(&source).unwrap();
    create_source_files(&source, 100);
    let container = tmp.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    bencher.bench_local(|| {
        let _ = verify_full_path(&container);
    });
}
