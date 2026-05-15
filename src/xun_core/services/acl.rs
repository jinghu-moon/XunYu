//! ACL 业务逻辑服务
//!
//! 封装 ACL 权限管理操作，桥接到 `crate::acl::*` 底层模块。

use std::path::Path;

use crate::xun_core::error::XunError;
use crate::xun_core::operation::{Change, Operation, OperationResult, Preview, RiskLevel};
use crate::xun_core::value::Value;

// ── 错误桥接 ─────────────────────────────────────────────────

fn anyhow_to_xun(e: anyhow::Error) -> XunError {
    XunError::user(format!("{e:#}"))
}

// ============================================================
// AclAddOp — Operation trait 实现
// ============================================================

/// ACL 添加操作（实现 Operation trait）。
pub struct AclAddOp {
    path: String,
    principal: String,
    rights: String,
    ace_type: String,
    inherit: String,
    preview: Preview,
}

impl AclAddOp {
    pub fn new(
        path: impl Into<String>,
        principal: impl Into<String>,
        rights: impl Into<String>,
        ace_type: impl Into<String>,
        inherit: impl Into<String>,
    ) -> Self {
        let path = path.into();
        let principal = principal.into();
        let rights = rights.into();
        let ace_type: String = ace_type.into();
        let inherit: String = inherit.into();
        let preview = Preview::new(format!(
            "Add ACL rule for '{}' on '{}' ({}, {})",
            principal, path, rights, ace_type
        ))
        .add_change(Change::new("add", format!("{principal} → {rights}")))
        .with_risk_level(RiskLevel::High);
        Self {
            path,
            principal,
            rights,
            ace_type,
            inherit,
            preview,
        }
    }
}

impl Operation for AclAddOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        use crate::acl::parse::{parse_ace_type, parse_inheritance, parse_rights};
        use crate::acl::writer::add_rule;

        let path = Path::new(&self.path);
        let rights_mask = parse_rights(&self.rights).map_err(anyhow_to_xun)?;
        let ace_type = parse_ace_type(&self.ace_type).map_err(anyhow_to_xun)?;
        let inheritance = parse_inheritance(&self.inherit).map_err(anyhow_to_xun)?;
        let propagation = crate::acl::types::PropagationFlags(0);

        add_rule(path, &self.principal, rights_mask, ace_type, inheritance, propagation)
            .map_err(anyhow_to_xun)?;

        Ok(OperationResult::new().with_changes_applied(1))
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
        let preview = Preview::new(format!(
            "Remove ACL rules for '{}' on '{}'",
            principal, path
        ))
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
        use crate::acl::writer::purge_principal;

        let path = Path::new(&self.path);
        let removed = purge_principal(path, &self.principal).map_err(anyhow_to_xun)?;

        Ok(OperationResult::new().with_changes_applied(removed))
    }
}

// ============================================================
// AclRepairOp — Operation trait 实现
// ============================================================

/// ACL 修复操作（实现 Operation trait）。
#[allow(dead_code)]
pub struct AclRepairOp {
    path: String,
    reset_clean: bool,
    grant: Option<String>,
    preview: Preview,
}

impl AclRepairOp {
    pub fn new(
        path: impl Into<String>,
        reset_clean: bool,
        grant: Option<String>,
    ) -> Self {
        let path = path.into();
        let desc = if reset_clean {
            format!("Clean reset ACL on '{}'", path)
        } else {
            format!("Repair ACL on '{}'", path)
        };
        let risk = if reset_clean {
            RiskLevel::Critical
        } else {
            RiskLevel::High
        };
        let preview = Preview::new(desc)
            .add_change(Change::new("repair", &path))
            .with_risk_level(risk);
        Self {
            path,
            reset_clean,
            grant,
            preview,
        }
    }
}

impl Operation for AclRepairOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        use crate::acl::repair::force_repair;
        use crate::config::load_config;

        let path = Path::new(&self.path);
        let config = load_config();
        let stats = force_repair(path, &config.acl, false).map_err(anyhow_to_xun)?;

        Ok(OperationResult::new().with_changes_applied(stats.total as u32))
    }
}

// ============================================================
// ACL 查询服务
// ============================================================

/// 显示 ACL 信息。
pub fn show_acl(path: &str, _detail: bool) -> Result<Value, XunError> {
    use crate::acl::reader::get_acl;
    use crate::acl::types::rights_short;

    let p = Path::new(path);
    let snapshot = get_acl(p).map_err(anyhow_to_xun)?;

    let mut entries = Vec::new();
    for entry in &snapshot.entries {
        entries.push(serde_json::json!({
            "principal": entry.principal,
            "rights": rights_short(entry.rights_mask).to_string(),
            "ace_type": format!("{:?}", entry.ace_type),
            "inherited": entry.is_inherited,
        }));
    }

    Ok(Value::String(serde_json::to_string_pretty(&entries).unwrap_or_default()))
}
