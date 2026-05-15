//! Lock CLI 定义（clap derive）
//!
//! 新架构的 lock 命令定义，替代 argh 版本。
//! LockCmd: 1 子命令 (Who) + MvCmd + RenFileCmd 独立命令。

use clap::Parser;

// ── Lock 主命令 ──────────────────────────────────────────────────

/// File locking and unlocking.
#[derive(Parser, Debug, Clone)]
#[command(name = "lock", about = "File locking and unlocking")]
pub struct LockCmd {
    #[command(subcommand)]
    pub cmd: LockSubCommand,
}

/// Lock 子命令枚举。
#[derive(clap::Subcommand, Debug, Clone)]
pub enum LockSubCommand {
    /// Show processes locking a file
    Who(LockWhoCmd),
}

/// Show processes locking a file.
#[derive(Parser, Debug, Clone)]
pub struct LockWhoCmd {
    /// target path
    pub path: String,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

// ── Mv 独立命令 ──────────────────────────────────────────────────

/// Move a file or directory.
#[derive(Parser, Debug, Clone)]
#[command(name = "mv", about = "Move a file or directory")]
pub struct MvCmd {
    /// source path
    pub src: String,

    /// destination path
    pub dst: String,

    /// unlock file if locked
    #[arg(long)]
    pub unlock: bool,

    /// force kill blocking processes
    #[arg(long)]
    pub force_kill: bool,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// force operation bypass protection
    #[arg(long)]
    pub force: bool,

    /// reason for bypass protection
    #[arg(long)]
    pub reason: Option<String>,
}

// ── RenFile 独立命令 ─────────────────────────────────────────────

/// Rename a file or directory.
#[derive(Parser, Debug, Clone)]
#[command(name = "ren", about = "Rename a file or directory")]
pub struct RenFileCmd {
    /// source path
    pub src: String,

    /// destination path
    pub dst: String,

    /// unlock file if locked
    #[arg(long)]
    pub unlock: bool,

    /// force kill blocking processes
    #[arg(long)]
    pub force_kill: bool,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// force operation bypass protection
    #[arg(long)]
    pub force: bool,

    /// reason for bypass protection
    #[arg(long)]
    pub reason: Option<String>,
}

// ============================================================
// CommandSpec 实现
// ============================================================

#[cfg(feature = "lock")]
use crate::xun_core::command::CommandSpec;
#[cfg(feature = "lock")]
use crate::xun_core::context::CmdContext;
#[cfg(feature = "lock")]
use crate::xun_core::error::XunError;
#[cfg(feature = "lock")]
use crate::xun_core::value::Value;

/// lock 命令。
#[cfg(feature = "lock")]
pub struct LockCmdSpec {
    pub args: LockCmd,
}

#[cfg(feature = "lock")]
impl CommandSpec for LockCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::lock::cmd_lock(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}

/// mv 命令。
#[cfg(feature = "lock")]
pub struct MvCmdSpec {
    pub args: MvCmd,
}

#[cfg(feature = "lock")]
impl CommandSpec for MvCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::lock::cmd_mv(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}

/// renfile 命令。
#[cfg(feature = "lock")]
pub struct RenFileCmdSpec {
    pub args: RenFileCmd,
}

#[cfg(feature = "lock")]
impl CommandSpec for RenFileCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::lock::cmd_ren_file(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}
