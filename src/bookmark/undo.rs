use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::bookmark_state::{Bookmark, Store};
use crate::output::{CliError, CliResult};

const UNDO_LOG_NAME: &str = ".xun.bookmark.undo.log";
const REDO_LOG_NAME: &str = ".xun.bookmark.redo.log";
const MAX_HISTORY: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BookmarkUndoBatch {
    pub(crate) ts: u64,
    pub(crate) action: String,
    pub(crate) before_schema_version: u32,
    pub(crate) after_schema_version: u32,
    pub(crate) ops: Vec<BookmarkUndoOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum BookmarkUndoOp {
    Create {
        after_index: usize,
        after: Bookmark,
    },
    Delete {
        before_index: usize,
        before: Bookmark,
    },
    Update {
        before: Bookmark,
        after: Bookmark,
    },
}

#[derive(Debug, Deserialize)]
struct LegacySnapshotEntry {
    #[allow(dead_code)]
    ts: u64,
    #[allow(dead_code)]
    action: String,
    #[allow(dead_code)]
    snapshot: serde_json::Value,
}

pub(crate) fn record_undo_batch(
    db_path: &Path,
    action: &str,
    before: &Store,
    after: &Store,
) -> CliResult<bool> {
    let Some(batch) = build_undo_batch(action, before, after)? else {
        return Ok(false);
    };
    append_undo_batch(db_path, &batch)?;
    Ok(true)
}

pub(crate) fn run_undo_steps(db_path: &Path, store: &mut Store, steps: usize) -> CliResult<usize> {
    if steps == 0 {
        return Err(CliError::new(2, "Undo steps must be greater than 0."));
    }
    reset_legacy_logs_if_needed(db_path)?;
    let undo_path = undo_log_path(db_path);
    let redo_path = redo_log_path(db_path);
    let mut undo_batches = read_batches(&undo_path)?;
    if undo_batches.is_empty() {
        return Err(CliError::new(2, "Nothing to undo."));
    }

    let steps = steps.min(undo_batches.len());
    let to_process = undo_batches.split_off(undo_batches.len() - steps);
    let mut moved = Vec::with_capacity(steps);
    for batch in to_process.iter().rev() {
        store
            .apply_undo_batch_inverse(batch)
            .map_err(|e| CliError::new(1, format!("Failed to apply undo batch: {e}")))?;
        moved.push(batch.clone());
    }

    write_batches(&undo_path, &undo_batches)?;
    append_batches(&redo_path, &moved)?;
    trim_log(&redo_path)?;
    Ok(steps)
}

pub(crate) fn run_redo_steps(db_path: &Path, store: &mut Store, steps: usize) -> CliResult<usize> {
    if steps == 0 {
        return Err(CliError::new(2, "Redo steps must be greater than 0."));
    }
    reset_legacy_logs_if_needed(db_path)?;
    let undo_path = undo_log_path(db_path);
    let redo_path = redo_log_path(db_path);
    let mut redo_batches = read_batches(&redo_path)?;
    if redo_batches.is_empty() {
        return Err(CliError::new(2, "Nothing to redo."));
    }

    let steps = steps.min(redo_batches.len());
    let to_process = redo_batches.split_off(redo_batches.len() - steps);
    let mut moved = Vec::with_capacity(steps);
    for batch in to_process.iter().rev() {
        store
            .apply_undo_batch_forward(batch)
            .map_err(|e| CliError::new(1, format!("Failed to apply redo batch: {e}")))?;
        moved.push(batch.clone());
    }

    write_batches(&redo_path, &redo_batches)?;
    append_batches(&undo_path, &moved)?;
    trim_log(&undo_path)?;
    Ok(steps)
}

fn build_undo_batch(action: &str, before: &Store, after: &Store) -> CliResult<Option<BookmarkUndoBatch>> {
    let before_map: HashMap<&str, (usize, &Bookmark)> = before
        .bookmarks
        .iter()
        .enumerate()
        .map(|(index, bookmark)| (bookmark.id.as_str(), (index, bookmark)))
        .collect();
    let after_map: HashMap<&str, (usize, &Bookmark)> = after
        .bookmarks
        .iter()
        .enumerate()
        .map(|(index, bookmark)| (bookmark.id.as_str(), (index, bookmark)))
        .collect();

    let mut deletes = Vec::new();
    let mut updates = Vec::new();
    let mut creates = Vec::new();

    for (id, (before_index, before_bookmark)) in &before_map {
        match after_map.get(id) {
            Some((_after_index, after_bookmark)) if *before_bookmark != *after_bookmark => {
                updates.push(BookmarkUndoOp::Update {
                    before: (*before_bookmark).clone(),
                    after: (*after_bookmark).clone(),
                });
            }
            Some(_) => {}
            None => deletes.push(BookmarkUndoOp::Delete {
                before_index: *before_index,
                before: (*before_bookmark).clone(),
            }),
        }
    }

    for (id, (after_index, after_bookmark)) in &after_map {
        if !before_map.contains_key(id) {
            creates.push(BookmarkUndoOp::Create {
                after_index: *after_index,
                after: (*after_bookmark).clone(),
            });
        }
    }

    deletes.sort_by_key(|op| match op {
        BookmarkUndoOp::Delete { before_index, .. } => *before_index,
        _ => usize::MAX,
    });
    creates.sort_by_key(|op| match op {
        BookmarkUndoOp::Create { after_index, .. } => *after_index,
        _ => usize::MAX,
    });
    updates.sort_by_key(|op| match op {
        BookmarkUndoOp::Update { before, .. } => before.id.clone(),
        _ => String::new(),
    });

    let mut ops = Vec::with_capacity(deletes.len() + updates.len() + creates.len());
    ops.extend(deletes);
    ops.extend(updates);
    ops.extend(creates);

    if ops.is_empty() && before.schema_version == after.schema_version {
        return Ok(None);
    }

    Ok(Some(BookmarkUndoBatch {
        ts: now_secs(),
        action: action.to_string(),
        before_schema_version: before.schema_version,
        after_schema_version: after.schema_version,
        ops,
    }))
}

fn append_undo_batch(db_path: &Path, batch: &BookmarkUndoBatch) -> CliResult {
    reset_legacy_logs_if_needed(db_path)?;
    let undo_path = undo_log_path(db_path);
    let redo_path = redo_log_path(db_path);
    append_batch(&undo_path, batch)?;
    let _ = fs::remove_file(redo_path);
    trim_log(&undo_path)
}

fn append_batch(path: &Path, batch: &BookmarkUndoBatch) -> CliResult {
    let line = serde_json::to_string(batch)
        .map_err(|e| CliError::new(1, format!("Failed to serialize undo batch: {e}")))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| CliError::new(1, format!("Failed to open undo log {}: {e}", path.display())))?;
    writeln!(&mut file, "{line}")
        .map_err(|e| CliError::new(1, format!("Failed to write undo log {}: {e}", path.display())))
}

fn append_batches(path: &Path, batches: &[BookmarkUndoBatch]) -> CliResult {
    if batches.is_empty() {
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| CliError::new(1, format!("Failed to open undo log {}: {e}", path.display())))?;
    for batch in batches {
        let line = serde_json::to_string(batch)
            .map_err(|e| CliError::new(1, format!("Failed to serialize undo batch: {e}")))?;
        writeln!(&mut file, "{line}")
            .map_err(|e| CliError::new(1, format!("Failed to write undo log {}: {e}", path.display())))?;
    }
    Ok(())
}

fn read_batches(path: &Path) -> CliResult<Vec<BookmarkUndoBatch>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .map_err(|e| CliError::new(1, format!("Failed to read undo log {}: {e}", path.display())))?;
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let batch: BookmarkUndoBatch = serde_json::from_str(line)
            .map_err(|e| CliError::new(1, format!("Invalid undo log entry: {e}")))?;
        out.push(batch);
    }
    Ok(out)
}

fn write_batches(path: &Path, batches: &[BookmarkUndoBatch]) -> CliResult {
    if batches.is_empty() {
        let _ = fs::remove_file(path);
        return Ok(());
    }
    let mut out = String::new();
    for batch in batches {
        let line = serde_json::to_string(batch)
            .map_err(|e| CliError::new(1, format!("Failed to serialize undo batch: {e}")))?;
        out.push_str(&line);
        out.push('\n');
    }
    fs::write(path, out)
        .map_err(|e| CliError::new(1, format!("Failed to rewrite undo log {}: {e}", path.display())))
}

fn trim_log(path: &Path) -> CliResult {
    let batches = read_batches(path)?;
    if batches.len() <= MAX_HISTORY {
        return Ok(());
    }
    let trimmed = &batches[batches.len() - MAX_HISTORY..];
    write_batches(path, trimmed)
}

fn reset_legacy_logs_if_needed(db_path: &Path) -> CliResult {
    reset_legacy_log_if_needed(&undo_log_path(db_path))?;
    reset_legacy_log_if_needed(&redo_log_path(db_path))
}

fn reset_legacy_log_if_needed(path: &Path) -> CliResult {
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(path)
        .map_err(|e| CliError::new(1, format!("Failed to read undo log {}: {e}", path.display())))?;
    let Some(first_line) = content.lines().map(str::trim).find(|line| !line.is_empty()) else {
        let _ = fs::remove_file(path);
        return Ok(());
    };

    if serde_json::from_str::<BookmarkUndoBatch>(first_line).is_ok() {
        return Ok(());
    }
    if serde_json::from_str::<LegacySnapshotEntry>(first_line).is_ok() {
        let _ = fs::remove_file(path);
        return Ok(());
    }
    Err(CliError::new(
        1,
        format!("Invalid undo log format: {}", path.display()),
    ))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn undo_log_path(db_path: &Path) -> PathBuf {
    if let Some(path) = std::env::var("_BM_UNDO_LOG_FILE")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return PathBuf::from(path);
    }
    let parent = db_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(UNDO_LOG_NAME)
}

fn redo_log_path(db_path: &Path) -> PathBuf {
    if let Some(path) = std::env::var("_BM_REDO_LOG_FILE")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return PathBuf::from(path);
    }
    let parent = db_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(REDO_LOG_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    fn sample_store(name: &str) -> Store {
        let mut store = Store::new();
        store.set(name, &format!("C:/work/{name}"), Path::new("C:/work"), None, 10).unwrap();
        store
    }

    #[test]
    fn build_batch_for_new_bookmark_uses_create_op() {
        let before = Store::new();
        let after = sample_store("home");
        let batch = build_undo_batch("set", &before, &after).unwrap().unwrap();
        assert!(matches!(
            batch.ops.as_slice(),
            [BookmarkUndoOp::Create { .. }]
        ));
    }

    #[test]
    fn record_undo_and_roundtrip_with_update() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".xun.bookmark.json");

        let before = sample_store("home");
        let mut after = before.clone();
        after.rename("home", "main").unwrap();

        record_undo_batch(&db, "rename", &before, &after).unwrap();

        let mut current = after.clone();
        assert_eq!(run_undo_steps(&db, &mut current, 1).unwrap(), 1);
        assert!(current.bookmarks.iter().any(|bookmark| bookmark.name.as_deref() == Some("home")));
        assert_eq!(run_redo_steps(&db, &mut current, 1).unwrap(), 1);
        assert!(current.bookmarks.iter().any(|bookmark| bookmark.name.as_deref() == Some("main")));
    }

    #[test]
    fn new_record_clears_redo_stack() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".xun.bookmark.json");

        let before = Store::new();
        let middle = sample_store("one");
        let mut after = middle.clone();
        after.rename("one", "two").unwrap();

        record_undo_batch(&db, "set", &before, &middle).unwrap();
        let mut current = middle.clone();
        run_undo_steps(&db, &mut current, 1).unwrap();
        record_undo_batch(&db, "rename", &current, &after).unwrap();

        let redo = read_batches(&redo_log_path(&db)).unwrap();
        assert!(redo.is_empty());
    }
}
