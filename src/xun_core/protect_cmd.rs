//! Protect CLI 定义（clap derive）
//!
//! 新架构的 protect 命令定义，替代 argh 版本。
//! 3 个子命令。

use clap::{Parser, Subcommand};

// ── Protect 主命令 ───────────────────────────────────────────────

/// Manage protection rules.
#[derive(Parser, Debug, Clone)]
#[command(name = "protect", about = "Manage protection rules")]
pub struct ProtectCmd {
    #[command(subcommand)]
    pub cmd: ProtectSubCommand,
}

/// Protect 子命令枚举（3 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum ProtectSubCommand {
    /// Set a protection rule
    Set(ProtectSetCmd),
    /// Clear a protection rule
    Clear(ProtectClearCmd),
    /// Show protection status
    Status(ProtectStatusCmd),
}

/// Set a protection rule.
#[derive(Parser, Debug, Clone)]
pub struct ProtectSetCmd {
    /// path to protect
    pub path: String,

    /// actions to deny (e.g. delete,move,rename)
    #[arg(long, default_value = "delete,move,rename")]
    pub deny: String,

    /// requirements to bypass (e.g. force,reason)
    #[arg(long, default_value = "force,reason")]
    pub require: String,

    /// apply NTFS ACL Deny Delete rule (deep Windows protection)
    #[arg(long)]
    pub system_acl: bool,
}

/// Clear a protection rule.
#[derive(Parser, Debug, Clone)]
pub struct ProtectClearCmd {
    /// path to clear protection
    pub path: String,

    /// remove NTFS ACL Deny Delete rule as well
    #[arg(long)]
    pub system_acl: bool,
}

/// Show protection status.
#[derive(Parser, Debug, Clone)]
pub struct ProtectStatusCmd {
    /// filter by path prefix
    pub path: Option<String>,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

// ============================================================
// CommandSpec 实现
// ============================================================

#[cfg(feature = "protect")]
use crate::xun_core::command::CommandSpec;
#[cfg(feature = "protect")]
use crate::xun_core::context::CmdContext;
#[cfg(feature = "protect")]
use crate::xun_core::error::XunError;
#[cfg(feature = "protect")]
use crate::xun_core::value::Value;

/// protect 命令。
#[cfg(feature = "protect")]
pub struct ProtectCmdSpec {
    pub args: ProtectCmd,
}

#[cfg(feature = "protect")]
impl CommandSpec for ProtectCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::protect::cmd_protect(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}
