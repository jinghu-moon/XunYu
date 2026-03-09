use std::path::PathBuf;

use crate::config::{RedirectOnConflict, RedirectProfile};
use crate::windows::safety::ensure_safe_target;

use super::super::super::plan::{ConflictAction, PlanItem, PlanKind, fingerprint_path};
use super::super::audit::{audit_if, audit_params_redirect};
use super::super::conflict::{ConflictResolution, resolve_conflict, resolve_conflict_with_source};
use super::super::ops::{copy_file, move_file};
use super::super::types::{RedirectOptions, RedirectResult};

fn map_action_to_on_conflict(action: ConflictAction) -> RedirectOnConflict {
    match action {
        ConflictAction::Skip => RedirectOnConflict::Skip,
        ConflictAction::RenameNew => RedirectOnConflict::RenameNew,
        ConflictAction::RenameDate => RedirectOnConflict::RenameDate,
        ConflictAction::RenameExisting => RedirectOnConflict::RenameExisting,
        ConflictAction::Overwrite => RedirectOnConflict::Overwrite,
        ConflictAction::Trash => RedirectOnConflict::Trash,
        ConflictAction::HashDedup => RedirectOnConflict::HashDedup,
    }
}

pub(super) fn apply_plan_item(
    tx: &str,
    profile: &RedirectProfile,
    opts: &RedirectOptions,
    it: &PlanItem,
    out: &mut Vec<RedirectResult>,
) {
    let src_path = PathBuf::from(&it.src);
    let mut dest = PathBuf::from(&it.dst);
    let is_copy = matches!(it.kind, PlanKind::Copy);
    let local_opts = RedirectOptions {
        copy: is_copy,
        ..*opts
    };

    if local_opts.dry_run {
        out.push(RedirectResult {
            action: match it.kind {
                PlanKind::Move => "move".to_string(),
                PlanKind::Copy => "copy".to_string(),
            },
            src: it.src.clone(),
            dst: it.dst.clone(),
            rule: it.rule.clone(),
            result: "dry_run".to_string(),
            reason: "".to_string(),
        });
        return;
    }

    if !src_path.exists() {
        out.push(RedirectResult {
            action: "fail".to_string(),
            src: it.src.clone(),
            dst: it.dst.clone(),
            rule: it.rule.clone(),
            result: "failed".to_string(),
            reason: "missing_source".to_string(),
        });
        return;
    }

    if let Some(fp) = &it.src_fp
        && let Some(cur) = fingerprint_path(&src_path)
        && (cur.size != fp.size || cur.mtime_ts != fp.mtime_ts)
    {
        out.push(RedirectResult {
            action: "skip".to_string(),
            src: it.src.clone(),
            dst: it.dst.clone(),
            rule: it.rule.clone(),
            result: "skipped".to_string(),
            reason: "stale".to_string(),
        });
        audit_if(
            &local_opts,
            "redirect_skip",
            &src_path.to_string_lossy(),
            "cli",
            audit_params_redirect(tx, dest.to_string_lossy().as_ref(), is_copy),
            "success",
            "stale",
        );
        return;
    }

    let Some(dest_dir) = dest.parent().map(|p| p.to_path_buf()) else {
        out.push(RedirectResult {
            action: "fail".to_string(),
            src: it.src.clone(),
            dst: it.dst.clone(),
            rule: it.rule.clone(),
            result: "failed".to_string(),
            reason: "invalid_dest".to_string(),
        });
        return;
    };
    if let Err(msg) = ensure_safe_target(&dest_dir) {
        let reason = format!("unsafe_dest:{msg}");
        out.push(RedirectResult {
            action: "skip".to_string(),
            src: it.src.clone(),
            dst: it.dst.clone(),
            rule: it.rule.clone(),
            result: "skipped".to_string(),
            reason: reason.clone(),
        });
        audit_if(
            &local_opts,
            "redirect_skip",
            &src_path.to_string_lossy(),
            "cli",
            audit_params_redirect(tx, dest_dir.to_string_lossy().as_ref(), is_copy),
            "success",
            &reason,
        );
        return;
    }

    let on_conflict = it
        .conflict_action
        .map(map_action_to_on_conflict)
        .unwrap_or_else(|| profile.on_conflict.clone());
    if matches!(on_conflict, RedirectOnConflict::Ask) && dest.exists() {
        out.push(RedirectResult {
            action: "fail".to_string(),
            src: it.src.clone(),
            dst: it.dst.clone(),
            rule: it.rule.clone(),
            result: "failed".to_string(),
            reason: "conflict_needs_input".to_string(),
        });
        return;
    }

    let mut warn_reason: Option<String> = None;
    match resolve_conflict_with_source(
        tx,
        &src_path,
        &mut dest,
        &on_conflict,
        &local_opts,
        &mut warn_reason,
    ) {
        ConflictResolution::Skip(reason) => {
            out.push(RedirectResult {
                action: "skip".to_string(),
                src: it.src.clone(),
                dst: dest.to_string_lossy().to_string(),
                rule: it.rule.clone(),
                result: "skipped".to_string(),
                reason: reason.clone(),
            });
            audit_if(
                &local_opts,
                "redirect_skip",
                &src_path.to_string_lossy(),
                "cli",
                audit_params_redirect(tx, dest.to_string_lossy().as_ref(), is_copy),
                "success",
                &reason,
            );
            return;
        }
        ConflictResolution::DedupDeleted(reason) => {
            out.push(RedirectResult {
                action: "dedup".to_string(),
                src: it.src.clone(),
                dst: dest.to_string_lossy().to_string(),
                rule: it.rule.clone(),
                result: "success".to_string(),
                reason: reason.clone(),
            });
            audit_if(
                &local_opts,
                "redirect_dedup",
                &src_path.to_string_lossy(),
                "cli",
                audit_params_redirect(tx, dest.to_string_lossy().as_ref(), is_copy),
                "success",
                &reason,
            );
            return;
        }
        ConflictResolution::Proceed => {}
    }

    if let Some(reason) = resolve_conflict(&mut dest, &on_conflict, &local_opts, &mut warn_reason) {
        out.push(RedirectResult {
            action: "skip".to_string(),
            src: it.src.clone(),
            dst: dest.to_string_lossy().to_string(),
            rule: it.rule.clone(),
            result: "skipped".to_string(),
            reason: reason.clone(),
        });
        audit_if(
            &local_opts,
            "redirect_skip",
            &src_path.to_string_lossy(),
            "cli",
            audit_params_redirect(tx, dest.to_string_lossy().as_ref(), is_copy),
            "success",
            &reason,
        );
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&dest_dir) {
        let reason = format!("mkdir_failed:{e}");
        out.push(RedirectResult {
            action: "fail".to_string(),
            src: it.src.clone(),
            dst: dest_dir.to_string_lossy().to_string(),
            rule: it.rule.clone(),
            result: "failed".to_string(),
            reason: reason.clone(),
        });
        audit_if(
            &local_opts,
            "redirect_skip",
            &src_path.to_string_lossy(),
            "cli",
            audit_params_redirect(tx, dest_dir.to_string_lossy().as_ref(), is_copy),
            "fail",
            "mkdir_failed",
        );
        return;
    }

    let op_res = match it.kind {
        PlanKind::Copy => copy_file(&src_path, &dest),
        PlanKind::Move => move_file(&src_path, &dest),
    };
    match op_res {
        Ok(warn) => {
            let warn_str = warn.or(warn_reason.clone()).unwrap_or_default();
            out.push(RedirectResult {
                action: match it.kind {
                    PlanKind::Copy => "copy".to_string(),
                    PlanKind::Move => "move".to_string(),
                },
                src: it.src.clone(),
                dst: dest.to_string_lossy().to_string(),
                rule: it.rule.clone(),
                result: "success".to_string(),
                reason: warn_str.clone(),
            });
            audit_if(
                &local_opts,
                if is_copy {
                    "redirect_copy"
                } else {
                    "redirect_move"
                },
                &src_path.to_string_lossy(),
                "cli",
                audit_params_redirect(tx, dest.to_string_lossy().as_ref(), is_copy),
                "success",
                &warn_str,
            );
        }
        Err(reason) => {
            out.push(RedirectResult {
                action: match it.kind {
                    PlanKind::Copy => "copy".to_string(),
                    PlanKind::Move => "move".to_string(),
                },
                src: it.src.clone(),
                dst: dest.to_string_lossy().to_string(),
                rule: it.rule.clone(),
                result: "failed".to_string(),
                reason: reason.clone(),
            });
            audit_if(
                &local_opts,
                if is_copy {
                    "redirect_copy"
                } else {
                    "redirect_move"
                },
                &src_path.to_string_lossy(),
                "cli",
                audit_params_redirect(tx, dest.to_string_lossy().as_ref(), is_copy),
                "fail",
                &reason,
            );
        }
    }
}
