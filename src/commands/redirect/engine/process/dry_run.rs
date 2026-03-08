use std::path::Path;

use crate::config::RedirectOnConflict;
use crate::security::audit::audit_log;

use super::super::super::fs_utils::sha256_file;
use super::super::audit::audit_params_redirect;
use super::super::types::{RedirectOptions, RedirectResult};

pub(super) fn handle_hash_dedup(
    tx: &str,
    on_conflict: &RedirectOnConflict,
    opts: &RedirectOptions,
    src_path: &Path,
    dest: &Path,
    rule_name: &str,
    warn_reason: &mut Option<String>,
    out: &mut Vec<RedirectResult>,
) -> bool {
    if !opts.dry_run || !dest.exists() || !matches!(on_conflict, RedirectOnConflict::HashDedup) {
        return false;
    }

    let src_hash = match sha256_file(src_path) {
        Ok(v) => Some(v),
        Err(e) => {
            *warn_reason = Some(format!("hash_dedup_fallback_src_hash_failed:{e}"));
            None
        }
    };
    let dst_hash = match sha256_file(dest) {
        Ok(v) => Some(v),
        Err(e) => {
            *warn_reason = Some(format!("hash_dedup_fallback_dst_hash_failed:{e}"));
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
                    rule: rule_name.to_string(),
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
                    rule: rule_name.to_string(),
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
            return true;
        }
    }

    false
}
