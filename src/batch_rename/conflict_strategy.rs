// batch_rename/conflict_strategy.rs
//
// Conflict resolution strategies for rename operations.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::batch_rename::types::RenameOp;
use crate::output::{CliError, CliResult};

#[derive(Clone, Copy, Debug)]
pub enum OnConflict {
    /// Abort the entire operation if any conflict is detected (default).
    Abort,
    /// Skip conflicting files; rename the rest.
    Skip,
    /// Append _1, _2, ... to make conflicting targets unique.
    RenameSeq,
}

/// Apply the conflict strategy to the given ops list.
/// `existing`: paths already on disk that could conflict.
/// Returns filtered/modified ops or an error.
pub fn apply_conflict_strategy(
    ops: Vec<RenameOp>,
    strategy: OnConflict,
    existing: &[PathBuf],
) -> CliResult<Vec<RenameOp>> {
    let existing_set: HashSet<PathBuf> = existing.iter().cloned().collect();

    match strategy {
        OnConflict::Abort => apply_abort(ops, &existing_set),
        OnConflict::Skip => Ok(apply_skip(ops, &existing_set)),
        OnConflict::RenameSeq => Ok(apply_rename_seq(ops, &existing_set)),
    }
}

// ─── Abort ────────────────────────────────────────────────────────────────────

fn apply_abort(ops: Vec<RenameOp>, existing: &HashSet<PathBuf>) -> CliResult<Vec<RenameOp>> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut conflicts: Vec<String> = Vec::new();

    for op in &ops {
        if existing.contains(&op.to) {
            conflicts.push(format!("Target already exists: {}", op.to.display()));
        }
        if !seen.insert(op.to.clone()) {
            conflicts.push(format!("Duplicate target: {}", op.to.display()));
        }
    }

    if conflicts.is_empty() {
        Ok(ops)
    } else {
        Err(CliError::with_details(1, "Conflicts detected.", &conflicts.iter().map(|s| s.as_str()).collect::<Vec<_>>()))
    }
}

// ─── Skip ─────────────────────────────────────────────────────────────────────

fn apply_skip(ops: Vec<RenameOp>, existing: &HashSet<PathBuf>) -> Vec<RenameOp> {
    // First pass: count how many ops share each target.
    let mut target_count: std::collections::HashMap<PathBuf, usize> = std::collections::HashMap::new();
    for op in &ops {
        *target_count.entry(op.to.clone()).or_insert(0) += 1;
    }
    // Second pass: keep only ops whose target is unique and not pre-existing.
    ops.into_iter()
        .filter(|op| {
            !existing.contains(&op.to) && target_count.get(&op.to).copied().unwrap_or(0) == 1
        })
        .collect()
}

// ─── RenameSeq ────────────────────────────────────────────────────────────────

fn apply_rename_seq(ops: Vec<RenameOp>, existing: &HashSet<PathBuf>) -> Vec<RenameOp> {
    let mut seen: HashSet<PathBuf> = existing.clone();
    ops.into_iter()
        .map(|op| {
            if seen.contains(&op.to) {
                let unique = make_unique(&op.to, &seen);
                seen.insert(unique.clone());
                RenameOp { from: op.from, to: unique }
            } else {
                seen.insert(op.to.clone());
                op
            }
        })
        .collect()
}

fn make_unique(path: &Path, seen: &HashSet<PathBuf>) -> PathBuf {
    let parent = path.parent().unwrap_or(std::path::Path::new(""));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    let mut i = 1usize;
    loop {
        let candidate = parent.join(format!("{}_{}{}", stem, i, ext));
        if !seen.contains(&candidate) {
            return candidate;
        }
        i += 1;
    }
}
