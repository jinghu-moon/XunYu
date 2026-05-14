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
    pub sub: ProtectSubCommand,
}

/// Protect 子命令枚举（3 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum ProtectSubCommand {
    /// Set a protection rule
    Set(ProtectSetArgs),
    /// Clear a protection rule
    Clear(ProtectClearArgs),
    /// Show protection status
    Status(ProtectStatusArgs),
}

/// Set a protection rule.
#[derive(Parser, Debug, Clone)]
pub struct ProtectSetArgs {
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
pub struct ProtectClearArgs {
    /// path to clear protection
    pub path: String,

    /// remove NTFS ACL Deny Delete rule as well
    #[arg(long)]
    pub system_acl: bool,
}

/// Show protection status.
#[derive(Parser, Debug, Clone)]
pub struct ProtectStatusArgs {
    /// filter by path prefix
    pub path: Option<String>,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}
