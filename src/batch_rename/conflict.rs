// batch_rename/conflict.rs
//
// Conflict detection before touching the filesystem.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::batch_rename::types::RenameOp;

pub(crate) fn detect_conflicts(ops: &[RenameOp]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen_targets: HashSet<&PathBuf> = HashSet::new();
    let sources: HashSet<&PathBuf> = ops.iter().map(|o| &o.from).collect();

    for op in ops {
        if op.to.exists() && !sources.contains(&op.to) {
            errors.push(format!(
                "Target already exists: {} (would overwrite)",
                op.to.display()
            ));
        }
        if !seen_targets.insert(&op.to) {
            errors.push(format!(
                "Duplicate target: {} (two files map to same name)",
                op.to.display()
            ));
        }
    }

    errors
}
