use std::fs;
use std::io::Write;

use tempfile::tempdir;
use xun::xunbak::constants::Codec;
use xun::xunbak::reader::{ContainerReader, ReaderError};
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

fn repeated_text_bytes(size: usize) -> Vec<u8> {
    let chunk = b"alpha alpha alpha beta beta beta gamma gamma gamma delta delta delta\n";
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        let remaining = size - out.len();
        if remaining >= chunk.len() {
            out.extend_from_slice(chunk);
        } else {
            out.extend_from_slice(&chunk[..remaining]);
        }
    }
    out
}

struct HashingChunkLimitedWriter {
    max_write_len: usize,
    total_bytes: u64,
    chunks: usize,
    hasher: blake3::Hasher,
}

impl HashingChunkLimitedWriter {
    fn new(max_write_len: usize) -> Self {
        Self {
            max_write_len,
            total_bytes: 0,
            chunks: 0,
            hasher: blake3::Hasher::new(),
        }
    }

    fn finalize(self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }
}

impl Write for HashingChunkLimitedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() > self.max_write_len {
            return Err(std::io::Error::other(format!(
                "chunk too large: {} > {}",
                buf.len(),
                self.max_write_len
            )));
        }
        self.total_bytes += buf.len() as u64;
        self.chunks += 1;
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

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
    reader
        .copy_and_verify_blob(&manifest.entries[0], &mut out)
        .unwrap();
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

#[test]
fn copy_and_verify_blob_streams_large_extended_codec_content_in_chunks() {
    const LARGE_SIZE: usize = (32 * 1024 * 1024) + 1_048_576;
    const MAX_WRITE_LEN: usize = 256 * 1024;
    const MULTIPART_CHUNK_BYTES: u64 = 16 * 1024 * 1024;

    for codec in [Codec::LZ4, Codec::PPMD, Codec::LZMA2] {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        let content = repeated_text_bytes(LARGE_SIZE);
        fs::write(source.join("large.txt"), &content).unwrap();
        let container = dir
            .path()
            .join(format!("backup-{}.xunbak", u8::from(codec)));
        ContainerWriter::backup(
            &container,
            &source,
            &BackupOptions {
                codec,
                auto_compression: false,
                zstd_level: 1,
                split_size: None,
            },
        )
        .unwrap();

        let reader = ContainerReader::open(&container).unwrap();
        let manifest = reader.load_manifest().unwrap();
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.path == "large.txt")
            .unwrap();
        let parts = entry
            .parts
            .as_ref()
            .expect("large file should use multipart");
        assert!(parts.len() >= 2, "codec={:?}", u8::from(codec));
        assert!(parts.iter().all(|part| part.codec == codec));
        assert!(
            parts
                .iter()
                .all(|part| part.raw_size <= MULTIPART_CHUNK_BYTES)
        );

        let mut writer = HashingChunkLimitedWriter::new(MAX_WRITE_LEN);
        reader.copy_and_verify_blob(entry, &mut writer).unwrap();

        assert_eq!(writer.total_bytes, content.len() as u64);
        assert!(writer.chunks > 1, "codec={:?}", u8::from(codec));
        assert_eq!(writer.finalize(), *blake3::hash(&content).as_bytes());
    }
}
