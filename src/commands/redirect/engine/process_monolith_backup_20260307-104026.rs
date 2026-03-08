use crate::config::{RedirectOnConflict, RedirectProfile, RedirectUnmatched};
use crate::security::audit::audit_log;
use crate::windows::safety::ensure_safe_target;

use super::super::fs_utils::sha256_file;
use super::super::matcher::{match_file, match_path};
use super::audit::{audit_if, audit_params_redirect};
use super::conflict::{ConflictResolution, resolve_conflict, resolve_conflict_with_source};
use super::ops::{copy_file, move_file};
use super::path::compare_key;
use super::template::{age_matches, resolve_dest_dir};
use super::types::{RedirectOptions, RedirectResult};

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

    if src_path.is_dir() {
        let Some(file_name) = file_name else {
            return;
        };
        if let Some(rule) = match_file(&file_name, &profile.rules) {
            let dest_dir = resolve_dest_dir(source_abs, &rule.dest, src_path);
            out.push(RedirectResult {
                action: "skip".to_string(),
                src: src_path.to_string_lossy().to_string(),
                dst: dest_dir.to_string_lossy().to_string(),
                rule: rule.name.clone(),
                result: "skipped".to_string(),
                reason: "directory_unsupported".to_string(),
            });
            audit_if(
                opts,
                "redirect_skip",
                &src_path.to_string_lossy(),
                "cli",
                audit_params_redirect(tx, dest_dir.to_string_lossy().as_ref(), opts.copy),
                "success",
                "directory_unsupported",
            );
        }
        return;
    }
    if !src_path.is_file() {
        return;
    }

    let Some(file_name) = file_name else {
        return;
    };

    let (rule_name, dest_dir) = if let Some(rule) = match_path(src_path, &profile.rules) {
        (
            rule.name.clone(),
            resolve_dest_dir(source_abs, &rule.dest, src_path),
        )
    } else {
        match &profile.unmatched {
            RedirectUnmatched::Skip => {
                out.push(RedirectResult {
                    action: "skip".to_string(),
                    src: src_path.to_string_lossy().to_string(),
                    dst: "".to_string(),
                    rule: "".to_string(),
                    result: "skipped".to_string(),
                    reason: "unmatched".to_string(),
                });
                audit_if(
                    opts,
                    "redirect_skip",
                    &src_path.to_string_lossy(),
                    "cli",
                    audit_params_redirect(tx, "", opts.copy),
                    "success",
                    "unmatched",
                );
                return;
            }
            RedirectUnmatched::Archive { age_expr, dest } => {
                if !age_matches(src_path, age_expr) {
                    out.push(RedirectResult {
                        action: "skip".to_string(),
                        src: src_path.to_string_lossy().to_string(),
                        dst: "".to_string(),
                        rule: "".to_string(),
                        result: "skipped".to_string(),
                        reason: "unmatched".to_string(),
                    });
                    audit_if(
                        opts,
                        "redirect_skip",
                        &src_path.to_string_lossy(),
                        "cli",
                        audit_params_redirect(tx, "", opts.copy),
                        "success",
                        "unmatched",
                    );
                    return;
                }
                (
                    "(unmatched)".to_string(),
                    resolve_dest_dir(source_abs, dest, src_path),
                )
            }
        }
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
    if opts.dry_run {
        if dest.exists() && matches!(profile.on_conflict, RedirectOnConflict::HashDedup) {
            let src_hash = match sha256_file(src_path) {
                Ok(v) => Some(v),
                Err(e) => {
                    warn_reason = Some(format!("hash_dedup_fallback_src_hash_failed:{e}"));
                    None
                }
            };
            let dst_hash = match sha256_file(&dest) {
                Ok(v) => Some(v),
                Err(e) => {
                    warn_reason = Some(format!("hash_dedup_fallback_dst_hash_failed:{e}"));
                    None
                }
            };
            if let (Some(a), Some(b)) = (src_hash, dst_hash) {
                if a == b {
                    if opts.copy {
                        out.push(RedirectResult {
                            action: "skip".to_string(),
                            src: src_path.to_string_lossy().to_string(),
                            dst: dest.to_string_lossy().to_string(),
                            rule: rule_name.clone(),
                            result: "dry_run".to_string(),
                            reason: "hash_dedup_same".to_string(),
                        });
                        audit_log(
                            "redirect_skip",
                            &src_path.to_string_lossy(),
                            "cli",
                            audit_params_redirect(tx, dest.to_string_lossy().as_ref(), opts.copy),
                            "success",
                            "dry_run",
                        );
                    } else {
                        out.push(RedirectResult {
                            action: "dedup".to_string(),
                            src: src_path.to_string_lossy().to_string(),
                            dst: dest.to_string_lossy().to_string(),
                            rule: rule_name.clone(),
                            result: "dry_run".to_string(),
                            reason: "hash_dedup_same_deleted_src".to_string(),
                        });
                        audit_log(
                            "redirect_dedup",
                            &src_path.to_string_lossy(),
                            "cli",
                            audit_params_redirect(tx, dest.to_string_lossy().as_ref(), opts.copy),
                            "success",
                            "dry_run",
                        );
                    }
                    return;
                }
            }
        }
    } else {
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
