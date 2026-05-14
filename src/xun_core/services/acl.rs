//! ACL 业务逻辑服务
//!
//! 封装 ACL 权限管理操作，支持 CommandSpec 和 Operation 实现。
//!
//! 注意：ACL 模块使用 Windows-specific 类型（PSID, AceType 等），
//! 此处提供桥接实现，后续可逐步完善。

use std::path::Path;

use crate::xun_core::error::XunError;
use crate::xun_core::operation::{Change, Operation, OperationResult, Preview, RiskLevel};
use crate::xun_core::value::Value;

// ============================================================
// AclAddOp — Operation trait 实现
// ============================================================

/// ACL 添加操作（实现 Operation trait）。
pub struct AclAddOp {
    path: String,
    principal: String,
    rights: String,
    preview: Preview,
}

impl AclAddOp {
    pub fn new(
        path: impl Into<String>,
        principal: impl Into<String>,
        rights: impl Into<String>,
    ) -> Self {
        let path = path.into();
        let principal = principal.into();
        let rights = rights.into();
        let preview = Preview::new(format!("Add ACL rule for '{}' on '{}'", principal, path))
            .add_change(Change::new("add", format!("{principal} → {rights}")))
            .with_risk_level(RiskLevel::High);
        Self {
            path,
            principal,
            rights,
            preview,
        }
    }
}

impl Operation for AclAddOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        // ACL writer 需要 Windows-specific 类型，此处提供桥接
        // 后续集成时替换为实际的 acl::writer::add_rule 调用
        Err(XunError::user(
            "ACL add not yet integrated with Operation Runtime",
        ))
    }
}

// ============================================================
// AclRemoveOp — Operation trait 实现
// ============================================================

/// ACL 删除操作（实现 Operation trait）。
pub struct AclRemoveOp {
    path: String,
    principal: String,
    preview: Preview,
}

impl AclRemoveOp {
    pub fn new(path: impl Into<String>, principal: impl Into<String>) -> Self {
        let path = path.into();
        let principal = principal.into();
        let preview = Preview::new(format!("Remove ACL rules for '{}' on '{}'", principal, path))
            .add_change(Change::new("remove", format!("{principal} from {path}")))
            .with_risk_level(RiskLevel::High);
        Self {
            path,
            principal,
            preview,
        }
    }
}

impl Operation for AclRemoveOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        // ACL writer 需要 AceEntry 列表，此处提供桥接
        // 后续集成时替换为实际的 acl::writer::remove_rules 调用
        Err(XunError::user(
            "ACL remove not yet integrated with Operation Runtime",
        ))
    }
}

// ============================================================
// AclRepairOp — Operation trait 实现
// ============================================================

/// ACL 修复操作（实现 Operation trait）。
pub struct AclRepairOp {
    path: String,
    preview: Preview,
}

impl AclRepairOp {
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        let preview = Preview::new(format!("Repair ACL on '{}'", path))
            .add_change(Change::new("repair", &path))
            .with_risk_level(RiskLevel::Critical);
        Self { path, preview }
    }
}

impl Operation for AclRepairOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        // ACL repair 需要 AclConfig 和 admins SID，此处提供桥接
        // 后续集成时替换为实际的 acl::repair::force_repair 调用
        Err(XunError::user(
            "ACL repair not yet integrated with Operation Runtime",
        ))
    }
}

// ============================================================
// ACL 查询服务
// ============================================================

/// 显示 ACL 信息。
pub fn show_acl(path: &str, _detail: bool) -> Result<Value, XunError> {
    let _p = Path::new(path);
    // ACL reader 需要 Windows PSID 类型，此处返回占位
    // 后续集成时替换为实际的 acl::reader::get_acl 调用
    Ok(Value::List(vec![]))
}
