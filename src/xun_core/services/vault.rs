//! Vault 业务逻辑服务
//!
//! 封装文件加密/解密操作，支持 Operation trait 实现。
//!
//! 注意：filevault 模块使用 EncryptOptions/DecryptOptions 等结构体，
//! 此处提供桥接实现，后续可逐步迁移。

use crate::xun_core::error::XunError;
use crate::xun_core::operation::{Change, Operation, OperationResult, Preview, RiskLevel};
use crate::xun_core::value::Value;

// ============================================================
// VaultEncOp — Operation trait 实现
// ============================================================

/// 加密操作（实现 Operation trait）。
pub struct VaultEncOp {
    path: String,
    output: Option<String>,
    preview: Preview,
}

impl VaultEncOp {
    pub fn new(path: impl Into<String>, output: Option<String>) -> Self {
        let path = path.into();
        let preview = Preview::new(format!("Encrypt file '{}'", path))
            .add_change(Change::new("encrypt", &path))
            .with_risk_level(RiskLevel::High);
        Self {
            path,
            output,
            preview,
        }
    }
}

impl Operation for VaultEncOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        // filevault::encrypt_file 需要 EncryptOptions 结构体
        // 此处提供桥接，后续集成时替换为实际调用
        Err(XunError::user(
            "vault encrypt not yet integrated with Operation Runtime",
        ))
    }
}

// ============================================================
// VaultDecOp — Operation trait 实现
// ============================================================

/// 解密操作（实现 Operation trait）。
pub struct VaultDecOp {
    path: String,
    output: Option<String>,
    preview: Preview,
}

impl VaultDecOp {
    pub fn new(path: impl Into<String>, output: Option<String>) -> Self {
        let path = path.into();
        let preview = Preview::new(format!("Decrypt file '{}'", path))
            .add_change(Change::new("decrypt", &path))
            .with_risk_level(RiskLevel::High);
        Self {
            path,
            output,
            preview,
        }
    }
}

impl Operation for VaultDecOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        // filevault::decrypt_file 需要 DecryptOptions 结构体
        // 此处提供桥接，后续集成时替换为实际调用
        Err(XunError::user(
            "vault decrypt not yet integrated with Operation Runtime",
        ))
    }
}

/// 列出加密文件。
///
/// TODO: 桥接到 `crate::filevault` 模块。
pub fn list_vault_entries() -> Result<Value, XunError> {
    Ok(Value::List(vec![]))
}
