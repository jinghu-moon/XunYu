use crate::bookmark_state::Store;
use crate::bookmark::storage::db_path;
use crate::bookmark::undo::{run_redo_steps, run_undo_steps};
use crate::cli::{RedoCmd, UndoCmd};
use crate::output::{CliError, CliResult};
use crate::store::now_secs;

pub(crate) fn cmd_undo(args: UndoCmd) -> CliResult {
    let file = db_path();
    let mut store = Store::load_or_default(&file)
        .map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let applied = run_undo_steps(&file, &mut store, args.steps)?;
    store
        .save_exact(&file, now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to save restored store: {e}")))?;
    ui_println!("Undo applied (steps={}).", applied);
    Ok(())
}

pub(crate) fn cmd_redo(args: RedoCmd) -> CliResult {
    let file = db_path();
    let mut store = Store::load_or_default(&file)
        .map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let applied = run_redo_steps(&file, &mut store, args.steps)?;
    store
        .save_exact(&file, now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to save restored store: {e}")))?;
    ui_println!("Redo applied (steps={}).", applied);
    Ok(())
}
