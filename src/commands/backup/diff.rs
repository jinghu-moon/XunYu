use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use console::Style;
use rayon::prelude::*;

use super::baseline::FileMeta;
use super::util::fmt_size;

pub(crate) struct DiffStats {
    pub(crate) new: u32,
    pub(crate) modified: u32,
    pub(crate) deleted: u32,
}

/// diff 条目（纯数据）
pub(crate) struct DiffEntry {
    pub(crate) rel: String,
    /// src_path 仅 new/modified 有效；deleted 为 None
    pub(crate) src_path: Option<PathBuf>,
    pub(crate) kind: DiffKind,
    pub(crate) size_delta: i64,
}

#[derive(PartialEq, Eq)]
pub(crate) enum DiffKind {
    New,
    Modified,
    Unchanged,
    Deleted,
}

/// 纯计算：对比 current 文件集与 old 快照，返回所有 diff 条目
pub(crate) fn compute_diff(
    current: &HashMap<String, PathBuf>,
    old: &mut HashMap<String, FileMeta>,
    skip_unchanged: bool,
) -> Vec<DiffEntry> {
    let mut entries: Vec<DiffEntry> = Vec::new();

    for (rel, src_path) in current {
        let meta = match fs::metadata(src_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if let Some(old_meta) = old.remove(rel) {
            let size_changed = meta.len() != old_meta.size;
            // 精确 mtime 比较，无容差
            let time_changed =
                meta.modified().unwrap_or(SystemTime::UNIX_EPOCH) > old_meta.modified;
            if size_changed || time_changed {
                let delta = meta.len() as i64 - old_meta.size as i64;
                entries.push(DiffEntry {
                    rel: rel.clone(),
                    src_path: Some(src_path.clone()),
                    kind: DiffKind::Modified,
                    size_delta: delta,
                });
            } else if !skip_unchanged {
                entries.push(DiffEntry {
                    rel: rel.clone(),
                    src_path: Some(src_path.clone()),
                    kind: DiffKind::Unchanged,
                    size_delta: 0,
                });
            }
        } else {
            entries.push(DiffEntry {
                rel: rel.clone(),
                src_path: Some(src_path.clone()),
                kind: DiffKind::New,
                size_delta: meta.len() as i64,
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
        });
    }

    entries
}

/// 纯输出：将 diff 条目打印到 stderr
pub(crate) fn print_diff(entries: &[DiffEntry], show_unchanged: bool) {
    let green = Style::new().green();
    let yellow = Style::new().yellow();
    let blue = Style::new().blue();
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
                    ("\u{2022}", String::new())          // •
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
pub(crate) fn apply_diff(entries: &[DiffEntry], dest: &Path, incremental: bool) -> DiffStats {
    // 阶段1：串行预建目录（避免并发 create_dir_all 竞争）
    let mut dir_set = std::collections::HashSet::new();
    for e in entries {
        let needs_copy = match e.kind {
            DiffKind::New | DiffKind::Modified => e.src_path.is_some(),
            DiffKind::Unchanged => !incremental && e.src_path.is_some(),
            DiffKind::Deleted => false,
        };
        if needs_copy {
            let dst = dest.join(e.rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
            if let Some(p) = dst.parent() {
                dir_set.insert(p.to_path_buf());
            }
        }
    }
    for dir in &dir_set {
        let _ = fs::create_dir_all(dir);
    }

    // 阶段2：rayon 并行复制
    let cnt_new = AtomicU32::new(0);
    let cnt_mod = AtomicU32::new(0);
    let cnt_del = AtomicU32::new(0);

    entries.par_iter().for_each(|e| match e.kind {
        DiffKind::New => {
            if let Some(src) = &e.src_path {
                let dst = dest.join(e.rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
                let _ = fs::copy(src, &dst);
            }
            cnt_new.fetch_add(1, Ordering::Relaxed);
        }
        DiffKind::Modified => {
            if let Some(src) = &e.src_path {
                let dst = dest.join(e.rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
                let _ = fs::copy(src, &dst);
            }
            cnt_mod.fetch_add(1, Ordering::Relaxed);
        }
        DiffKind::Unchanged => {
            if !incremental
                && let Some(src) = &e.src_path
            {
                let dst = dest.join(e.rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
                let _ = fs::copy(src, &dst);
            }
        }
        DiffKind::Deleted => {
            cnt_del.fetch_add(1, Ordering::Relaxed);
        }
    });

    DiffStats {
        new: cnt_new.into_inner(),
        modified: cnt_mod.into_inner(),
        deleted: cnt_del.into_inner(),
    }
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
