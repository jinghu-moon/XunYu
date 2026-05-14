//! Batch Rename 业务逻辑服务
//!
//! 封装批量重命名操作，支持 Operation trait 实现。
//!
//! 注意：batch_rename 模块使用旧的 CLI 类型（CliResult 等），
//! 此处提供桥接实现，后续可逐步迁移。

use crate::xun_core::error::XunError;
use crate::xun_core::operation::{Change, Operation, OperationResult, Preview, RiskLevel};
use crate::xun_core::value::Value;

// ============================================================
// RenameOperation — Operation trait 实现
// ============================================================

/// 批量重命名操作（实现 Operation trait）。
pub struct RenameOperation {
    directory: String,
    pattern: String,
    replacement: String,
    preview: Preview,
}

impl RenameOperation {
    pub fn new(
        directory: impl Into<String>,
        pattern: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Self {
        let directory = directory.into();
        let pattern = pattern.into();
        let replacement = replacement.into();
        let preview = Preview::new(format!(
            "Rename files in '{}' matching '{}' → '{}'",
            directory, pattern, replacement
        ))
        .add_change(Change::new("rename", format!("{} → {}", pattern, replacement)))
        .with_risk_level(RiskLevel::High);
        Self {
            directory,
            pattern,
            replacement,
            preview,
        }
    }
}

impl Operation for RenameOperation {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        // batch_rename 模块使用 RenameMode + 文件列表，
        // 此处提供桥接，后续集成时替换为实际调用
        Err(XunError::user(
            "batch rename not yet integrated with Operation Runtime",
        ))
    }

    fn rollback(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<(), XunError> {
        // batch_rename::undo::run_undo 可用于回滚
        Err(XunError::user("rollback not yet integrated"))
    }
}

/// 预览重命名结果。
///
/// TODO: 桥接到 `crate::batch_rename::compute::compute_ops`。
pub fn preview_rename(
    _directory: &str,
    _pattern: &str,
    _replacement: &str,
) -> Result<Value, XunError> {
    Ok(Value::List(vec![]))
}
