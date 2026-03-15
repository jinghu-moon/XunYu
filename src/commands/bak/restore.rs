use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::BakCmd;
use crate::output::{CliError, CliResult, can_interact};
use crate::path_guard::{PathPolicy, validate_paths};

use super::config::BakConfig;
use super::util::norm;

pub(crate) fn cmd_bak_restore(
    root: &Path,
    cfg: &BakConfig,
    name: &str,
    args: &BakCmd,
) -> CliResult {
    let backups_root = root.join(&cfg.storage.backups_dir);
    let Some(src) = backup_source_path(&backups_root, name) else {
        return Err(CliError::with_details(
            2,
            format!("Backup not found: {name}"),
            &["Fix: Run `xun bak list` and pick an existing name."],
        ));
    };

    if !args.yes && can_interact() {
        let ok = dialoguer::Confirm::new()
            .with_prompt("Restore may overwrite files in the project. Continue?")
            .default(false)
            .interact()
            .unwrap_or(false);
        if !ok {
            return Err(CliError::new(3, "Cancelled."));
        }
    }

    let file = args.file.as_deref().map(PathBuf::from);
    if let Some(ref rel) = file {
        let mut policy = PathPolicy::for_output();
        policy.allow_relative = true;
        let validation = validate_paths(vec![rel.to_string_lossy().to_string()], &policy);
        if !validation.issues.is_empty() {
            let mut details: Vec<String> = validation
                .issues
                .iter()
                .map(|issue| format!("Invalid restore path: {} ({})", issue.raw, issue.detail))
                .collect();
            details.push(
                "Fix: Use a relative path without '..' (e.g. src/main.rs).".to_string(),
            );
            return Err(CliError::with_details(
                2,
                "Invalid restore path.".to_string(),
                &details,
            ));
        }
    }
    if let Some(ref rel) = file
        && !is_safe_rel_path(rel)
    {
        return Err(CliError::with_details(
            2,
            format!("Unsafe restore path: {}", rel.display()),
            &["Fix: Pass a relative path without '..' (e.g. src/main.rs)."],
        ));
    }

    if src.is_dir() {
        restore_from_dir(&src, root, file.as_deref(), args.dry_run)?;
    } else {
        restore_from_zip(&src, root, file.as_deref(), args.dry_run)?;
    }

    if args.dry_run {
        ui_println!("bak restore: DRY RUN complete.");
    } else {
        ui_println!("bak restore: OK.");
    }
    Ok(())
}

fn backup_source_path(backups_root: &Path, name: &str) -> Option<PathBuf> {
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

fn is_safe_rel_path(p: &Path) -> bool {
    if p.is_absolute() {
        return false;
    }
    for c in p.components() {
        match c {
            std::path::Component::Normal(_) => {}
            _ => return false,
        }
    }
    true
}

fn restore_from_dir(
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
        fs::copy(&src, &dst).map_err(|e| CliError::new(1, format!("Restore failed: {e}")))?;
        return Ok(());
    }

    fn walk(src: &Path, base: &Path, dest_root: &Path, dry_run: bool) -> Result<(), CliError> {
        let Ok(rd) = fs::read_dir(src) else {
            return Ok(());
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, base, dest_root, dry_run)?;
            } else {
                let rel = p.strip_prefix(base).unwrap_or(&p);
                let dst = dest_root.join(rel);
                if let Some(parent) = dst.parent()
                    && !dry_run
                {
                    let _ = fs::create_dir_all(parent);
                }
                if dry_run {
                    ui_println!("DRY RUN: would restore {}", rel.display());
                } else {
                    fs::copy(&p, &dst)
                        .map_err(|e| CliError::new(1, format!("Restore failed: {e}")))?;
                }
            }
        }
        Ok(())
    }

    walk(src_dir, src_dir, dest_root, dry_run)?;
    Ok(())
}

fn restore_from_zip(
    zip_path: &Path,
    dest_root: &Path,
    file: Option<&Path>,
    dry_run: bool,
) -> CliResult {
    let file_in = fs::File::open(zip_path)
        .map_err(|e| CliError::new(1, format!("Failed to open zip: {e}")))?;
    let mut archive =
        zip::ZipArchive::new(file_in).map_err(|e| CliError::new(1, format!("Invalid zip: {e}")))?;

    let want = file.map(|p| norm(&p.to_string_lossy()));

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CliError::new(1, format!("Zip read failed: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let name = norm(entry.name());
        if name.is_empty() {
            continue;
        }
        if let Some(ref w) = want
            && &name != w
        {
            continue;
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
