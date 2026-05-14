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
    pub sub: LockSubCommand,
}

/// Lock 子命令枚举。
#[derive(clap::Subcommand, Debug, Clone)]
pub enum LockSubCommand {
    /// Show processes locking a file
    Who(LockWhoArgs),
}

/// Show processes locking a file.
#[derive(Parser, Debug, Clone)]
pub struct LockWhoArgs {
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
