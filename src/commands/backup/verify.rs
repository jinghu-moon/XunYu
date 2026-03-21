//! `xun backup verify <name>` — blake3 完整性校验

use std::fs;
use std::path::Path;

use crate::output::{CliError, CliResult};

use super::checksum::{VerifyResult, verify_manifest};
use super::config::BackupConfig;

pub(crate) fn cmd_backup_verify(root: &Path, cfg: &BackupConfig, name: &str) -> CliResult {
    let backups_root = root.join(&cfg.storage.backups_dir);

    // 定位备份（dir 或 zip 目录）
    let backup_path = locate_backup(&backups_root, name);
    let Some(backup_path) = backup_path else {
        return Err(CliError::with_details(
            2,
            format!("Backup not found: {name}"),
            &["Fix: Run `xun backup list` to see available backups."],
        ));
    };

    if backup_path.extension().is_some_and(|e| e == "zip") {
        return Err(CliError::with_details(
            2,
            "Verify is only supported for directory backups.".to_string(),
            &["Hint: Re-run backup with --no-compress to use verify."],
        ));
    }

    match verify_manifest(&backup_path) {
        VerifyResult::Ok => {
            println!("✔ All files OK: {name}");
            Ok(())
        }
        VerifyResult::Corrupted(files) => {
            eprintln!("✘ CORRUPTED backup: {name}");
            for f in &files {
                eprintln!("  CORRUPTED: {f}");
            }
            Err(CliError::new(
                1,
                format!("{} file(s) corrupted", files.len()),
            ))
        }
        VerifyResult::NoManifest => Err(CliError::with_details(
            2,
            format!("No manifest found in backup: {name}"),
            &[
                "Hint: Manifest is only generated for backups created after this update.",
                "Hint: Re-create the backup to generate a manifest.",
            ],
        )),
    }
}

fn locate_backup(backups_root: &Path, name: &str) -> Option<std::path::PathBuf> {
    // 精确匹配 dir 或 zip
    let dir = backups_root.join(name);
    if dir.is_dir() {
        return Some(dir);
    }
    let zip = backups_root.join(format!("{name}.zip"));
    if zip.is_file() {
        return Some(zip);
    }
    // 前缀模糊匹配
    if let Ok(rd) = fs::read_dir(backups_root) {
        for e in rd.flatten() {
            let entry_name = e.file_name().to_string_lossy().into_owned();
            let stem = entry_name.strip_suffix(".zip").unwrap_or(&entry_name);
            if stem == name || entry_name == name {
                return Some(e.path());
            }
        }
    }
    None
}
