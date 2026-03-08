mod common;
mod dry_run;

use crate::config::RedirectProfile;
use crate::security::audit::audit_log;
use crate::windows::safety::ensure_safe_target;

use super::audit::{audit_if, audit_params_redirect};
use super::conflict::{ConflictResolution, resolve_conflict, resolve_conflict_with_source};
use super::ops::{copy_file, move_file};
use super::path::compare_key;
use super::types::{RedirectOptions, RedirectResult};
use dry_run::handle_hash_dedup;

use std::path::Path;

pub(super) fn process_one_path(
    tx: &str,
    source_abs: &Path,
    profile: &RedirectProfile,
    opts: &RedirectOptions,
    src_path: &Path,
    out: &mut Vec<RedirectResult>,
) {
    let file_name = src_path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    if common::early_handle_non_file(
        tx,
        source_abs,
        profile,
        opts,
        src_path,
        file_name.as_deref(),
        out,
    ) {
        return;
    }

    let Some(file_name) = file_name else {
        return;
    };
    let (rule_name, dest_dir) =
        match common::resolve_rule_and_dest(tx, source_abs, profile, opts, src_path, out) {
            Some(v) => v,
            None => return,
        };

    if let Err(msg) = ensure_safe_target(&dest_dir) {
        let reason = format!("unsafe_dest:{msg}");
        out.push(RedirectResult {
            action: "skip".to_string(),
            src: src_path.to_string_lossy().to_string(),
            dst: dest_dir.to_string_lossy().to_string(),
            rule: rule_name.clone(),
            result: "skipped".to_string(),
            reason: reason.clone(),
        });
        audit_if(
            opts,
            "redirect_skip",
            &src_path.to_string_lossy(),
            "cli",
            audit_params_redirect(tx, dest_dir.to_string_lossy().as_ref(), opts.copy),
            "success",
            &reason,
        );
        return;
    }

    let mut dest = dest_dir.join(&file_name);
    let src_key = compare_key(src_path);
    let dst_key = compare_key(&dest);
    if src_key == dst_key {
        out.push(RedirectResult {
            action: "skip".to_string(),
            src: src_path.to_string_lossy().to_string(),
            dst: dest.to_string_lossy().to_string(),
            rule: rule_name.clone(),
            result: "skipped".to_string(),
            reason: "same_path".to_string(),
        });
        audit_log(
            "redirect_skip",
            &src_path.to_string_lossy(),
            "cli",
            audit_params_redirect(tx, dest.to_string_lossy().as_ref(), opts.copy),
            "success",
            "same_path",
        );
        return;
    }

    let mut warn_reason: Option<String> = None;
    if handle_hash_dedup(
        tx,
        &profile.on_conflict,
        opts,
        src_path,
        &dest,
        &rule_name,
        &mut warn_reason,
        out,
    ) {
        return;
    }

    if !opts.dry_run {
        match resolve_conflict_with_source(
            tx,
            src_path,
            &mut dest,
            &profile.on_conflict,
            opts,
            &mut warn_reason,
        ) {
            ConflictResolution::Skip(reason) => {
                out.push(RedirectResult {
                    action: "skip".to_string(),
                    src: src_path.to_string_lossy().to_string(),
                    dst: dest.to_string_lossy().to_string(),
                    rule: rule_name.clone(),
                    result: "skipped".to_string(),
                    reason: reason.clone(),
                });
                audit_log(
                    "redirect_skip",
                    &src_path.to_string_lossy(),
                    "cli",
                    audit_params_redirect(tx, dest.to_string_lossy().as_ref(), opts.copy),
                    "success",
                    &reason,
                );
                return;
            }
            ConflictResolution::DedupDeleted(reason) => {
                out.push(RedirectResult {
                    action: "dedup".to_string(),
                    src: src_path.to_string_lossy().to_string(),
                    dst: dest.to_string_lossy().to_string(),
                    rule: rule_name.clone(),
                    result: "success".to_string(),
                    reason: reason.clone(),
                });
                audit_log(
                    "redirect_dedup",
                    &src_path.to_string_lossy(),
                    "cli",
                    audit_params_redirect(tx, dest.to_string_lossy().as_ref(), opts.copy),
                    "success",
                    &reason,
                );
                return;
            }
            ConflictResolution::Proceed => {}
        }
    }

    if let Some(reason) = resolve_conflict(&mut dest, &profile.on_conflict, opts, &mut warn_reason)
    {
        out.push(RedirectResult {
            action: "skip".to_string(),
            src: src_path.to_string_lossy().to_string(),
            dst: dest.to_string_lossy().to_string(),
            rule: rule_name.clone(),
            result: "skipped".to_string(),
            reason: reason.clone(),
        });
        audit_log(
            "redirect_skip",
            &src_path.to_string_lossy(),
            "cli",
            audit_params_redirect(tx, dest.to_string_lossy().as_ref(), opts.copy),
            "success",
            &reason,
        );
        return;
    }

    #[cfg(feature = "protect")]
    {
        let rules = &crate::config::load_config().protect.rules;
        if let Some(_rule) = crate::protect::is_protected(rules, src_path, "move") {
            out.push(RedirectResult {
                action: "skip".to_string(),
                src: src_path.to_string_lossy().to_string(),
                dst: dest.to_string_lossy().to_string(),
                rule: rule_name.clone(),
                result: "skipped".to_string(),
                reason: "protected".to_string(),
            });
            audit_log(
                "redirect_skip",
                &src_path.to_string_lossy(),
                "cli",
                audit_params_redirect(tx, dest.to_string_lossy().as_ref(), opts.copy),
                "success",
                "protected",
            );
            return;
        }
    }

    if opts.dry_run {
        out.push(RedirectResult {
            action: if opts.copy { "copy" } else { "move" }.to_string(),
            src: src_path.to_string_lossy().to_string(),
            dst: dest.to_string_lossy().to_string(),
            rule: rule_name.clone(),
            result: "dry_run".to_string(),
            reason: warn_reason.unwrap_or_default(),
        });
        audit_if(
            opts,
            if opts.copy {
                "redirect_copy"
            } else {
                "redirect_move"
            },
            &src_path.to_string_lossy(),
            "cli",
            audit_params_redirect(tx, dest.to_string_lossy().as_ref(), opts.copy),
            "success",
            "dry_run",
        );
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&dest_dir) {
        out.push(RedirectResult {
            action: "fail".to_string(),
            src: src_path.to_string_lossy().to_string(),
            dst: dest_dir.to_string_lossy().to_string(),
            rule: rule_name.clone(),
            result: "failed".to_string(),
            reason: format!("mkdir_failed:{e}"),
        });
        audit_if(
            opts,
            "redirect_skip",
            &src_path.to_string_lossy(),
            "cli",
            audit_params_redirect(tx, dest_dir.to_string_lossy().as_ref(), opts.copy),
            "fail",
            "mkdir_failed",
        );
        return;
    }

    let op_res = if opts.copy {
        copy_file(src_path, &dest)
    } else {
        move_file(src_path, &dest)
    };

    match op_res {
        Ok(warn) => {
            let warn_str = warn.or_else(|| warn_reason.clone()).unwrap_or_default();
            out.push(RedirectResult {
                action: if opts.copy { "copy" } else { "move" }.to_string(),
                src: src_path.to_string_lossy().to_string(),
                dst: dest.to_string_lossy().to_string(),
                rule: rule_name.clone(),
                result: "success".to_string(),
                reason: warn_str.clone(),
            });
            audit_if(
                opts,
                if opts.copy {
                    "redirect_copy"
                } else {
                    "redirect_move"
                },
                &src_path.to_string_lossy(),
                "cli",
                audit_params_redirect(tx, dest.to_string_lossy().as_ref(), opts.copy),
                "success",
                &warn_str,
            );
        }
        Err(reason) => {
            out.push(RedirectResult {
                action: if opts.copy { "copy" } else { "move" }.to_string(),
                src: src_path.to_string_lossy().to_string(),
                dst: dest.to_string_lossy().to_string(),
                rule: rule_name.clone(),
                result: "failed".to_string(),
                reason: reason.clone(),
            });
            audit_if(
                opts,
                if opts.copy {
                    "redirect_copy"
                } else {
                    "redirect_move"
                },
                &src_path.to_string_lossy(),
                "cli",
                audit_params_redirect(tx, dest.to_string_lossy().as_ref(), opts.copy),
                "fail",
                &reason,
            );
        }
    }
}
