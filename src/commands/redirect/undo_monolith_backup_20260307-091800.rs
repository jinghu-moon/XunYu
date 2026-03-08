use crate::config::RedirectOnConflict;
use crate::output::can_interact;
use crate::security::audit::{AuditParams, audit_file_path, audit_log};
use crate::windows::safety::ensure_safe_target;
use std::collections::BTreeMap;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_COPY_ALLOWED, MOVEFILE_WRITE_THROUGH, MoveFileWithProgressW,
};

use super::fs_utils::{sha256_file, unique_dest_path, unique_dest_path_with_timestamp, wide};

#[derive(Debug, Clone)]
struct AuditEntry {
    action: String,
    target: String,
    params: serde_json::Value,
    result: String,
    reason: String,
}

#[derive(Clone)]
pub(crate) struct UndoResult {
    pub(crate) action: String,
    pub(crate) src: String,
    pub(crate) dst: String,
    pub(crate) result: String,
    pub(crate) reason: String,
}

pub(crate) fn run_undo(
    tx: &str,
    on_conflict: &RedirectOnConflict,
    dry_run: bool,
    yes: bool,
) -> Vec<UndoResult> {
    let entries = load_audit_entries_for_tx(tx);
    let mut out = Vec::new();

    if entries.is_empty() {
        out.push(UndoResult {
            action: "undo".to_string(),
            src: "".to_string(),
            dst: "".to_string(),
            result: "skipped".to_string(),
            reason: "tx_not_found".to_string(),
        });
        return out;
    }

    for e in entries.into_iter().rev() {
        if e.result != "success" || e.reason == "dry_run" {
            continue;
        }

        let Some((dst, copy)) = parse_dst_copy_from_params(&e.params) else {
            out.push(UndoResult {
                action: format!("undo_{}", e.action),
                src: e.target.clone(),
                dst: "".to_string(),
                result: "failed".to_string(),
                reason: "missing_dst_or_copy".to_string(),
            });
            continue;
        };

        match e.action.as_str() {
            "redirect_move" => {
                out.push(undo_move_one(
                    tx,
                    &e.target,
                    &dst,
                    on_conflict,
                    dry_run,
                    yes,
                ));
            }
            "redirect_copy" => {
                out.push(undo_copy_one(tx, &e.target, &dst, copy, dry_run));
            }
            "redirect_dedup" => {
                out.push(UndoResult {
                    action: "undo_dedup".to_string(),
                    src: e.target.clone(),
                    dst,
                    result: "failed".to_string(),
                    reason: "dedup_cannot_restore_deleted_source".to_string(),
                });
            }
            _ => {
                out.push(UndoResult {
                    action: format!("undo_{}", e.action),
                    src: e.target.clone(),
                    dst,
                    result: "skipped".to_string(),
                    reason: "unsupported_action".to_string(),
                });
            }
        }
    }

    out
}

fn load_audit_entries_for_tx(tx: &str) -> Vec<AuditEntry> {
    let p = audit_file_path();
    let f = match std::fs::File::open(&p) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let rdr = io::BufReader::new(f);
    let mut out = Vec::new();
    for line in rdr.lines().flatten() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let action = v.get("action").and_then(|v| v.as_str()).unwrap_or("");
        if !action.starts_with("redirect_") {
            continue;
        }
        let params = v
            .get("params_json")
            .cloned()
            .or_else(|| v.get("params").cloned())
            .unwrap_or(serde_json::Value::Null);
        if !params_matches_tx(&params, tx) {
            continue;
        }
        out.push(AuditEntry {
            action: action.to_string(),
            target: v
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            params,
            result: v
                .get("result")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            reason: v
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }
    out
}

fn parse_tx_from_params_text(params: &str) -> Option<String> {
    let idx = params.find("tx=")?;
    let rest = &params[(idx + 3)..];
    let end = rest.find(' ').unwrap_or(rest.len());
    let tx = rest[..end].trim();
    if tx.is_empty() {
        None
    } else {
        Some(tx.to_string())
    }
}

fn params_matches_tx(params: &serde_json::Value, tx: &str) -> bool {
    match params {
        serde_json::Value::Object(map) => map
            .get("tx")
            .and_then(|v| v.as_str())
            .map(|v| v == tx)
            .unwrap_or(false),
        serde_json::Value::String(s) => parse_tx_from_params_text(s).as_deref() == Some(tx),
        _ => false,
    }
}

fn parse_dst_copy_from_params(params: &serde_json::Value) -> Option<(String, bool)> {
    match params {
        serde_json::Value::Object(map) => {
            let dst = map.get("dst").and_then(|v| v.as_str())?.trim();
            if dst.is_empty() {
                return None;
            }
            let copy = match map.get("copy") {
                Some(serde_json::Value::Bool(b)) => *b,
                Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0) != 0,
                Some(serde_json::Value::String(s)) => matches!(
                    s.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                ),
                _ => false,
            };
            Some((dst.to_string(), copy))
        }
        serde_json::Value::String(s) => parse_dst_copy_from_params_text(s),
        _ => None,
    }
}

fn parse_dst_copy_from_params_text(params: &str) -> Option<(String, bool)> {
    // Expected format: "tx=<id> dst=<path> copy=<bool>"
    // dst may contain spaces; copy is typically the last key.
    let copy_idx = find_bool_key_from_end(params, "copy")?;
    let copy_raw = params[(copy_idx + "copy=".len())..].trim();
    let copy = match copy_raw.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => return None,
    };

    let dst_idx = find_key(params, "dst=")?;
    let dst_start = dst_idx + "dst=".len();
    if dst_start >= copy_idx {
        return None;
    }
    let dst = params[dst_start..copy_idx].trim().to_string();
    if dst.is_empty() {
        return None;
    }
    Some((dst, copy))
}

fn find_key(params: &str, key: &str) -> Option<usize> {
    for (idx, _) in params.match_indices(key) {
        if idx == 0 {
            return Some(idx);
        }
        if params
            .as_bytes()
            .get(idx - 1)
            .map(|b| b.is_ascii_whitespace())
            .unwrap_or(false)
        {
            return Some(idx);
        }
    }
    None
}

fn find_bool_key_from_end(params: &str, key: &str) -> Option<usize> {
    let needle = format!("{key}=");
    let mut last_valid: Option<usize> = None;
    for (idx, _) in params.match_indices(&needle) {
        if idx == 0
            || params
                .as_bytes()
                .get(idx - 1)
                .map(|b| b.is_ascii_whitespace())
                .unwrap_or(false)
        {
            last_valid = Some(idx);
        }
    }
    last_valid
}

fn undo_copy_one(tx: &str, src: &str, dst: &str, copy: bool, dry_run: bool) -> UndoResult {
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

fn undo_move_one(
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
