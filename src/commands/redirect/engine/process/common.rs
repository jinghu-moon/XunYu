use std::path::{Path, PathBuf};

use crate::config::{RedirectProfile, RedirectUnmatched};

use super::super::super::matcher::{match_file, match_path};
use super::super::template::{age_matches, resolve_dest_dir};
use super::{RedirectOptions, RedirectResult, audit_if, audit_params_redirect};

pub(super) fn early_handle_non_file(
    tx: &str,
    source_abs: &Path,
    profile: &RedirectProfile,
    opts: &RedirectOptions,
    src_path: &Path,
    file_name: Option<&str>,
    out: &mut Vec<RedirectResult>,
) -> bool {
    if src_path.is_dir() {
        let Some(file_name) = file_name else {
            return true;
        };
        if let Some(rule) = match_file(file_name, &profile.rules) {
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
        return true;
    }

    !src_path.is_file() || file_name.is_none()
}

pub(super) fn resolve_rule_and_dest(
    tx: &str,
    source_abs: &Path,
    profile: &RedirectProfile,
    opts: &RedirectOptions,
    src_path: &Path,
    out: &mut Vec<RedirectResult>,
) -> Option<(String, PathBuf)> {
    if let Some(rule) = match_path(src_path, &profile.rules) {
        return Some((
            rule.name.clone(),
            resolve_dest_dir(source_abs, &rule.dest, src_path),
        ));
    }

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
            None
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
                None
            } else {
                Some((
                    "(unmatched)".to_string(),
                    resolve_dest_dir(source_abs, dest, src_path),
                ))
            }
        }
    }
}
