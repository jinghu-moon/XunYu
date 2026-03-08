use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_COPY_ALLOWED, MOVEFILE_WRITE_THROUGH, MoveFileWithProgressW,
};

use crate::config::RedirectOnConflict;
use crate::output::can_interact;
use crate::security::audit::{AuditParams, audit_log};
use crate::windows::safety::ensure_safe_target;

use super::super::fs_utils::{
    sha256_file, unique_dest_path, unique_dest_path_with_timestamp, wide,
};
use super::report::UndoResult;

pub(super) fn undo_copy_one(
    tx: &str,
    src: &str,
    dst: &str,
    copy: bool,
    dry_run: bool,
) -> UndoResult {
    let _ = copy;
    let dst_path = PathBuf::from(dst);
    if dry_run {
        let mut p: BTreeMap<String, serde_json::Value> = BTreeMap::new();
        p.insert("tx".to_string(), serde_json::Value::String(tx.to_string()));
        p.insert(
            "dst".to_string(),
            serde_json::Value::String(dst.to_string()),
        );
        p.insert("dry_run".to_string(), serde_json::Value::Bool(true));
        audit_log(
            "redirect_undo_copy",
            src,
            "cli",
            AuditParams::Map(p),
            "success",
            "dry_run",
        );
        return UndoResult {
            action: "undo_copy".to_string(),
            src: src.to_string(),
            dst: dst.to_string(),
            result: "dry_run".to_string(),
            reason: "".to_string(),
        };
    }

    match std::fs::remove_file(&dst_path) {
        Ok(_) => {
            let mut p: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            p.insert("tx".to_string(), serde_json::Value::String(tx.to_string()));
            p.insert(
                "dst".to_string(),
                serde_json::Value::String(dst.to_string()),
            );
            audit_log(
                "redirect_undo_copy",
                src,
                "cli",
                AuditParams::Map(p),
                "success",
                "",
            );
            UndoResult {
                action: "undo_copy".to_string(),
                src: src.to_string(),
                dst: dst.to_string(),
                result: "success".to_string(),
                reason: "".to_string(),
            }
        }
        Err(e) => {
            let mut p: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            p.insert("tx".to_string(), serde_json::Value::String(tx.to_string()));
            p.insert(
                "dst".to_string(),
                serde_json::Value::String(dst.to_string()),
            );
            audit_log(
                "redirect_undo_copy",
                src,
                "cli",
                AuditParams::Map(p),
                "fail",
                &format!("remove_failed:{e}"),
            );
            UndoResult {
                action: "undo_copy".to_string(),
                src: src.to_string(),
                dst: dst.to_string(),
                result: "failed".to_string(),
                reason: format!("remove_failed:{e}"),
            }
        }
    }
}

pub(super) fn undo_move_one(
    tx: &str,
    original_src: &str,
    current_dst: &str,
    on_conflict: &RedirectOnConflict,
    dry_run: bool,
    yes: bool,
) -> UndoResult {
    let src_path = PathBuf::from(original_src);
    let mut dst_path = PathBuf::from(current_dst);

    if let Err(msg) = ensure_safe_target(&src_path) {
        return UndoResult {
            action: "undo_move".to_string(),
            src: current_dst.to_string(),
            dst: original_src.to_string(),
            result: "failed".to_string(),
            reason: format!("unsafe_restore_dest:{msg}"),
        };
    }

    if !dst_path.exists() {
        return UndoResult {
            action: "undo_move".to_string(),
            src: current_dst.to_string(),
            dst: original_src.to_string(),
            result: "failed".to_string(),
            reason: "missing_current_dst".to_string(),
        };
    }

    let mut warn: Option<String> = None;
    let mut restore_target = src_path.clone();

    if restore_target.exists() {
        if matches!(on_conflict, RedirectOnConflict::Skip) {
            return UndoResult {
                action: "undo_move".to_string(),
                src: current_dst.to_string(),
                dst: original_src.to_string(),
                result: "skipped".to_string(),
                reason: "exists".to_string(),
            };
        }
        if matches!(on_conflict, RedirectOnConflict::Overwrite) {
            if !can_interact() && !yes {
                return UndoResult {
                    action: "undo_move".to_string(),
                    src: current_dst.to_string(),
                    dst: original_src.to_string(),
                    result: "skipped".to_string(),
                    reason: "overwrite_requires_yes".to_string(),
                };
            }
            let _ = std::fs::remove_file(&restore_target);
        } else if matches!(
            on_conflict,
            RedirectOnConflict::RenameNew | RedirectOnConflict::HashDedup
        ) {
            if matches!(on_conflict, RedirectOnConflict::HashDedup) {
                match (sha256_file(&dst_path), sha256_file(&restore_target)) {
                    (Ok(a), Ok(b)) if a == b => {
                        if dry_run {
                            return UndoResult {
                                action: "undo_move".to_string(),
                                src: current_dst.to_string(),
                                dst: original_src.to_string(),
                                result: "dry_run".to_string(),
                                reason: "hash_dedup_same_deleted_dst".to_string(),
                            };
                        }
                        return match std::fs::remove_file(&dst_path) {
                            Ok(_) => UndoResult {
                                action: "undo_dedup".to_string(),
                                src: current_dst.to_string(),
                                dst: original_src.to_string(),
                                result: "success".to_string(),
                                reason: "hash_dedup_same_deleted_dst".to_string(),
                            },
                            Err(e) => UndoResult {
                                action: "undo_dedup".to_string(),
                                src: current_dst.to_string(),
                                dst: original_src.to_string(),
                                result: "failed".to_string(),
                                reason: format!("delete_dst_failed:{e}"),
                            },
                        };
                    }
                    (Err(e), _) => warn = Some(format!("hash_dedup_fallback_src_hash_failed:{e}")),
                    (_, Err(e)) => warn = Some(format!("hash_dedup_fallback_dst_hash_failed:{e}")),
                    _ => {}
                }
            }
            restore_target = unique_dest_path(&restore_target);
        } else if matches!(on_conflict, RedirectOnConflict::RenameDate) {
            restore_target = unique_dest_path_with_timestamp(&restore_target);
        } else {
            return UndoResult {
                action: "undo_move".to_string(),
                src: current_dst.to_string(),
                dst: original_src.to_string(),
                result: "failed".to_string(),
                reason: "unsupported_conflict".to_string(),
            };
        }
    }

    if dry_run {
        let mut p: BTreeMap<String, serde_json::Value> = BTreeMap::new();
        p.insert("tx".to_string(), serde_json::Value::String(tx.to_string()));
        p.insert(
            "dst".to_string(),
            serde_json::Value::String(restore_target.to_string_lossy().to_string()),
        );
        p.insert(
            "restored_to".to_string(),
            serde_json::Value::String(src_path.to_string_lossy().to_string()),
        );
        p.insert("dry_run".to_string(), serde_json::Value::Bool(true));
        audit_log(
            "redirect_undo_move",
            &dst_path.to_string_lossy(),
            "cli",
            AuditParams::Map(p),
            "success",
            "dry_run",
        );
        return UndoResult {
            action: "undo_move".to_string(),
            src: current_dst.to_string(),
            dst: restore_target.to_string_lossy().to_string(),
            result: "dry_run".to_string(),
            reason: warn.unwrap_or_default(),
        };
    }

    if let Some(parent) = restore_target.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return UndoResult {
                action: "undo_move".to_string(),
                src: current_dst.to_string(),
                dst: restore_target.to_string_lossy().to_string(),
                result: "failed".to_string(),
                reason: format!("mkdir_failed:{e}"),
            };
        }
    }

    let op = move_file_cross_volume_or_rename(&mut dst_path, &restore_target);
    match op {
        Ok(_) => {
            let reason = warn.unwrap_or_default();
            let mut p: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            p.insert("tx".to_string(), serde_json::Value::String(tx.to_string()));
            p.insert(
                "dst".to_string(),
                serde_json::Value::String(restore_target.to_string_lossy().to_string()),
            );
            audit_log(
                "redirect_undo_move",
                &dst_path.to_string_lossy(),
                "cli",
                AuditParams::Map(p),
                "success",
                &reason,
            );
            UndoResult {
                action: "undo_move".to_string(),
                src: current_dst.to_string(),
                dst: restore_target.to_string_lossy().to_string(),
                result: "success".to_string(),
                reason,
            }
        }
        Err(reason) => {
            let mut p: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            p.insert("tx".to_string(), serde_json::Value::String(tx.to_string()));
            p.insert(
                "dst".to_string(),
                serde_json::Value::String(restore_target.to_string_lossy().to_string()),
            );
            audit_log(
                "redirect_undo_move",
                &dst_path.to_string_lossy(),
                "cli",
                AuditParams::Map(p),
                "fail",
                &reason,
            );
            UndoResult {
                action: "undo_move".to_string(),
                src: current_dst.to_string(),
                dst: restore_target.to_string_lossy().to_string(),
                result: "failed".to_string(),
                reason,
            }
        }
    }
}

fn move_file_cross_volume_or_rename(src: &mut PathBuf, dst: &Path) -> Result<(), String> {
    if std::fs::rename(&*src, dst).is_ok() {
        return Ok(());
    }
    move_file_with_progress(src, dst)
}

fn move_file_with_progress(src: &Path, dst: &Path) -> Result<(), String> {
    let src_w = wide(src);
    let dst_w = wide(dst);
    let ok = unsafe {
        MoveFileWithProgressW(
            src_w.as_ptr(),
            dst_w.as_ptr(),
            None,
            std::ptr::null_mut(),
            MOVEFILE_COPY_ALLOWED | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        let err = unsafe { GetLastError() };
        return Err(format!("move_failed:os={err}"));
    }
    Ok(())
}
