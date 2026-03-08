use crate::config::RedirectOnConflict;

use super::executor::{undo_copy_one, undo_move_one};
use super::history::{load_audit_entries_for_tx, parse_dst_copy_from_params};
use super::report::UndoResult;

pub(super) fn run_undo(
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
