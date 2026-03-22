#[doc(hidden)]
pub mod backup {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::commands;
    use crate::windows::file_copy::{FileCopyBackend, copy_file};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CopyBackend {
        Std,
        CopyFile2,
    }

    pub fn read_baseline_len(prev: &Path) -> usize {
        commands::backup::bench_read_baseline_len(prev)
    }

    pub fn scan_and_diff_count(current_root: &Path, prev: &Path) -> usize {
        commands::backup::bench_scan_and_diff_count(current_root, prev)
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

    fn collect_tree_jobs_inner(src_root: &Path, current: &Path, dst_root: &Path, out: &mut Vec<TreeJob>) {
        let Ok(rd) = fs::read_dir(current) else { return };
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
