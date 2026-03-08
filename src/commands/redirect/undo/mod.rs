mod executor;
mod history;
mod plan;
mod report;

use crate::config::RedirectOnConflict;

pub(crate) use report::UndoResult;

pub(crate) fn run_undo(
    tx: &str,
    on_conflict: &RedirectOnConflict,
    dry_run: bool,
    yes: bool,
) -> Vec<UndoResult> {
    plan::run_undo(tx, on_conflict, dry_run, yes)
}
