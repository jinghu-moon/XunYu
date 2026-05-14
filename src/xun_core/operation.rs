//! Operation — 危险操作统一协议
//!
//! 所有危险操作（删除、重命名、移动等）实现 Operation trait，
//! 通过 run_operation() 统一调度：preview → confirm → execute。

use std::time::Instant;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;

/// 风险等级。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Type)]
pub enum RiskLevel {
    /// 低风险：可轻松撤销
    Low,
    /// 中等风险：需要确认
    #[default]
    Medium,
    /// 高风险：不可逆操作
    High,
    /// 关键风险：影响系统/数据完整性
    Critical,
}

/// 变更描述。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Change {
    action: String,
    target: String,
}

impl Change {
    pub fn new(action: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            target: target.into(),
        }
    }

    pub fn action(&self) -> &str {
        &self.action
    }

    pub fn target(&self) -> &str {
        &self.target
    }
}

/// 操作预览：描述将要执行的操作及其影响。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Preview {
    description: String,
    changes: Vec<Change>,
    #[serde(default)]
    risk_level: RiskLevel,
}

impl Preview {
    /// 创建新的预览。
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            changes: Vec::new(),
            risk_level: RiskLevel::default(),
        }
    }

    /// 添加变更项。
    pub fn add_change(mut self, change: Change) -> Self {
        self.changes.push(change);
        self
    }

    /// 设置风险等级。
    pub fn with_risk_level(mut self, level: RiskLevel) -> Self {
        self.risk_level = level;
        self
    }

    /// 获取描述。
    pub fn description(&self) -> &str {
        &self.description
    }

    /// 获取变更列表。
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// 获取风险等级。
    pub fn risk_level(&self) -> RiskLevel {
        self.risk_level
    }
}

/// 操作执行结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct OperationResult {
    changes_applied: u32,
    duration_ms: u32,
}

impl OperationResult {
    /// 创建空结果。
    pub fn new() -> Self {
        Self {
            changes_applied: 0,
            duration_ms: 0,
        }
    }

    /// 设置已应用的变更数。
    pub fn with_changes_applied(mut self, count: u32) -> Self {
        self.changes_applied = count;
        self
    }

    /// 设置执行耗时。
    pub fn with_duration_ms(mut self, ms: u32) -> Self {
        self.duration_ms = ms;
        self
    }

    /// 获取已应用变更数。
    pub fn changes_applied(&self) -> u32 {
        self.changes_applied
    }

    /// 获取执行耗时（毫秒）。
    pub fn duration_ms(&self) -> u32 {
        self.duration_ms
    }
}

/// 危险操作 trait。
///
/// 所有需要用户确认的操作实现此 trait。
pub trait Operation {
    /// 获取操作预览。
    fn preview(&self) -> &Preview;

    /// 执行操作。
    fn execute(&self, ctx: &mut CmdContext) -> Result<OperationResult, XunError>;

    /// 回滚操作（默认不支持）。
    fn rollback(&self, _ctx: &mut CmdContext) -> Result<(), XunError> {
        Err(XunError::user("rollback not supported for this operation"))
    }
}

/// 统一调度函数：preview → confirm → execute。
pub fn run_operation<O: Operation>(
    op: &O,
    ctx: &mut CmdContext,
) -> Result<OperationResult, XunError> {
    let _preview = op.preview();

    // 非交互模式下自动确认；交互模式下需要用户确认
    // 后续集成 confirm_with_preview() 显示预览并询问
    if !ctx.is_non_interactive() {
        // 交互模式：后续实现具体的确认逻辑
        // 目前默认通过
    }

    let start = Instant::now();
    let mut result = op.execute(ctx)?;
    let elapsed = start.elapsed().as_millis() as u32;
    result = result.with_duration_ms(elapsed);
    Ok(result)
}
