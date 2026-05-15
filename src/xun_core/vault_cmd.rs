//! Vault CLI 定义（clap derive）
//!
//! 新架构的 vault 命令定义，替代 argh 版本。
//! 8 个子命令。

use clap::{Parser, Subcommand};

use super::table_row::TableRow;
use super::value::{ColumnDef, Value, ValueKind};

// ── Vault 主命令 ──────────────────────────────────────────────────

/// FileVault v13 foundation commands.
#[derive(Parser, Debug, Clone)]
#[command(name = "vault", about = "FileVault v13 management")]
pub struct VaultCmd {
    #[command(subcommand)]
    pub cmd: VaultSubCommand,
}

/// Vault 子命令枚举（8 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum VaultSubCommand {
    /// Encrypt a file into FileVault v13 format
    Enc(VaultEncCmd),
    /// Decrypt a FileVault v13 ciphertext
    Dec(VaultDecCmd),
    /// Inspect FileVault v13 structure and slot metadata
    Inspect(VaultInspectCmd),
    /// Verify FileVault v13 integrity without exporting plaintext
    Verify(VaultVerifyCmd),
    /// Resume an interrupted encryption task from its journal
    Resume(VaultResumeCmd),
    /// Remove FileVault temporary artifacts
    Cleanup(VaultCleanupCmd),
    /// Replace wrapped slots without re-encrypting payload
    Rewrap(VaultRewrapCmd),
    /// Rebuild a recovery-key slot from another legal unlock path
    RecoverKey(VaultRecoverKeyCmd),
}

// ── 子命令参数 ──────────────────────────────────────────────────

/// Encrypt a file into FileVault v13 format.
#[derive(Parser, Debug, Clone)]
pub struct VaultEncCmd {
    /// source plaintext path
    pub input: String,

    /// output ciphertext path (default: <input>.fv)
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// password slot value
    #[arg(long)]
    pub password: Option<String>,

    /// keyfile slot path
    #[arg(long)]
    pub keyfile: Option<String>,

    /// import an existing recovery key as a slot
    #[arg(long)]
    pub recovery_key: Option<String>,

    /// generate a new recovery key and write it to the given file
    #[arg(long)]
    pub emit_recovery_key: Option<String>,

    /// add a same-profile Windows DPAPI slot
    #[arg(long)]
    pub dpapi: bool,

    /// payload algorithm: aes256-gcm|xchacha20-poly1305
    #[arg(long, default_value = "aes256-gcm")]
    pub algo: String,

    /// password/keyfile KDF: argon2id|pbkdf2-sha256
    #[arg(long, default_value = "argon2id")]
    pub kdf: String,

    /// chunk size in bytes
    #[arg(long, default_value_t = 262144)]
    pub chunk_size: u32,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

/// Decrypt a FileVault v13 ciphertext.
#[derive(Parser, Debug, Clone)]
pub struct VaultDecCmd {
    /// source ciphertext path
    pub input: String,

    /// output plaintext path
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// unlock with password
    #[arg(long)]
    pub password: Option<String>,

    /// unlock with keyfile path
    #[arg(long)]
    pub keyfile: Option<String>,

    /// unlock with recovery key text
    #[arg(long)]
    pub recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[arg(long)]
    pub dpapi: bool,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

/// Inspect FileVault v13 structure and slot metadata.
#[derive(Parser, Debug, Clone)]
pub struct VaultInspectCmd {
    /// ciphertext path
    pub path: String,

    /// print json instead of human-readable text
    #[arg(long)]
    pub json: bool,
}

/// Verify FileVault v13 integrity without exporting plaintext.
#[derive(Parser, Debug, Clone)]
pub struct VaultVerifyCmd {
    /// ciphertext path
    pub path: String,

    /// unlock with password for authenticated verification
    #[arg(long)]
    pub password: Option<String>,

    /// unlock with keyfile path for authenticated verification
    #[arg(long)]
    pub keyfile: Option<String>,

    /// unlock with recovery key text for authenticated verification
    #[arg(long)]
    pub recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI for authenticated verification
    #[arg(long)]
    pub dpapi: bool,

    /// print json instead of human-readable text
    #[arg(long)]
    pub json: bool,
}

/// Resume an interrupted encryption task from its journal.
#[derive(Parser, Debug, Clone)]
pub struct VaultResumeCmd {
    /// intended final ciphertext path
    pub path: String,

    /// unlock with password
    #[arg(long)]
    pub password: Option<String>,

    /// unlock with keyfile path
    #[arg(long)]
    pub keyfile: Option<String>,

    /// unlock with recovery key text
    #[arg(long)]
    pub recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[arg(long)]
    pub dpapi: bool,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

/// Remove FileVault temporary artifacts.
#[derive(Parser, Debug, Clone)]
pub struct VaultCleanupCmd {
    /// intended final ciphertext path
    pub path: String,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

/// Replace wrapped slots without re-encrypting payload.
#[derive(Parser, Debug, Clone)]
pub struct VaultRewrapCmd {
    /// ciphertext path
    pub path: String,

    /// unlock with current password
    #[arg(long)]
    pub unlock_password: Option<String>,

    /// unlock with current keyfile path
    #[arg(long)]
    pub unlock_keyfile: Option<String>,

    /// unlock with current recovery key text
    #[arg(long)]
    pub unlock_recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[arg(long)]
    pub unlock_dpapi: bool,

    /// add or replace a password slot
    #[arg(long)]
    pub add_password: Option<String>,

    /// add or replace a keyfile slot
    #[arg(long)]
    pub add_keyfile: Option<String>,

    /// add or replace a recovery-key slot with an existing recovery key
    #[arg(long)]
    pub add_recovery_key: Option<String>,

    /// generate and add a new recovery-key slot, then write it to this file
    #[arg(long)]
    pub emit_recovery_key: Option<String>,

    /// add or replace a DPAPI slot
    #[arg(long)]
    pub add_dpapi: bool,

    /// remove slots by kind: password|keyfile|recovery-key|dpapi
    #[arg(long)]
    pub remove_slot: Vec<String>,

    /// password/keyfile KDF: argon2id|pbkdf2-sha256
    #[arg(long, default_value = "argon2id")]
    pub kdf: String,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

/// Rebuild a recovery-key slot from another legal unlock path.
#[derive(Parser, Debug, Clone)]
pub struct VaultRecoverKeyCmd {
    /// ciphertext path
    pub path: String,

    /// unlock with current password
    #[arg(long)]
    pub unlock_password: Option<String>,

    /// unlock with current keyfile path
    #[arg(long)]
    pub unlock_keyfile: Option<String>,

    /// unlock with current recovery key text
    #[arg(long)]
    pub unlock_recovery_key: Option<String>,

    /// unlock with same-profile Windows DPAPI
    #[arg(long)]
    pub unlock_dpapi: bool,

    /// output file to receive the regenerated recovery key
    pub output: String,

    /// print machine-readable json
    #[arg(long)]
    pub json: bool,
}

// ── 输出类型：VaultEntry ──────────────────────────────────────────

/// Vault 条目（inspect/verify 结果）。
#[derive(Debug, Clone)]
pub struct VaultEntry {
    pub path: String,
    pub algo: String,
    pub slots: usize,
    pub size: u64,
}

impl VaultEntry {
    pub fn new(
        path: impl Into<String>,
        algo: impl Into<String>,
        slots: usize,
        size: u64,
    ) -> Self {
        Self {
            path: path.into(),
            algo: algo.into(),
            slots,
            size,
        }
    }
}

impl TableRow for VaultEntry {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("path", ValueKind::Path),
            ColumnDef::new("algo", ValueKind::String),
            ColumnDef::new("slots", ValueKind::Int),
            ColumnDef::new("size", ValueKind::Int),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.path.clone()),
            Value::String(self.algo.clone()),
            Value::Int(self.slots as i64),
            Value::Int(self.size as i64),
        ]
    }
}

// ============================================================
// CommandSpec 实现
// ============================================================

#[cfg(feature = "crypt")]
use crate::xun_core::command::CommandSpec;
#[cfg(feature = "crypt")]
use crate::xun_core::context::CmdContext;
#[cfg(feature = "crypt")]
use crate::xun_core::error::XunError;

/// vault 命令。
#[cfg(feature = "crypt")]
pub struct VaultCmdSpec {
    pub args: VaultCmd,
}

#[cfg(feature = "crypt")]
impl CommandSpec for VaultCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::vault::cmd_vault(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}
