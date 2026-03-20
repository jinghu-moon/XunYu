// batch_rename/undo.rs

use std::path::Path;

use crate::output::{CliError, CliResult};
use serde::{Deserialize, Serialize};

pub const UNDO_FILE: &str = ".xun-brn-undo.json";

#[derive(Serialize, Deserialize, Clone)]
pub struct UndoRecord {
    pub from: String,
    pub to: String,
}

/// One batch of renames stored in the history file.
#[derive(Serialize, Deserialize)]
pub struct UndoBatch {
    /// Unix timestamp (seconds) when this batch was recorded.
    pub ts: u64,
    pub ops: Vec<UndoRecord>,
}

/// Append a new undo batch to the history file (creates it if absent).
pub fn append_undo(dir: &Path, records: &[UndoRecord]) -> CliResult {
    let mut history = read_undo_history(dir).unwrap_or_default();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    history.push(UndoBatch { ts, ops: records.to_vec() });
    write_history(dir, &history)
}

/// Read the full undo history from disk. Returns empty vec if file absent.
pub fn read_undo_history(dir: &Path) -> CliResult<Vec<UndoBatch>> {
    let path = dir.join(UNDO_FILE);
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| CliError::new(1, format!("Cannot read undo file: {}", e)))?;

    // Try new format (array of batches) first
    if let Ok(batches) = serde_json::from_str::<Vec<UndoBatch>>(&data) {
        return Ok(batches);
    }
    // Fall back to legacy format (flat array of UndoRecord)
    let records: Vec<UndoRecord> = serde_json::from_str(&data)
        .map_err(|e| CliError::new(1, format!("Invalid undo file: {}", e)))?;
    Ok(vec![UndoBatch { ts: 0, ops: records }])
}

fn write_history(dir: &Path, history: &[UndoBatch]) -> CliResult {
    let path = dir.join(UNDO_FILE);
    let json = serde_json::to_string_pretty(history)
        .map_err(|e| CliError::new(1, format!("Failed to serialize undo history: {}", e)))?;
    std::fs::write(&path, json)
        .map_err(|e| CliError::new(1, format!("Failed to write {}: {}", path.display(), e)))?;
    Ok(())
}

/// 将 undo 记录写入指定目录下的 undo 文件（覆盖写，旧格式兼容）
pub fn write_undo(dir: &Path, records: &[UndoRecord]) -> CliResult {
    let path = dir.join(UNDO_FILE);
    let json = serde_json::to_string_pretty(records)
        .map_err(|e| CliError::new(1, format!("Failed to serialize undo: {}", e)))?;
    std::fs::write(&path, json)
        .map_err(|e| CliError::new(1, format!("Failed to write {}: {}", path.display(), e)))?;
    Ok(())
}

/// Undo the last `steps` batches (default 1). Silently caps at history length.
pub fn run_undo_steps(dir: &str, steps: usize) -> CliResult {
    let undo_dir = std::path::Path::new(dir);
    let mut history = read_undo_history(undo_dir)?;

    if history.is_empty() {
        ui_println!("Undo file is empty. Nothing to undo.");
        return Ok(());
    }

    let n = steps.min(history.len());
    // Reverse the last `n` batches (most recent first)
    let to_undo: Vec<&UndoBatch> = history.iter().rev().take(n).collect();

    let mut success = 0usize;
    let mut errors = 0usize;

    for batch in &to_undo {
        for r in &batch.ops {
            // Undo: rename to → from
            match std::fs::rename(&r.to, &r.from) {
                Ok(()) => {
                    success += 1;
                    ui_println!("  OK  {} -> {}", r.to, r.from);
                }
                Err(e) => {
                    errors += 1;
                    ui_println!("  ERR {} -> {}: {}", r.to, r.from, e);
                }
            }
        }
    }

    ui_println!("\n{} restored, {} failed.", success, errors);

    // Remove the undone batches from history
    if errors == 0 {
        let keep = history.len() - n;
        history.truncate(keep);
        if history.is_empty() {
            let _ = std::fs::remove_file(undo_dir.join(UNDO_FILE));
        } else {
            write_history(undo_dir, &history)?;
        }
    }

    Ok(())
}

/// 从指定目录下读取 undo 文件并执行回滚（单步，向后兼容）
pub fn run_undo(dir: &str) -> CliResult {
    let undo_dir = std::path::Path::new(dir);
    let undo_path = undo_dir.join(UNDO_FILE);
    if !undo_path.exists() {
        return Err(CliError::new(
            1,
            format!("Undo file '{}' not found. Nothing to undo.", undo_path.display()),
        ));
    }
    // Read legacy flat format or new batched format and undo last batch
    let data = std::fs::read_to_string(&undo_path)
        .map_err(|e| CliError::new(1, format!("Cannot read undo file: {}", e)))?;

    // Try legacy flat format first (array of UndoRecord)
    let records: Vec<UndoRecord> = if let Ok(recs) = serde_json::from_str::<Vec<UndoRecord>>(&data) {
        recs
    } else {
        // New batched format — take last batch ops
        let batches: Vec<UndoBatch> = serde_json::from_str(&data)
            .map_err(|e| CliError::new(1, format!("Invalid undo file: {}", e)))?;
        batches.into_iter().last().map(|b| b.ops).unwrap_or_default()
    };

    if records.is_empty() {
        ui_println!("Undo file is empty. Nothing to undo.");
        return Ok(());
    }

    ui_println!("Undoing {} rename(s):\n", records.len());
    let mut success = 0usize;
    let mut errors = 0usize;

    for r in &records {
        // Legacy run_undo: records store (current_name → target_name), execute as-is
        match std::fs::rename(&r.from, &r.to) {
            Ok(()) => {
                success += 1;
                ui_println!("  OK  {} -> {}", r.to, r.from);
            }
            Err(e) => {
                errors += 1;
                ui_println!("  ERR {} -> {}: {}", r.to, r.from, e);
            }
        }
    }

    ui_println!("\n{} restored, {} failed.", success, errors);

    if errors == 0 {
        let _ = std::fs::remove_file(&undo_path);
        ui_println!("Undo file removed.");
    }

    Ok(())
}
