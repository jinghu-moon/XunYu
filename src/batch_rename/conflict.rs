// batch_rename/conflict.rs
//
// Conflict detection before touching the filesystem.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::batch_rename::types::RenameOp;

/// 检测冲突。
/// - `check_existing`：是否检查目标文件已存在（apply 模式需要，dry-run 可跳过以减少 stat 调用）
pub(crate) fn detect_conflicts(ops: &[RenameOp], check_existing: bool) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen_targets: HashSet<&PathBuf> = HashSet::new();
    let sources: HashSet<&PathBuf> = ops.iter().map(|o| &o.from).collect();

    for op in ops {
        if check_existing && op.to.exists() && !sources.contains(&op.to) {
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
