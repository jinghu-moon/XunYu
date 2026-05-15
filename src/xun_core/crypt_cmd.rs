//! Crypt CLI 定义（clap derive）
//!
//! 新架构的 crypt 命令定义，替代 argh 版本。
//! EncryptCmd + DecryptCmd 独立命令。

use clap::Parser;

// ── Encrypt 命令 ─────────────────────────────────────────────────

/// Encrypt a file using Windows EFS (or other providers).
#[derive(Parser, Debug, Clone)]
#[command(name = "encrypt", about = "Encrypt a file")]
pub struct EncryptCmd {
    /// target path
    pub path: String,

    /// use Windows EFS encryption (Encrypting File System)
    #[arg(long)]
    pub efs: bool,

    /// public key to encrypt to (age format, can be repeated)
    #[arg(long)]
    pub to: Vec<String>,

    /// encrypt with a passphrase (interactive)
    #[arg(long)]
    pub passphrase: bool,

    /// output file path (default: <path>.age if not efs)
    #[arg(short = 'o', long)]
    pub out: Option<String>,
}

// ── Decrypt 命令 ─────────────────────────────────────────────────

/// Decrypt a file.
#[derive(Parser, Debug, Clone)]
#[command(name = "decrypt", about = "Decrypt a file")]
pub struct DecryptCmd {
    /// target path
    pub path: String,

    /// use Windows EFS decryption
    #[arg(long)]
    pub efs: bool,

    /// identity file to decrypt with (age format, can be repeated)
    #[arg(short = 'i', long)]
    pub identity: Vec<String>,

    /// decrypt with a passphrase (interactive)
    #[arg(long)]
    pub passphrase: bool,

    /// output file path (default: remove .age extension if not efs)
    #[arg(short = 'o', long)]
    pub out: Option<String>,
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
#[cfg(feature = "crypt")]
use crate::xun_core::value::Value;

/// encrypt 命令。
#[cfg(feature = "crypt")]
pub struct EncryptCmdSpec {
    pub args: EncryptCmd,
}

#[cfg(feature = "crypt")]
impl CommandSpec for EncryptCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::crypt::cmd_encrypt(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}

/// decrypt 命令。
#[cfg(feature = "crypt")]
pub struct DecryptCmdSpec {
    pub args: DecryptCmd,
}

#[cfg(feature = "crypt")]
impl CommandSpec for DecryptCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::crypt::cmd_decrypt(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}
