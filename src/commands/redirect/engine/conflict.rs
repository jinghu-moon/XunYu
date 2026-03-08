use crate::config::RedirectOnConflict;
use crate::output::{can_interact, emit_warning};
use crate::windows::trash::trash_file;

use super::super::fs_utils::{sha256_file, unique_dest_path, unique_dest_path_with_timestamp};
use super::RedirectOptions;

use std::path::{Path, PathBuf};

pub(crate) enum ConflictResolution {
    Proceed,
    Skip(String),
    DedupDeleted(String),
}

pub(crate) fn resolve_conflict(
    dest: &mut PathBuf,
    on_conflict: &RedirectOnConflict,
    opts: &RedirectOptions,
    warn_reason: &mut Option<String>,
) -> Option<String> {
    if !dest.exists() {
        return None;
    }
    if matches!(on_conflict, RedirectOnConflict::Skip) {
        return Some("exists".to_string());
    }
    if matches!(on_conflict, RedirectOnConflict::Trash) {
        if opts.dry_run {
            return None;
        }
        match trash_file(dest) {
            Ok(warn) => {
                if warn.is_some() {
                    *warn_reason = warn;
                }
                return None;
            }
            Err(e) => return Some(e),
        }
    }
    if matches!(on_conflict, RedirectOnConflict::RenameExisting) {
        let bak = unique_dest_path(dest);
        if opts.dry_run {
            return None;
        }
        if let Err(e) = std::fs::rename(&*dest, &bak) {
            return Some(format!("rename_existing_failed:{e}"));
        }
        return None;
    }
    if matches!(on_conflict, RedirectOnConflict::RenameNew) {
        *dest = unique_dest_path(dest);
        return None;
    }
    if matches!(on_conflict, RedirectOnConflict::RenameDate) {
        *dest = unique_dest_path_with_timestamp(dest);
        return None;
    }
    if matches!(on_conflict, RedirectOnConflict::HashDedup) {
        *dest = unique_dest_path(dest);
        return None;
    }
    if matches!(on_conflict, RedirectOnConflict::Overwrite) {
        if !can_interact() && !opts.yes {
            return Some("overwrite_requires_yes".to_string());
        }
        if let Err(e) = std::fs::remove_file(dest) {
            *warn_reason = Some(format!("overwrite_delete_failed:{e}"));
        }
        return None;
    }
    if matches!(on_conflict, RedirectOnConflict::Ask) {
        return Some("conflict_requires_user_input".to_string());
    }
    Some("unsupported_conflict".to_string())
}

pub(crate) fn resolve_conflict_with_source(
    tx: &str,
    src_path: &Path,
    dest: &mut PathBuf,
    on_conflict: &RedirectOnConflict,
    opts: &RedirectOptions,
    warn_reason: &mut Option<String>,
) -> ConflictResolution {
    if !dest.exists() {
        return ConflictResolution::Proceed;
    }
    if !matches!(on_conflict, RedirectOnConflict::HashDedup) {
        return ConflictResolution::Proceed;
    }

    let src_hash = match sha256_file(src_path) {
        Ok(v) => v,
        Err(e) => {
            *warn_reason = Some(format!("hash_dedup_fallback_src_hash_failed:{e}"));
            return ConflictResolution::Proceed;
        }
    };
    let dst_hash = match sha256_file(dest) {
        Ok(v) => v,
        Err(e) => {
            *warn_reason = Some(format!("hash_dedup_fallback_dst_hash_failed:{e}"));
            return ConflictResolution::Proceed;
        }
    };

    if src_hash != dst_hash {
        return ConflictResolution::Proceed;
    }

    if opts.copy {
        return ConflictResolution::Skip("hash_dedup_same".to_string());
    }

    match std::fs::remove_file(src_path) {
        Ok(_) => ConflictResolution::DedupDeleted("hash_dedup_same_deleted_src".to_string()),
        Err(e) => {
            let ctx = format!("Context: tx={tx} src={}", src_path.display());
            let detail = format!("Details: {e}");
            emit_warning(
                "hash_dedup: failed to delete source.",
                &[ctx.as_str(), detail.as_str()],
            );
            ConflictResolution::Skip(format!("hash_dedup_same_delete_failed:{e}"))
        }
    }
}
