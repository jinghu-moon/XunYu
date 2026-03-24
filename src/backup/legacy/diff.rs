use std::collections::HashMap;
use std::fs;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use console::Style;
use rayon::prelude::*;

use super::baseline::FileMeta;
use super::scan::ScannedFile;
use super::util::fmt_size;
use crate::windows::file_copy::{FileCopyBackend, copy_file};

pub(crate) struct DiffStats {
    pub(crate) new: u32,
    pub(crate) modified: u32,
    pub(crate) reused: u32,
    pub(crate) deleted: u32,
    pub(crate) logical_bytes: u64,
    pub(crate) copied_bytes: u64,
    pub(crate) hardlinked_files: u32,
}

struct CopyJob {
    src: PathBuf,
    dst: PathBuf,
    link_wide: Option<WideLinkPaths>,
}

struct WideLinkPaths {
    src: Vec<u16>,
    dst: Vec<u16>,
}

/// diff 条目（纯数据）
pub(crate) struct DiffEntry {
    pub(crate) rel: String,
    /// src_path 仅 new/modified 有效；deleted 为 None
    pub(crate) src_path: Option<PathBuf>,
    pub(crate) kind: DiffKind,
    pub(crate) size_delta: i64,
    pub(crate) file_size: u64,
    pub(crate) reuse_from_rel: Option<String>,
}

#[derive(PartialEq, Eq)]
pub(crate) enum DiffKind {
    New,
    Modified,
    Reused,
    Unchanged,
    Deleted,
}

/// 纯计算：对比 current 文件集与 old 快照，返回所有 diff 条目
pub(crate) fn compute_diff(
    current: &HashMap<String, ScannedFile>,
    old: &mut HashMap<String, FileMeta>,
    skip_unchanged: bool,
) -> Vec<DiffEntry> {
    let mut entries: Vec<DiffEntry> = Vec::with_capacity(current.len() + old.len());

    for (rel, scanned) in current {
        if let Some(old_meta) = old.remove(rel) {
            if let (Some(current_hash), Some(old_hash)) = (scanned.content_hash, old_meta.content_hash)
            {
                if current_hash == old_hash {
                    if !skip_unchanged {
                        entries.push(DiffEntry {
                            rel: rel.clone(),
                            src_path: Some(scanned.path.clone()),
                            kind: DiffKind::Unchanged,
                            size_delta: 0,
                            file_size: scanned.size,
                            reuse_from_rel: None,
                        });
                    }
                } else {
                    let delta = scanned.size as i64 - old_meta.size as i64;
                    entries.push(DiffEntry {
                        rel: rel.clone(),
                        src_path: Some(scanned.path.clone()),
                        kind: DiffKind::Modified,
                        size_delta: delta,
                        file_size: scanned.size,
                        reuse_from_rel: None,
                    });
                }
                continue;
            }

            let size_changed = scanned.size != old_meta.size;
            // 只要 mtime 不同就视为变更。备份场景宁可多备份，不能漏备份。
            let time_changed = scanned.modified_ns != old_meta.modified_ns;
            if size_changed || time_changed {
                let delta = scanned.size as i64 - old_meta.size as i64;
                entries.push(DiffEntry {
                    rel: rel.clone(),
                    src_path: Some(scanned.path.clone()),
                    kind: DiffKind::Modified,
                    size_delta: delta,
                    file_size: scanned.size,
                    reuse_from_rel: None,
                });
            } else if !skip_unchanged {
                entries.push(DiffEntry {
                    rel: rel.clone(),
                    src_path: Some(scanned.path.clone()),
                    kind: DiffKind::Unchanged,
                    size_delta: 0,
                    file_size: scanned.size,
                    reuse_from_rel: None,
                });
            }
        } else {
            entries.push(DiffEntry {
                rel: rel.clone(),
                src_path: Some(scanned.path.clone()),
                kind: DiffKind::New,
                size_delta: scanned.size as i64,
                file_size: scanned.size,
                reuse_from_rel: None,
            });
        }
    }

    // 剩余 old 条目为已删除
    for rel in old.keys() {
        entries.push(DiffEntry {
            rel: rel.clone(),
            src_path: None,
            kind: DiffKind::Deleted,
            size_delta: 0,
            file_size: 0,
            reuse_from_rel: None,
        });
    }

    entries
}

/// 纯输出：将 diff 条目打印到 stderr
pub(crate) fn print_diff(entries: &[DiffEntry], show_unchanged: bool) {
    let green = Style::new().green();
    let yellow = Style::new().yellow();
    let blue = Style::new().blue();
    let cyan = Style::new().cyan();
    let dim = Style::new().dim();
    let red = Style::new().red();

    let max_len = entries
        .iter()
        .filter(|e| show_unchanged || e.kind != DiffKind::Unchanged)
        .map(|e| e.rel.len())
        .max()
        .unwrap_or(0);

    for e in entries {
        match e.kind {
            DiffKind::Unchanged if !show_unchanged => continue,
            DiffKind::New => {
                eprint!("{} ", green.apply_to("+"));
                print_colored_path(&e.rel);
                eprintln!();
            }
            DiffKind::Modified => {
                let (sym, extra) = if e.size_delta > 0 {
                    ("\u{2191}", fmt_size(e.size_delta)) // ↑
                } else if e.size_delta < 0 {
                    ("\u{2193}", fmt_size(e.size_delta)) // ↓
                } else {
                    ("\u{2022}", String::new()) // •
                };
                let color_style = if e.size_delta > 0 { &yellow } else { &blue };
                eprint!("{} ", color_style.apply_to(sym));
                print_colored_path(&e.rel);
                if !extra.is_empty() {
                    let pad = max_len.saturating_sub(e.rel.len()) + 4;
                    eprint!("{:>width$}", "", width = pad);
                    eprint!("{}", color_style.apply_to(&extra));
                }
                eprintln!();
            }
            DiffKind::Reused => {
                eprint!("{} ", cyan.apply_to("\u{21ba}"));
                print_colored_path(&e.rel);
                eprintln!();
            }
            DiffKind::Unchanged => {
                eprint!("{} ", dim.apply_to("="));
                print_colored_path(&e.rel);
                eprintln!();
            }
            DiffKind::Deleted => {
                eprint!("{} ", red.apply_to("-"));
                print_colored_path(&e.rel);
                eprintln!();
            }
        }
    }
}

/// 纯写入：将 new/modified（以及可选 unchanged）条目并行复制到 dest
pub(crate) fn apply_diff(
    entries: &[DiffEntry],
    dest: &Path,
    incremental: bool,
    prev_backup_dir: Option<&Path>,
    backend: FileCopyBackend,
) -> DiffStats {
    // 阶段1：预计算复制任务和目录，避免在并行阶段重复做路径转换/分支判断
    let mut dir_set = std::collections::HashSet::new();
    let mut copy_jobs: Vec<CopyJob> = Vec::new();
    let mut link_jobs: Vec<CopyJob> = Vec::new();
    let mut cnt_new = 0u32;
    let mut cnt_mod = 0u32;
    let mut cnt_reused = 0u32;
    let mut cnt_del = 0u32;
    let mut logical_bytes = 0u64;

    for e in entries {
        match e.kind {
            DiffKind::New => {
                cnt_new += 1;
                if let Some(src) = &e.src_path {
                    let dst = dest.join(e.rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
                    if let Some(parent) = dst.parent() {
                        dir_set.insert(parent.to_path_buf());
                    }
                    copy_jobs.push(CopyJob {
                        src: src.clone(),
                        dst,
                        link_wide: None,
                    });
                    logical_bytes += e.file_size;
                }
            }
            DiffKind::Modified => {
                cnt_mod += 1;
                if let Some(src) = &e.src_path {
                    let dst = dest.join(e.rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
                    if let Some(parent) = dst.parent() {
                        dir_set.insert(parent.to_path_buf());
                    }
                    copy_jobs.push(CopyJob {
                        src: src.clone(),
                        dst,
                        link_wide: None,
                    });
                    logical_bytes += e.file_size;
                }
            }
            DiffKind::Reused => {
                cnt_reused += 1;
                if let Some(src) = &e.src_path {
                    let dst = dest.join(e.rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
                    if let Some(parent) = dst.parent() {
                        dir_set.insert(parent.to_path_buf());
                    }
                    let link_wide = prev_backup_dir.and_then(|prev_dir| {
                        e.reuse_from_rel.as_ref().map(|reuse_from_rel| WideLinkPaths {
                            src: wide_null(
                                &prev_dir.join(
                                    reuse_from_rel.replace('\\', std::path::MAIN_SEPARATOR_STR),
                                ),
                            ),
                            dst: wide_null(&dst),
                        })
                    });
                    let job = CopyJob {
                        src: src.clone(),
                        dst,
                        link_wide,
                    };
                    if job.link_wide.is_some() {
                        link_jobs.push(job);
                    } else {
                        copy_jobs.push(job);
                    }
                    logical_bytes += e.file_size;
                }
            }
            DiffKind::Unchanged => {
                if !incremental && let Some(src) = &e.src_path {
                    let dst = dest.join(e.rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
                    if let Some(parent) = dst.parent() {
                        dir_set.insert(parent.to_path_buf());
                    }
                    let link_wide = prev_backup_dir.map(|prev_dir| WideLinkPaths {
                        src: wide_null(
                            &prev_dir.join(e.rel.replace('\\', std::path::MAIN_SEPARATOR_STR)),
                        ),
                        dst: wide_null(&dst),
                    });
                    let job = CopyJob {
                        src: src.clone(),
                        dst,
                        link_wide,
                    };
                    if job.link_wide.is_some() {
                        link_jobs.push(job);
                    } else {
                        copy_jobs.push(job);
                    }
                    logical_bytes += e.file_size;
                }
            }
            DiffKind::Deleted => {
                cnt_del += 1;
            }
        }
    }
    for dir in &dir_set {
        let _ = fs::create_dir_all(dir);
    }

    // 阶段2：unchanged 文件优先做 hardlink。路径在 job 构建阶段转成 UTF-16，
    // 避免每次 CreateHardLinkW 重复编码。
    let mut hardlinked_files = 0u32;
    let mut fallback_copy_jobs = Vec::new();
    for job in link_jobs {
        if try_hard_link(&job) {
            hardlinked_files += 1;
        } else {
            fallback_copy_jobs.push(CopyJob {
                src: job.src,
                dst: job.dst,
                link_wide: None,
            });
        }
    }
    copy_jobs.extend(fallback_copy_jobs);

    // 阶段3：真实文件复制保持并行。
    let copied_bytes: u64 = copy_jobs
        .par_iter()
        .map(|job| copy_file(&job.src, &job.dst, backend).unwrap_or(0))
        .sum();

    DiffStats {
        new: cnt_new,
        modified: cnt_mod,
        reused: cnt_reused,
        deleted: cnt_del,
        logical_bytes,
        copied_bytes,
        hardlinked_files,
    }
}

#[cfg(windows)]
fn try_hard_link(job: &CopyJob) -> bool {
    let Some(link_wide) = &job.link_wide else {
        return false;
    };
    use windows_sys::Win32::Storage::FileSystem::CreateHardLinkW;

    unsafe {
        CreateHardLinkW(
            link_wide.dst.as_ptr(),
            link_wide.src.as_ptr(),
            std::ptr::null(),
        ) != 0
    }
}

#[cfg(not(windows))]
fn try_hard_link(_job: &CopyJob) -> bool {
    false
}

#[cfg(windows)]
fn wide_null(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}

#[cfg(not(windows))]
fn wide_null(_path: &Path) -> Vec<u16> {
    Vec::new()
}

fn print_colored_path(rel: &str) {
    let dim = Style::new().dim();
    let yellow = Style::new().yellow();
    if let Some(pos) = rel.rfind('\\') {
        eprint!(
            "{}{}",
            dim.apply_to(&rel[..=pos]),
            yellow.apply_to(&rel[pos + 1..])
        );
    } else {
        eprint!("{}", yellow.apply_to(rel));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    use tempfile::tempdir;

    use super::{DiffEntry, DiffKind, apply_diff, compute_diff};
    use crate::backup::legacy::baseline::FileMeta;
    use crate::backup::legacy::scan::ScannedFile;
    use crate::windows::file_copy::FileCopyBackend;

    #[test]
    fn compute_diff_marks_mtime_difference_as_modified_even_if_not_newer() {
        let path = std::path::PathBuf::from("C:\\tmp\\a.txt");
        let newer = SystemTime::UNIX_EPOCH + Duration::from_secs(200);
        let older = SystemTime::UNIX_EPOCH + Duration::from_secs(100);

        let mut current = HashMap::new();
        current.insert(
            "a.txt".to_string(),
            ScannedFile {
                path: path.clone(),
                size: 10,
                modified: older,
                modified_ns: older
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
                created_time_ns: None,
                win_attributes: 0,
                file_id: None,
                content_hash: None,
            },
        );

        let mut old = HashMap::new();
        old.insert(
            "a.txt".to_string(),
            FileMeta {
                size: 10,
                modified: newer,
                modified_ns: newer
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
                created_time_ns: None,
                win_attributes: 0,
                file_id: None,
                content_hash: None,
            },
        );

        let diff = compute_diff(&current, &mut old, false);
        assert_eq!(diff.len(), 1);
        assert!(matches!(diff[0].kind, DiffKind::Modified));
    }

    #[test]
    fn compute_diff_prefers_content_hash_when_available() {
        let path = std::path::PathBuf::from("C:\\tmp\\a.txt");
        let time_a = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
        let time_b = SystemTime::UNIX_EPOCH + Duration::from_secs(200);

        let mut current = HashMap::new();
        current.insert(
            "a.txt".to_string(),
            ScannedFile {
                path: path.clone(),
                size: 10,
                modified: time_b,
                modified_ns: time_b
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
                created_time_ns: None,
                win_attributes: 0,
                file_id: None,
                content_hash: Some([7; 32]),
            },
        );

        let mut old = HashMap::new();
        old.insert(
            "a.txt".to_string(),
            FileMeta {
                size: 10,
                modified: time_a,
                modified_ns: time_a
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
                created_time_ns: None,
                win_attributes: 0,
                file_id: None,
                content_hash: Some([7; 32]),
            },
        );

        let diff = compute_diff(&current, &mut old, false);
        assert_eq!(diff.len(), 1);
        assert!(matches!(diff[0].kind, DiffKind::Unchanged));
    }

    #[test]
    fn apply_diff_reused_falls_back_to_copy_when_hardlink_fails() {
        let dir = tempdir().unwrap();
        let src_root = dir.path().join("src");
        let prev_root = dir.path().join("prev");
        let dst_root = dir.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&prev_root).unwrap();

        let src_path = src_root.join("same.txt");
        std::fs::write(&src_path, "same-content").unwrap();

        let stats = apply_diff(
            &[DiffEntry {
                rel: "same.txt".to_string(),
                src_path: Some(PathBuf::from(&src_path)),
                kind: DiffKind::Reused,
                size_delta: 0,
                file_size: 12,
                reuse_from_rel: Some("missing.txt".to_string()),
            }],
            &dst_root,
            false,
            Some(&prev_root),
            FileCopyBackend::Std,
        );

        assert_eq!(stats.reused, 1);
        assert_eq!(stats.hardlinked_files, 0);
        assert!(stats.copied_bytes > 0);
        assert_eq!(
            std::fs::read_to_string(dst_root.join("same.txt")).unwrap(),
            "same-content"
        );
    }
}
