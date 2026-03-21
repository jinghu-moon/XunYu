use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use crate::output::{CliError, CliResult};
use crate::path_guard::{PathPolicy, validate_paths};

use super::baseline::read_baseline;
use super::config::BakConfig;
use super::diff::{DiffKind, compute_diff};
use super::scan::scan_files;
use super::util::norm;

/// zip entry 路径安全校验：使用 path_guard 检测路径穿越、非法字符等
fn is_safe_zip_entry(name: &str) -> bool {
    let mut policy = PathPolicy::for_output();
    policy.allow_relative = true;
    let result = validate_paths([name], &policy);
    result.issues.is_empty()
}

pub(crate) fn backup_source_path(backups_root: &Path, name: &str) -> Option<PathBuf> {
    let candidate = backups_root.join(name);
    if candidate.is_dir() || candidate.is_file() {
        return Some(candidate);
    }
    let zip = backups_root.join(format!("{name}.zip"));
    if zip.is_file() {
        return Some(zip);
    }
    None
}

pub(crate) fn restore_from_dir(
    src_dir: &Path,
    dest_root: &Path,
    file: Option<&Path>,
    dry_run: bool,
) -> CliResult {
    if let Some(rel) = file {
        let src = src_dir.join(rel);
        let dst = dest_root.join(rel);
        if let Some(p) = dst.parent()
            && !dry_run
        {
            let _ = fs::create_dir_all(p);
        }
        if dry_run {
            ui_println!("DRY RUN: would restore {}", rel.display());
            return Ok(());
        }
        fs::copy(&src, &dst)
            .map_err(|e| CliError::new(1, format!("Restore failed: {e}")))?;
        return Ok(());
    }

    // 全量还原：递归收集文件列表
    let entries = collect_files_recursive(src_dir);

    let had_error = AtomicBool::new(false);
    entries.par_iter().for_each(|src_path| {
        let rel = match src_path.strip_prefix(src_dir) {
            Ok(r) => r,
            Err(_) => return,
        };
        let dst = dest_root.join(rel);
        if dry_run {
            ui_println!("DRY RUN: would restore {}", norm(&rel.to_string_lossy()));
            return;
        }
        if let Some(p) = dst.parent() {
            let _ = fs::create_dir_all(p);
        }
        if let Err(e) = fs::copy(src_path, &dst) {
            eprintln!("Restore error {}: {e}", norm(&rel.to_string_lossy()));
            had_error.store(true, Ordering::Relaxed);
        }
    });

    if had_error.load(Ordering::Relaxed) {
        return Err(CliError::new(1, "Some files failed to restore."));
    }
    Ok(())
}

pub(crate) fn restore_from_zip(
    zip_path: &Path,
    dest_root: &Path,
    file: Option<&Path>,
    dry_run: bool,
) -> CliResult {
    let f =
        fs::File::open(zip_path).map_err(|e| CliError::new(1, format!("Open zip failed: {e}")))?;
    let mut archive = zip::ZipArchive::new(f)
        .map_err(|e| CliError::new(1, format!("Read zip failed: {e}")))?;

    let want = file.map(|p| p.to_string_lossy().replace('\\', "/"));

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CliError::new(1, format!("Zip entry error: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_owned();
        // path_guard 校验：拒绝路径穿越、绝对路径、非法字符
        let name_for_check = name.replace('\\', "/");
        if !is_safe_zip_entry(&name_for_check) {
            eprintln!("Skipping unsafe zip entry: {name}");
            continue;
        }
        if let Some(ref w) = want {
            let w_norm = w.replace('\\', "/");
            let n_norm = name.replace('\\', "/");
            if n_norm != w_norm {
                continue;
            }
        }
        let rel = PathBuf::from(name.replace('/', "\\"));
        let dst = dest_root.join(&rel);
        if let Some(parent) = dst.parent()
            && !dry_run
        {
            let _ = fs::create_dir_all(parent);
        }
        if dry_run {
            ui_println!("DRY RUN: would restore {}", rel.display());
            if want.is_some() {
                break;
            }
            continue;
        }
        let mut out =
            fs::File::create(&dst).map_err(|e| CliError::new(1, format!("Restore failed: {e}")))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(|e| CliError::new(1, format!("Restore failed: {e}")))?;
        if want.is_some() {
            break;
        }
    }
    Ok(())
}

/// restore 前展示将被覆盖的文件列表（modify/new 文件数）
pub(crate) fn show_restore_preview(root: &Path, cfg: &BakConfig, backup_src: &Path) {
    // 用备份快照作为 "current"（还原源），用工作目录作为 "old"（当前状态）
    // compute_diff 会找出哪些文件将被新增/覆盖
    let backup_files = scan_files(backup_src, &[], &[], &[]);
    let mut current_snapshot = read_baseline(root);

    let entries = compute_diff(&backup_files, &mut current_snapshot, true);

    let overwrite_count = entries
        .iter()
        .filter(|e| e.kind == DiffKind::Modified)
        .count();
    let new_count = entries
        .iter()
        .filter(|e| e.kind == DiffKind::New)
        .count();

    if overwrite_count == 0 && new_count == 0 {
        eprintln!("  (no files will be changed)");
        return;
    }

    eprintln!("Files to be restored:");
    let max_show = 20usize;
    let mut shown = 0;
    for e in &entries {
        if e.kind == DiffKind::Modified || e.kind == DiffKind::New {
            if shown < max_show {
                let tag = if e.kind == DiffKind::Modified { "overwrite" } else { "new" };
                eprintln!("  [{tag}] {}", e.rel);
            }
            shown += 1;
        }
    }
    if shown > max_show {
        eprintln!("  ... and {} more", shown - max_show);
    }
    eprintln!("  Total: {} overwrite, {} new", overwrite_count, new_count);
    // cfg 备用（后续可按需使用 backups_dir 等字段）
    let _ = cfg;
}

/// 递归收集目录下所有文件（不依赖 walkdir）
fn collect_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    collect_recursive_inner(dir, &mut result);
    result
}

fn collect_recursive_inner(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive_inner(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}
