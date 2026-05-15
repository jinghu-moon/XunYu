//! FileVault 业务逻辑服务
//!
//! 封装 FileVault v13 加密/解密操作，桥接到 `crate::filevault::*` 底层模块。
//! 仅在 `crypt` feature 启用时编译。

use crate::xun_core::error::XunError;
use crate::xun_core::operation::{Change, Operation, OperationResult, Preview, RiskLevel};
use crate::xun_core::value::Value;

use crate::filevault::{
    DecryptOptions, EncryptOptions, RecoverKeyOptions,
    RewrapOptions, VerifyOptions, cleanup_artifacts, decrypt_file, encrypt_file,
    inspect_file, recover_key_file, resume_file, rewrap_file, verify_file,
};

// ── 类型转换 ─────────────────────────────────────────────────

fn json_value_to_xun(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Value::Int(i) }
            else if let Some(f) = n.as_f64() { Value::Float(f) }
            else { Value::String(n.to_string()) }
        }
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => Value::List(arr.into_iter().map(json_value_to_xun).collect()),
        serde_json::Value::Object(map) => Value::Record(
            map.into_iter().map(|(k, v)| (k, json_value_to_xun(v))).collect()
        ),
    }
}

// ── 错误桥接 ─────────────────────────────────────────────────

fn vault_err_to_xun(e: crate::filevault::FileVaultError) -> XunError {
    XunError::user(format!("{e}"))
}

// ============================================================
// VaultEncryptOp — 加密操作
// ============================================================

/// FileVault 加密操作。
pub struct VaultEncryptOp {
    options: EncryptOptions,
    preview: Preview,
}

impl VaultEncryptOp {
    pub fn new(options: EncryptOptions) -> Self {
        let input = options.input.to_string_lossy().to_string();
        let preview = Preview::new(format!("Encrypt '{}'", input))
            .add_change(Change::new("encrypt", &input))
            .with_risk_level(RiskLevel::Medium);
        Self { options, preview }
    }
}

impl Operation for VaultEncryptOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        let _value = encrypt_file(&self.options).map_err(vault_err_to_xun)?;
        Ok(OperationResult::new().with_changes_applied(1))
    }
}

// ============================================================
// VaultDecryptOp — 解密操作
// ============================================================

/// FileVault 解密操作。
pub struct VaultDecryptOp {
    options: DecryptOptions,
    preview: Preview,
}

impl VaultDecryptOp {
    pub fn new(options: DecryptOptions) -> Self {
        let input = options.input.to_string_lossy().to_string();
        let preview = Preview::new(format!("Decrypt '{}'", input))
            .add_change(Change::new("decrypt", &input))
            .with_risk_level(RiskLevel::Medium);
        Self { options, preview }
    }
}

impl Operation for VaultDecryptOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        let _value = decrypt_file(&self.options).map_err(vault_err_to_xun)?;
        Ok(OperationResult::new().with_changes_applied(1))
    }
}

// ============================================================
// VaultVerifyOp — 校验操作
// ============================================================

/// FileVault 校验操作。
pub struct VaultVerifyOp {
    options: VerifyOptions,
    preview: Preview,
}

impl VaultVerifyOp {
    pub fn new(options: VerifyOptions) -> Self {
        let path = options.input.to_string_lossy().to_string();
        let preview = Preview::new(format!("Verify '{}'", path))
            .add_change(Change::new("verify", &path))
            .with_risk_level(RiskLevel::Low);
        Self { options, preview }
    }
}

impl Operation for VaultVerifyOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        let _value = verify_file(&self.options).map_err(vault_err_to_xun)?;
        Ok(OperationResult::new().with_changes_applied(1))
    }
}

// ============================================================
// VaultRewrapOp — 密钥重包装操作
// ============================================================

/// FileVault 密钥重包装操作。
pub struct VaultRewrapOp {
    options: RewrapOptions,
    preview: Preview,
}

impl VaultRewrapOp {
    pub fn new(options: RewrapOptions) -> Self {
        let path = options.path.to_string_lossy().to_string();
        let preview = Preview::new(format!("Rewrap '{}'", path))
            .add_change(Change::new("rewrap", &path))
            .with_risk_level(RiskLevel::High);
        Self { options, preview }
    }
}

impl Operation for VaultRewrapOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        let _value = rewrap_file(&self.options).map_err(vault_err_to_xun)?;
        Ok(OperationResult::new().with_changes_applied(1))
    }
}

// ============================================================
// VaultRecoverKeyOp — 恢复密钥操作
// ============================================================

/// FileVault 恢复密钥操作。
pub struct VaultRecoverKeyOp {
    options: RecoverKeyOptions,
    preview: Preview,
}

impl VaultRecoverKeyOp {
    pub fn new(options: RecoverKeyOptions) -> Self {
        let path = options.path.to_string_lossy().to_string();
        let preview = Preview::new(format!("Recover key for '{}'", path))
            .add_change(Change::new("recover_key", &path))
            .with_risk_level(RiskLevel::High);
        Self { options, preview }
    }
}

impl Operation for VaultRecoverKeyOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        let _value = recover_key_file(&self.options).map_err(vault_err_to_xun)?;
        Ok(OperationResult::new().with_changes_applied(1))
    }
}

// ============================================================
// 查询函数
// ============================================================

/// 检查 FileVault 文件结构。
pub fn inspect(path: &str) -> Result<Value, XunError> {
    let p = std::path::Path::new(path);
    let value = inspect_file(p).map_err(vault_err_to_xun)?;
    Ok(json_value_to_xun(value))
}

/// 恢复中断的加密任务。
pub fn resume(path: &str, _password: Option<&str>, _keyfile: Option<&str>, _recovery_key: Option<&str>, _dpapi: bool) -> Result<Value, XunError> {
    let p = std::path::Path::new(path);
    let value = resume_file(p).map_err(vault_err_to_xun)?;
    Ok(json_value_to_xun(value))
}

/// 清理临时文件。
pub fn cleanup(path: &str) -> Result<Value, XunError> {
    let p = std::path::Path::new(path);
    let value = cleanup_artifacts(p).map_err(vault_err_to_xun)?;
    Ok(json_value_to_xun(value))
}
