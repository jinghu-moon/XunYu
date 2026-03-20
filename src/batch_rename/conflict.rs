// batch_rename/conflict.rs
//
// Conflict detection before touching the filesystem.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::batch_rename::types::RenameOp;

/// 冲突类型。
#[derive(Debug)]
pub enum ConflictKind {
    /// 目标文件在磁盘上已存在且不在本次重命名源集合中（会被覆盖）。
    WouldOverwrite,
    /// 两个或更多文件重命名后产生同一目标名称。
    DuplicateTarget,
}

/// 单条冲突信息。
#[derive(Debug)]
pub struct ConflictInfo {
    pub kind: ConflictKind,
    /// 涉及的源文件（DuplicateTarget 时包含所有发生碰撞的源文件）。
    pub sources: Vec<PathBuf>,
    /// 发生冲突的目标路径。
    pub target: PathBuf,
}

impl ConflictInfo {
    /// 单行摘要（兼容旧 Vec<String> 展示逻辑）。
    pub fn summary(&self) -> String {
        match self.kind {
            ConflictKind::WouldOverwrite => format!(
                "[overwrite] {} → {} (target already exists)",
                self.sources[0].file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                self.target.display(),
            ),
            ConflictKind::DuplicateTarget => {
                let srcs: Vec<&str> = self.sources
                    .iter()
                    .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
                    .collect();
                format!(
                    "[duplicate] {} file(s) → {} (same target name)",
                    srcs.len(),
                    self.target.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                )
            }
        }
    }
}

/// 检测冲突，返回结构化 ConflictInfo 列表。
/// - `check_existing`：是否检查目标文件已存在（apply 模式需要，dry-run 可跳过以减少 stat 调用）
pub fn detect_conflicts(ops: &[RenameOp], check_existing: bool) -> Vec<ConflictInfo> {
    let mut conflicts: Vec<ConflictInfo> = Vec::new();
    let sources: std::collections::HashSet<&PathBuf> = ops.iter().map(|o| &o.from).collect();

    // Check for overwrite conflicts
    if check_existing {
        for op in ops {
            if op.to.exists() && !sources.contains(&op.to) {
                conflicts.push(ConflictInfo {
                    kind: ConflictKind::WouldOverwrite,
                    sources: vec![op.from.clone()],
                    target: op.to.clone(),
                });
            }
        }
    }

    // Check for duplicate target conflicts
    // Group ops by target path; any group with >1 source is a conflict.
    let mut target_map: HashMap<&PathBuf, Vec<&PathBuf>> = HashMap::new();
    for op in ops {
        target_map.entry(&op.to).or_default().push(&op.from);
    }
    for (target, sources) in &target_map {
        if sources.len() > 1 {
            conflicts.push(ConflictInfo {
                kind: ConflictKind::DuplicateTarget,
                sources: sources.iter().map(|p| (*p).clone()).collect(),
                target: (*target).clone(),
            });
        }
    }

    conflicts
}
