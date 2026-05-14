use clap::{Args, Parser, Subcommand};

use super::defaults::default_output_format;

#[cfg(feature = "protect")]
/// Manage protection rules.
#[derive(Parser, Debug, Clone)]
pub struct ProtectCmd {
    #[command(subcommand)]
    pub cmd: ProtectSubCommand,
}

#[cfg(feature = "protect")]
#[derive(Subcommand, Debug, Clone)]
pub enum ProtectSubCommand {
    Set(ProtectSetCmd),
    Clear(ProtectClearCmd),
    Status(ProtectStatusCmd),
}

#[cfg(feature = "protect")]
/// Set a protection rule.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "protect")]
/// Clear a protection rule.
#[derive(Args, Debug, Clone)]
pub struct ProtectClearCmd {
    /// path to clear protection
    pub path: String,

    /// remove NTFS ACL Deny Delete rule as well
    #[arg(long)]
    pub system_acl: bool,
}

#[cfg(feature = "protect")]
/// Show protection status.
#[derive(Args, Debug, Clone)]
pub struct ProtectStatusCmd {
    /// filter by path prefix
    pub path: Option<String>,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}
