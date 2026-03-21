use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::output::{CliError, CliResult};
use crate::path_guard::{PathPolicy, validate_paths};

const BACKUP_META_FILE: &str = ".bak-meta.json";
const BACKUP_MANIFEST_FILE: &str = ".bak-manifest.json";

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
        fs::copy(&src, &dst).map_err(|e| CliError::new(1, format!("Restore failed: {e}")))?;
        return Ok(());
    }

    let entries = collect_files_recursive(src_dir);
    let had_error = AtomicBool::new(false);

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
        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::copy(src_path, &dst) {
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
    Ok(())
}

pub(crate) fn restore_from_zip(
    zip_path: &Path,
    dest_root: &Path,
    file: Option<&Path>,
    dry_run: bool,
) -> CliResult {
    let file_handle =
        fs::File::open(zip_path).map_err(|e| CliError::new(1, format!("Open zip failed: {e}")))?;
    let mut archive = zip::ZipArchive::new(file_handle)
        .map_err(|e| CliError::new(1, format!("Read zip failed: {e}")))?;

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

    if let Some(wanted) = want
        && !matched
    {
        return Err(CliError::new(
            1,
            format!("Restore failed: file not found in backup: {wanted}"),
        ));
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
    let entries = collect_files_recursive(src_dir);
    let restored = AtomicUsize::new(0);
    let fail_count = AtomicUsize::new(0);

    entries.par_iter().for_each(|src_path| {
        let rel = match src_path.strip_prefix(src_dir) {
            Ok(r) => r,
            Err(_) => return,
        };
        if is_backup_internal_rel_path(rel) {
            return;
        }
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if !filter(rel, &rel_str) {
            return;
        }

        let dst = dest_root.join(rel);
        if dry_run {
            eprintln!("DRY RUN: would restore {}", rel.display());
            restored.fetch_add(1, Ordering::Relaxed);
            return;
        }
        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::copy(src_path, &dst) {
            Ok(_) => {
                restored.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                eprintln!("Error restoring {}: {e}", rel.display());
                fail_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    });

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
    let file_handle =
        fs::File::open(zip_path).map_err(|e| CliError::new(1, format!("Open zip failed: {e}")))?;
    let mut archive = zip::ZipArchive::new(file_handle)
        .map_err(|e| CliError::new(1, format!("Read zip failed: {e}")))?;

    let mut restored = 0usize;
    let mut failed = 0usize;

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
        if let Some(parent) = dst.parent() {
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
}
