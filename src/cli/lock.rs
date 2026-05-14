use clap::{Args, Parser, Subcommand};

use super::defaults::default_output_format;

#[cfg(feature = "lock")]
/// File locking and unlocking.
#[derive(Parser, Debug, Clone)]
pub struct LockCmd {
    #[command(subcommand)]
    pub cmd: LockSubCommand,
}

#[cfg(feature = "lock")]
#[derive(Subcommand, Debug, Clone)]
pub enum LockSubCommand {
    Who(LockWhoCmd),
}

#[cfg(feature = "lock")]
/// Show processes locking a file.
#[derive(Args, Debug, Clone)]
pub struct LockWhoCmd {
    /// target path
    pub path: String,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

#[cfg(feature = "lock")]
/// Move a file or directory.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "lock")]
/// Rename a file or directory.
#[derive(Args, Debug, Clone)]
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
