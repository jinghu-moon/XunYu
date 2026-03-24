//! `xun backup verify <name>` — blake3 完整性校验

use std::path::Path;

use serde::Serialize;

use crate::output::{CliError, CliResult};

use super::checksum::{VerifyResult, verify_manifest};
use super::config::BackupConfig;
use super::meta::collect_backup_records;

#[derive(Serialize)]
struct BackupVerifyResultView {
    action: String,
    name: String,
    status: String,
    backup_type: String,
    corrupted_files: Vec<String>,
}

pub(crate) fn cmd_backup_verify(
    root: &Path,
    cfg: &BackupConfig,
    name: &str,
    json: bool,
) -> CliResult {
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
        if json {
            let payload = BackupVerifyResultView {
                action: "verify".to_string(),
                name: name.to_string(),
                status: "unsupported".to_string(),
                backup_type: "zip".to_string(),
                corrupted_files: Vec::new(),
            };
            out_println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_default()
            );
        }
        return Err(CliError::with_details(
            2,
            "Verify is only supported for directory backups.".to_string(),
            &["Hint: Re-run backup with --no-compress to use verify."],
        ));
    }

    match verify_manifest(&backup_path) {
        VerifyResult::Ok => {
            if json {
                let payload = BackupVerifyResultView {
                    action: "verify".to_string(),
                    name: name.to_string(),
                    status: "ok".to_string(),
                    backup_type: "dir".to_string(),
                    corrupted_files: Vec::new(),
                };
                out_println!(
                    "{}",
                    serde_json::to_string_pretty(&payload).unwrap_or_default()
                );
            } else {
                println!("✔ All files OK: {name}");
            }
            Ok(())
        }
        VerifyResult::Corrupted(files) => {
            if json {
                let payload = BackupVerifyResultView {
                    action: "verify".to_string(),
                    name: name.to_string(),
                    status: "corrupted".to_string(),
                    backup_type: "dir".to_string(),
                    corrupted_files: files.clone(),
                };
                out_println!(
                    "{}",
                    serde_json::to_string_pretty(&payload).unwrap_or_default()
                );
            }
            eprintln!("✘ CORRUPTED backup: {name}");
            for f in &files {
                eprintln!("  CORRUPTED: {f}");
            }
            Err(CliError::new(
                1,
                format!("{} file(s) corrupted", files.len()),
            ))
        }
        VerifyResult::NoManifest => {
            if json {
                let payload = BackupVerifyResultView {
                    action: "verify".to_string(),
                    name: name.to_string(),
                    status: "no_manifest".to_string(),
                    backup_type: "dir".to_string(),
                    corrupted_files: Vec::new(),
                };
                out_println!(
                    "{}",
                    serde_json::to_string_pretty(&payload).unwrap_or_default()
                );
            }
            Err(CliError::with_details(
                2,
                format!("No manifest found in backup: {name}"),
                &[
                    "Hint: Manifest is only generated for backups created after this update.",
                    "Hint: Re-create the backup to generate a manifest.",
                ],
            ))
        }
    }
}

fn locate_backup(backups_root: &Path, name: &str) -> Option<std::path::PathBuf> {
    collect_backup_records(backups_root, "")
        .into_iter()
        .find(|record| record.entry_name == name || record.display_name == name)
        .map(|record| record.path)
}
