//! Batch Rename 业务逻辑服务
//!
//! 封装批量重命名核心逻辑，桥接到 `crate::batch_rename::*` 底层模块。
//! 仅在 `batch_rename` feature 启用时编译。

use std::path::Path;

use crate::xun_core::error::XunError;
use crate::xun_core::operation::{Change, Operation, OperationResult, Preview, RiskLevel};
use crate::xun_core::value::Value;

// ============================================================
// BrnPreviewOp — 预览重命名操作
// ============================================================

/// 批量重命名预览操作。
pub struct BrnPreviewOp {
    path: String,
    steps: Vec<crate::batch_rename::compute::RenameMode>,
    ext: Vec<String>,
    recursive: bool,
    depth: Option<usize>,
    filter: Option<String>,
    exclude: Option<String>,
    preview: Preview,
}

impl BrnPreviewOp {
    pub fn new(
        path: impl Into<String>,
        steps: Vec<crate::batch_rename::compute::RenameMode>,
        ext: Vec<String>,
        recursive: bool,
        depth: Option<usize>,
        filter: Option<String>,
        exclude: Option<String>,
    ) -> Self {
        let path = path.into();
        let preview = Preview::new(format!("Preview batch rename in '{}'", path))
            .add_change(Change::new("collect", &path))
            .with_risk_level(RiskLevel::Low);
        Self {
            path,
            steps,
            ext,
            recursive,
            depth,
            filter,
            exclude,
            preview,
        }
    }
}

impl Operation for BrnPreviewOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        use crate::batch_rename::collect::{collect_files, collect_files_depth};
        use crate::batch_rename::compute::{compute_ops, compute_ops_chain};

        let depth = self
            .depth
            .map(|d| if d == 0 { 1 } else { d })
            .or(if self.recursive { None } else { Some(1) });

        let files = if self.filter.is_some() || self.exclude.is_some() || self.depth.is_some() {
            collect_files_depth(
                &self.path,
                &self.ext,
                depth,
                self.filter.as_deref(),
                self.exclude.as_deref(),
            )
            ?
        } else {
            collect_files(&self.path, &self.ext, self.recursive)?
        };

        if files.is_empty() {
            return Ok(OperationResult::new().with_changes_applied(0));
        }

        let ops = if self.steps.len() == 1 {
            compute_ops(&files, &self.steps[0])?
        } else {
            compute_ops_chain(&files, &self.steps)?
        };

        let effective: Vec<_> = ops.into_iter().filter(|o| o.from != o.to).collect();
        Ok(OperationResult::new().with_changes_applied(effective.len() as u32))
    }
}

// ============================================================
// BrnApplyOp — 执行重命名操作
// ============================================================

/// 批量重命名执行操作。
pub struct BrnApplyOp {
    path: String,
    steps: Vec<crate::batch_rename::compute::RenameMode>,
    ext: Vec<String>,
    recursive: bool,
    depth: Option<usize>,
    filter: Option<String>,
    exclude: Option<String>,
    preview: Preview,
}

impl BrnApplyOp {
    pub fn new(
        path: impl Into<String>,
        steps: Vec<crate::batch_rename::compute::RenameMode>,
        ext: Vec<String>,
        recursive: bool,
        depth: Option<usize>,
        filter: Option<String>,
        exclude: Option<String>,
    ) -> Self {
        let path = path.into();
        let preview = Preview::new(format!("Apply batch rename in '{}'", path))
            .add_change(Change::new("rename", &path))
            .with_risk_level(RiskLevel::Medium);
        Self {
            path,
            steps,
            ext,
            recursive,
            depth,
            filter,
            exclude,
            preview,
        }
    }
}

impl Operation for BrnApplyOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        use crate::batch_rename::collect::{collect_files, collect_files_depth};
        use crate::batch_rename::compute::{compute_ops, compute_ops_chain};
        use crate::batch_rename::undo::{UndoRecord, push_undo};

        let depth = self
            .depth
            .map(|d| if d == 0 { 1 } else { d })
            .or(if self.recursive { None } else { Some(1) });

        let files = if self.filter.is_some() || self.exclude.is_some() || self.depth.is_some() {
            collect_files_depth(
                &self.path,
                &self.ext,
                depth,
                self.filter.as_deref(),
                self.exclude.as_deref(),
            )
            ?
        } else {
            collect_files(&self.path, &self.ext, self.recursive)?
        };

        if files.is_empty() {
            return Ok(OperationResult::new().with_changes_applied(0));
        }

        let ops = if self.steps.len() == 1 {
            compute_ops(&files, &self.steps[0])?
        } else {
            compute_ops_chain(&files, &self.steps)?
        };

        let effective: Vec<_> = ops.into_iter().filter(|o| o.from != o.to).collect();
        if effective.is_empty() {
            return Ok(OperationResult::new().with_changes_applied(0));
        }

        let scan_root = Path::new(&self.path);
        let mut records: Vec<UndoRecord> = Vec::new();
        let mut success = 0usize;

        for op in &effective {
            if std::fs::rename(&op.from, &op.to).is_ok() {
                success += 1;
                records.push(UndoRecord {
                    from: op.to.to_string_lossy().into_owned(),
                    to: op.from.to_string_lossy().into_owned(),
                });
            }
        }

        if !records.is_empty() {
            push_undo(scan_root, &records)?;
        }

        Ok(OperationResult::new().with_changes_applied(success as u32))
    }
}

// ============================================================
// 查询函数
// ============================================================

/// 收集目录中的文件列表（预览用）。
pub fn collect_files_preview(
    path: &str,
    ext: &[String],
    recursive: bool,
) -> Result<Value, XunError> {
    use crate::batch_rename::collect::collect_files;

    let files = collect_files(path, ext, recursive)?;
    let names: Vec<String> = files
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();
    Ok(Value::List(
        names.into_iter().map(Value::String).collect(),
    ))
}
