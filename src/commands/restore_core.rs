use std::fs;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use rayon::prelude::*;

use crate::output::{CliError, CliResult};
use crate::path_guard::{PathPolicy, validate_paths};
use crate::windows::file_copy::{copy_file, detect_copy_backend_for_restore};

const BACKUP_META_FILE: &str = ".bak-meta.json";
const BACKUP_MANIFEST_FILE: &str = ".bak-manifest.json";

fn restore_timing_enabled() -> bool {
    restore_timing_enabled_with(|name| std::env::var_os(name))
}

fn restore_timing_enabled_with<F>(mut get_env: F) -> bool
where
    F: FnMut(&str) -> Option<OsString>,
{
    ["XUN_CMD_TIMING", "XUN_RESTORE_TIMING"]
        .into_iter()
        .any(|name| get_env(name).is_some())
}

fn emit_restore_core_timing(label: &str, elapsed: std::time::Duration, extra: Option<String>) {
    match extra {
        Some(extra) if !extra.is_empty() => {
            eprintln!("    [{label:<12}] {:>5}ms  {extra}", elapsed.as_millis());
        }
        _ => eprintln!("    [{label:<12}] {:>5}ms", elapsed.as_millis()),
    }
}

struct RestoreCopyJob {
    src: PathBuf,
    dst: PathBuf,
    rel_display: String,
}

fn norm_path_display(p: &str) -> String {
    p.trim().replace('/', "\\").trim_matches('\\').to_string()
}

pub(crate) fn is_safe_zip_entry(name: &str) -> bool {
    let rel = Path::new(name);
    if rel.is_absolute()
        || rel.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return false;
    }
    let mut policy = PathPolicy::for_output();
    policy.allow_relative = true;
    validate_paths([name], &policy).issues.is_empty()
}

pub(crate) fn is_backup_internal_name(name: &str) -> bool {
    Path::new(name)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some_and(|file_name| {
            file_name == BACKUP_META_FILE || file_name == BACKUP_MANIFEST_FILE
        })
}

pub(crate) fn is_backup_internal_rel_path(rel: &Path) -> bool {
    rel.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == BACKUP_META_FILE || name == BACKUP_MANIFEST_FILE)
}

pub(crate) fn restore_from_dir(
    src_dir: &Path,
    dest_root: &Path,
    file: Option<&Path>,
    dry_run: bool,
) -> CliResult {
    let timing = restore_timing_enabled();
    let copy_backend = detect_copy_backend_for_restore();
    if let Some(rel) = file {
        if is_backup_internal_rel_path(rel) {
            return Err(CliError::new(
                1,
                "Restore failed: backup internal files cannot be restored.",
            ));
        }
        let src = src_dir.join(rel);
        let dst = dest_root.join(rel);
        if let Some(parent) = dst.parent()
            && !dry_run
        {
            let _ = fs::create_dir_all(parent);
        }
        if dry_run {
            ui_println!("DRY RUN: would restore {}", rel.display());
            return Ok(());
        }
        copy_file(&src, &dst, copy_backend)
            .map_err(|e| CliError::new(1, format!("Restore failed: {e}")))?;
        return Ok(());
    }

    let t_collect = Instant::now();
    let entries = collect_files_recursive(src_dir);
    if timing {
        emit_restore_core_timing(
            "collect-dir",
            t_collect.elapsed(),
            Some(format!("files={}", entries.len())),
        );
    }
    let had_error = AtomicBool::new(false);
    let t_copy = Instant::now();
    let mut dir_set = std::collections::HashSet::new();
    for src_path in &entries {
        let rel = match src_path.strip_prefix(src_dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if is_backup_internal_rel_path(rel) {
            continue;
        }
        if let Some(parent) = dest_root.join(rel).parent() {
            dir_set.insert(parent.to_path_buf());
        }
    }
    for dir in &dir_set {
        let _ = fs::create_dir_all(dir);
    }

    entries.par_iter().for_each(|src_path| {
        let rel = match src_path.strip_prefix(src_dir) {
            Ok(r) => r,
            Err(_) => return,
        };
        if is_backup_internal_rel_path(rel) {
            return;
        }

        let dst = dest_root.join(rel);
        if dry_run {
            ui_println!("DRY RUN: would restore {}", norm_path_display(&rel.to_string_lossy()));
            return;
        }
        if let Err(e) = copy_file(src_path, &dst, copy_backend) {
            eprintln!(
                "Restore error {}: {e}",
                norm_path_display(&rel.to_string_lossy())
            );
            had_error.store(true, Ordering::Relaxed);
        }
    });

    if had_error.load(Ordering::Relaxed) {
        return Err(CliError::new(1, "Some files failed to restore."));
    }
    if timing {
        emit_restore_core_timing(
            "copy-dir",
            t_copy.elapsed(),
            Some(format!("backend={copy_backend:?}")),
        );
    }
    Ok(())
}

pub(crate) fn restore_from_zip(
    zip_path: &Path,
    dest_root: &Path,
    file: Option<&Path>,
    dry_run: bool,
) -> CliResult {
    let timing = restore_timing_enabled();
    let t_open = Instant::now();
    let file_handle =
        fs::File::open(zip_path).map_err(|e| CliError::new(1, format!("Open zip failed: {e}")))?;
    let mut archive = zip::ZipArchive::new(file_handle)
        .map_err(|e| CliError::new(1, format!("Read zip failed: {e}")))?;
    if timing {
        emit_restore_core_timing("open-zip", t_open.elapsed(), Some(zip_path.display().to_string()));
    }

    let want = file.map(|path| path.to_string_lossy().replace('\\', "/"));
    if let Some(ref wanted) = want
        && is_backup_internal_name(wanted)
    {
        return Err(CliError::new(
            1,
            "Restore failed: backup internal files cannot be restored.",
        ));
    }
    let mut matched = false;
    let mut created_dirs = std::collections::HashSet::new();
    let t_iter = Instant::now();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CliError::new(1, format!("Zip entry error: {e}")))?;
        if entry.is_dir() {
            continue;
        }

        let name = entry.name().to_owned();
        let name_norm = name.replace('\\', "/");
        if !is_safe_zip_entry(&name_norm) {
            eprintln!("Skipping unsafe zip entry: {name}");
            continue;
        }
        if is_backup_internal_name(&name_norm) {
            continue;
        }
        if let Some(ref wanted) = want
            && name_norm != wanted.replace('\\', "/")
        {
            continue;
        }

        matched = true;
        let rel = PathBuf::from(name.replace('/', "\\"));
        let dst = dest_root.join(&rel);
        if let Some(parent) = dst.parent() && !dry_run && created_dirs.insert(parent.to_path_buf()) {
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

    if let Some(wanted) = want
        && !matched
    {
        return Err(CliError::new(
            1,
            format!("Restore failed: file not found in backup: {wanted}"),
        ));
    }
    if timing {
        emit_restore_core_timing("copy-zip", t_iter.elapsed(), None);
    }
    Ok(())
}

pub(crate) fn restore_many_from_dir<F>(
    src_dir: &Path,
    dest_root: &Path,
    dry_run: bool,
    filter: F,
) -> (usize, usize)
where
    F: Fn(&Path, &str) -> bool + Sync,
{
    let timing = restore_timing_enabled();
    let copy_backend = detect_copy_backend_for_restore();
    let t_collect = Instant::now();
    let entries = collect_files_recursive(src_dir);
    if timing {
        emit_restore_core_timing(
            "collect-dir",
            t_collect.elapsed(),
            Some(format!("files={}", entries.len())),
        );
    }
    let restored = AtomicUsize::new(0);
    let fail_count = AtomicUsize::new(0);
    let t_copy = Instant::now();

    let mut dir_set = std::collections::HashSet::new();
    let mut jobs: Vec<RestoreCopyJob> = Vec::new();
    for src_path in &entries {
        let rel = match src_path.strip_prefix(src_dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if is_backup_internal_rel_path(rel) {
            continue;
        }
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if !filter(rel, &rel_str) {
            continue;
        }
        let dst = dest_root.join(rel);
        if let Some(parent) = dst.parent() {
            dir_set.insert(parent.to_path_buf());
        }
        jobs.push(RestoreCopyJob {
            src: src_path.clone(),
            dst,
            rel_display: rel.display().to_string(),
        });
    }
    if !dry_run {
        for dir in &dir_set {
            let _ = fs::create_dir_all(dir);
        }
    }

    jobs.par_iter().for_each(|job| {
        if dry_run {
            eprintln!("DRY RUN: would restore {}", job.rel_display);
            restored.fetch_add(1, Ordering::Relaxed);
            return;
        }
        match copy_file(&job.src, &job.dst, copy_backend) {
            Ok(_) => {
                restored.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                eprintln!("Error restoring {}: {e}", job.rel_display);
                fail_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    });

    if timing {
        emit_restore_core_timing(
            "copy-dir",
            t_copy.elapsed(),
            Some(format!("backend={copy_backend:?}")),
        );
    }

    (
        restored.load(Ordering::Relaxed),
        fail_count.load(Ordering::Relaxed),
    )
}

pub(crate) fn restore_many_from_zip<F>(
    zip_path: &Path,
    dest_root: &Path,
    dry_run: bool,
    filter: F,
) -> Result<(usize, usize), CliError>
where
    F: Fn(&str) -> bool,
{
    let timing = restore_timing_enabled();
    let t_open = Instant::now();
    let file_handle =
        fs::File::open(zip_path).map_err(|e| CliError::new(1, format!("Open zip failed: {e}")))?;
    let mut archive = zip::ZipArchive::new(file_handle)
        .map_err(|e| CliError::new(1, format!("Read zip failed: {e}")))?;
    if timing {
        emit_restore_core_timing("open-zip", t_open.elapsed(), Some(zip_path.display().to_string()));
    }

    let mut restored = 0usize;
    let mut failed = 0usize;
    let mut created_dirs = std::collections::HashSet::new();
    let t_iter = Instant::now();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CliError::new(1, format!("Zip entry error: {e}")))?;
        if entry.is_dir() {
            continue;
        }

        let name = entry.name().to_owned();
        let name_norm = name.replace('\\', "/");
        if !filter(&name_norm) {
            continue;
        }
        if !is_safe_zip_entry(&name_norm) {
            eprintln!("Skipping unsafe zip entry: {name}");
            failed += 1;
            continue;
        }
        if is_backup_internal_name(&name_norm) {
            continue;
        }

        let rel = PathBuf::from(name.replace('/', "\\"));
        let dst = dest_root.join(&rel);
        if dry_run {
            eprintln!("DRY RUN: would restore {}", rel.display());
            restored += 1;
            continue;
        }
        if let Some(parent) = dst.parent() && created_dirs.insert(parent.to_path_buf()) {
            let _ = fs::create_dir_all(parent);
        }
        match fs::File::create(&dst) {
            Ok(mut out) => {
                if let Err(e) = std::io::copy(&mut entry, &mut out) {
                    eprintln!("Error writing {}: {e}", rel.display());
                    failed += 1;
                } else {
                    restored += 1;
                }
            }
            Err(e) => {
                eprintln!("Error creating {}: {e}", rel.display());
                failed += 1;
            }
        }
    }

    if timing {
        emit_restore_core_timing(
            "copy-zip",
            t_iter.elapsed(),
            Some(format!("restored={restored} failed={failed}")),
        );
    }

    Ok((restored, failed))
}

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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use tempfile::tempdir;

    #[test]
    fn zip_entry_with_parent_dir_is_unsafe() {
        assert!(!super::is_safe_zip_entry("../evil.txt"));
        assert!(!super::is_safe_zip_entry("..\\evil.txt"));
    }

    #[test]
    fn backup_internal_files_are_detected() {
        assert!(super::is_backup_internal_name(".bak-meta.json"));
        assert!(super::is_backup_internal_name(".bak-manifest.json"));
        assert!(!super::is_backup_internal_name("data.txt"));
    }

    #[test]
    fn restore_many_from_dir_counts_partial_failures() {
        let src = tempdir().unwrap();
        let dest = tempdir().unwrap();

        std::fs::write(src.path().join("ok.txt"), "ok").unwrap();
        std::fs::create_dir_all(src.path().join("blocked")).unwrap();
        std::fs::write(src.path().join("blocked").join("fail.txt"), "fail").unwrap();

        std::fs::write(dest.path().join("blocked"), "collision").unwrap();

        let (restored, failed) =
            super::restore_many_from_dir(src.path(), dest.path(), false, |_, _| true);

        assert_eq!(restored, 1);
        assert_eq!(failed, 1);
        assert!(dest.path().join("ok.txt").exists());
        assert!(!dest.path().join("blocked").join("fail.txt").exists());
    }

    #[test]
    fn restore_timing_enabled_accepts_command_and_restore_env_names() {
        let env = HashMap::from([("XUN_CMD_TIMING", OsString::from("1"))]);
        assert!(super::restore_timing_enabled_with(|name| env.get(name).cloned()));

        let env = HashMap::from([("XUN_RESTORE_TIMING", OsString::from("1"))]);
        assert!(super::restore_timing_enabled_with(|name| env.get(name).cloned()));
    }
}
