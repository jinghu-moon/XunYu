#[doc(hidden)]
pub mod backup {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::backup::legacy;
    use crate::windows::file_copy::{FileCopyBackend, copy_file};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CopyBackend {
        Std,
        CopyFile2,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HashDiffBenchStats {
        pub diff_entries: usize,
        pub total_files: usize,
        pub hash_checked_files: u64,
        pub hash_cache_hits: u64,
        pub hash_computed_files: u64,
        pub hash_failed_files: u64,
    }

    pub fn read_baseline_len(prev: &Path) -> usize {
        legacy::bench_read_baseline_len(prev)
    }

    pub fn scan_and_metadata_diff_count(
        current_root: &Path,
        prev: &Path,
        includes: &[String],
    ) -> usize {
        legacy::bench_scan_and_metadata_diff_count(current_root, prev, includes)
    }

    pub fn scan_and_hash_diff_stats(
        current_root: &Path,
        prev: &Path,
        includes: &[String],
    ) -> HashDiffBenchStats {
        let stats = legacy::bench_scan_and_hash_diff(current_root, prev, includes);
        HashDiffBenchStats {
            diff_entries: stats.diff_entries,
            total_files: stats.total_files,
            hash_checked_files: stats.hash_checked_files,
            hash_cache_hits: stats.hash_cache_hits,
            hash_computed_files: stats.hash_computed_files,
            hash_failed_files: stats.hash_failed_files,
        }
    }

    pub fn copy_tree_with_backend(src_root: &Path, dst_root: &Path, backend: CopyBackend) -> u64 {
        let backend = match backend {
            CopyBackend::Std => FileCopyBackend::Std,
            CopyBackend::CopyFile2 => FileCopyBackend::CopyFile2,
        };
        let jobs = collect_tree_jobs(src_root, dst_root);
        precreate_parent_dirs(&jobs);
        let mut bytes = 0u64;
        for job in jobs {
            bytes += copy_file(&job.src, &job.dst, backend).unwrap_or(0);
        }
        bytes
    }

    pub fn hardlink_tree(src_root: &Path, dst_root: &Path) -> usize {
        let jobs = collect_tree_jobs(src_root, dst_root);
        precreate_parent_dirs(&jobs);
        let mut linked = 0usize;
        for job in jobs {
            if try_hard_link(&job.src, &job.dst) {
                linked += 1;
            }
        }
        linked
    }

    struct TreeJob {
        src: PathBuf,
        dst: PathBuf,
    }

    fn precreate_parent_dirs(jobs: &[TreeJob]) {
        let mut dirs = std::collections::HashSet::new();
        for job in jobs {
            if let Some(parent) = job.dst.parent() {
                dirs.insert(parent.to_path_buf());
            }
        }
        for dir in dirs {
            let _ = fs::create_dir_all(dir);
        }
    }

    fn collect_tree_jobs(src_root: &Path, dst_root: &Path) -> Vec<TreeJob> {
        let mut jobs = Vec::new();
        collect_tree_jobs_inner(src_root, src_root, dst_root, &mut jobs);
        jobs
    }

    fn collect_tree_jobs_inner(
        src_root: &Path,
        current: &Path,
        dst_root: &Path,
        out: &mut Vec<TreeJob>,
    ) {
        let Ok(rd) = fs::read_dir(current) else {
            return;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_tree_jobs_inner(src_root, &path, dst_root, out);
            } else if path.is_file() {
                let rel = path.strip_prefix(src_root).unwrap_or(&path).to_path_buf();
                out.push(TreeJob {
                    src: path,
                    dst: dst_root.join(rel),
                });
            }
        }
    }

    #[cfg(windows)]
    fn try_hard_link(src: &Path, dst: &Path) -> bool {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::CreateHardLinkW;

        let mut src_w: Vec<u16> = src.as_os_str().encode_wide().collect();
        src_w.push(0);
        let mut dst_w: Vec<u16> = dst.as_os_str().encode_wide().collect();
        dst_w.push(0);
        unsafe { CreateHardLinkW(dst_w.as_ptr(), src_w.as_ptr(), std::ptr::null()) != 0 }
    }

    #[cfg(not(windows))]
    fn try_hard_link(_src: &Path, _dst: &Path) -> bool {
        false
    }
}

#[cfg(feature = "xunbak")]
#[doc(hidden)]
pub mod backup_perf {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::backup::artifact::entry::{SourceEntry, SourceKind};
    use crate::backup::artifact::sidecar::{
        SidecarPackingHint, SidecarSourceInfo, build_sidecar_bytes,
    };
    use crate::backup::artifact::verify::verify_entries_content;
    use crate::backup::common::hash::compute_file_content_hash;
    use crate::backup_formats::BackupArtifactFormat;
    use crate::xunbak::constants::Codec;
    use crate::xunbak::reader::ContainerReader;
    use crate::xunbak::verify::verify_full_path;
    use crate::xunbak::writer::{BackupOptions, ContainerWriter};

    pub struct SidecarBenchFixture {
        _root: PathBuf,
        source: SidecarSourceInfo,
        entries: Vec<SourceEntry>,
    }

    impl SidecarBenchFixture {
        pub fn build_sidecar_bytes(&self) -> usize {
            let refs = self.entries.iter().collect::<Vec<_>>();
            build_sidecar_bytes(
                BackupArtifactFormat::Zip,
                SidecarPackingHint::Zip(crate::backup::artifact::zip::ZipCompressionMethod::Stored),
                &self.source,
                &refs,
            )
            .unwrap()
            .len()
        }
    }

    pub fn prepare_sidecar_fixture(
        root: &Path,
        file_count: usize,
        file_size: usize,
        prehash: bool,
    ) -> SidecarBenchFixture {
        let source_root = root.join("sidecar-src");
        let _ = fs::remove_dir_all(&source_root);
        fs::create_dir_all(&source_root).unwrap();
        let mut entries = Vec::with_capacity(file_count);
        for i in 0..file_count {
            let dir = source_root.join(format!("d{:03}", i / 100));
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join(format!("f{i:04}.txt"));
            let content = vec![b'a' + (i % 23) as u8; file_size];
            fs::write(&path, &content).unwrap();
            entries.push(SourceEntry {
                path: path
                    .strip_prefix(&source_root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                source_path: Some(path.clone()),
                size: content.len() as u64,
                mtime_ns: None,
                created_time_ns: None,
                win_attributes: 0,
                content_hash: prehash.then(|| *blake3::hash(&content).as_bytes()),
                kind: SourceKind::Filesystem,
            });
        }
        SidecarBenchFixture {
            _root: source_root.clone(),
            source: SidecarSourceInfo {
                snapshot_id: "bench-sidecar".to_string(),
                source_root: source_root.display().to_string(),
            },
            entries,
        }
    }

    pub struct HashBenchFixture {
        _root: PathBuf,
        path: PathBuf,
    }

    impl HashBenchFixture {
        pub fn compute_hash(&self) -> [u8; 32] {
            compute_file_content_hash(&self.path).unwrap()
        }
    }

    pub fn prepare_hash_fixture(root: &Path, file_size: usize) -> HashBenchFixture {
        let source_root = root.join("hash-src");
        let _ = fs::remove_dir_all(&source_root);
        fs::create_dir_all(&source_root).unwrap();
        let path = source_root.join("large.bin");
        fs::write(&path, vec![0x5Au8; file_size]).unwrap();
        HashBenchFixture {
            _root: source_root,
            path,
        }
    }

    pub struct VerifyBenchFixture {
        _root: PathBuf,
        dir_path: PathBuf,
        xunbak_path: PathBuf,
    }

    impl VerifyBenchFixture {
        pub fn verify_dir_entries_content(&self) {
            verify_entries_content(&self.dir_path).unwrap();
        }

        pub fn verify_xunbak_entries_content(&self) {
            verify_entries_content(&self.xunbak_path).unwrap();
        }

        pub fn verify_xunbak_full(&self) {
            let report = verify_full_path(&self.xunbak_path);
            assert!(report.passed);
        }
    }

    pub fn prepare_verify_fixture(
        root: &Path,
        file_count: usize,
        file_size: usize,
    ) -> VerifyBenchFixture {
        let dir_path = root.join("verify-src");
        let _ = fs::remove_dir_all(&dir_path);
        fs::create_dir_all(&dir_path).unwrap();
        for i in 0..file_count {
            let dir = dir_path.join(format!("d{:03}", i / 100));
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join(format!("f{i:04}.txt"));
            let content = vec![b'a' + (i % 19) as u8; file_size];
            fs::write(path, content).unwrap();
        }

        let xunbak_path = root.join("verify-bench.xunbak");
        let options = BackupOptions {
            codec: Codec::NONE,
            auto_compression: false,
            zstd_level: 1,
            split_size: None,
        };
        let _ = fs::remove_file(&xunbak_path);
        ContainerWriter::backup(&xunbak_path, &dir_path, &options).unwrap();

        VerifyBenchFixture {
            _root: root.to_path_buf(),
            dir_path,
            xunbak_path,
        }
    }

    pub struct RestoreBenchFixture {
        _root: PathBuf,
        xunbak_path: PathBuf,
    }

    impl RestoreBenchFixture {
        pub fn restore_all(&self, target: &Path) {
            let reader = ContainerReader::open(&self.xunbak_path).unwrap();
            let _ = fs::remove_dir_all(target);
            reader.restore_all(target).unwrap();
        }

        pub fn restore_all_incremental(&self, target: &Path) {
            let reader = ContainerReader::open(&self.xunbak_path).unwrap();
            reader.restore_all(target).unwrap();
        }
    }

    pub fn prepare_restore_fixture(
        root: &Path,
        file_count: usize,
        file_size: usize,
    ) -> RestoreBenchFixture {
        let dir_path = root.join("restore-src");
        let _ = fs::remove_dir_all(&dir_path);
        fs::create_dir_all(&dir_path).unwrap();
        for i in 0..file_count {
            let dir = dir_path.join(format!("d{:03}", i / 100));
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join(format!("f{i:04}.txt"));
            let content = vec![b'a' + (i % 17) as u8; file_size];
            fs::write(path, content).unwrap();
        }

        let xunbak_path = root.join("restore-bench.xunbak");
        let options = BackupOptions {
            codec: Codec::NONE,
            auto_compression: false,
            zstd_level: 1,
            split_size: None,
        };
        let _ = fs::remove_file(&xunbak_path);
        ContainerWriter::backup(&xunbak_path, &dir_path, &options).unwrap();

        RestoreBenchFixture {
            _root: root.to_path_buf(),
            xunbak_path,
        }
    }
}
