// batch_rename/undo.rs

use crate::output::{CliError, CliResult};
use serde::{Deserialize, Serialize};

pub(crate) const UNDO_FILE: &str = ".xun-brn-undo.json";

#[derive(Serialize, Deserialize)]
pub(crate) struct UndoRecord {
    pub from: String,
    pub to: String,
}

pub(crate) fn write_undo(records: &[UndoRecord]) -> CliResult {
    let json = serde_json::to_string_pretty(records)
        .map_err(|e| CliError::new(1, format!("Failed to serialize undo: {}", e)))?;
    std::fs::write(UNDO_FILE, json)
        .map_err(|e| CliError::new(1, format!("Failed to write {}: {}", UNDO_FILE, e)))?;
    Ok(())
}

pub(crate) fn run_undo() -> CliResult {
    let data = std::fs::read_to_string(UNDO_FILE).map_err(|_| {
        CliError::new(
            1,
            format!("Undo file '{}' not found. Nothing to undo.", UNDO_FILE),
        )
    })?;
    let records: Vec<UndoRecord> = serde_json::from_str(&data)
        .map_err(|e| CliError::new(1, format!("Invalid undo file: {}", e)))?;

    if records.is_empty() {
        ui_println!("Undo file is empty. Nothing to undo.");
        return Ok(());
    }

    ui_println!("Undoing {} rename(s):\n", records.len());
    let mut success = 0usize;
    let mut errors = 0usize;

    for r in &records {
        match std::fs::rename(&r.from, &r.to) {
            Ok(()) => {
                success += 1;
                ui_println!("  OK  {} -> {}", r.from, r.to);
            }
            Err(e) => {
                errors += 1;
                ui_println!("  ERR {} -> {}: {}", r.from, r.to, e);
            }
        }
    }

    ui_println!("\n{} restored, {} failed.", success, errors);

    if errors == 0 {
        let _ = std::fs::remove_file(UNDO_FILE);
        ui_println!("Undo file removed.");
    }

    Ok(())
}
