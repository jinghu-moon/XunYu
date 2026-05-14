use clap::Args;

use super::defaults::default_output_format;

#[cfg(feature = "fs")]
/// Delete a file or directory.
#[derive(Args, Debug, Clone)]
pub struct RmCmd {
    /// target path
    pub path: String,

    /// unlock file if locked
    #[cfg(feature = "lock")]
    #[arg(long)]
    pub unlock: bool,

    /// force kill blocking processes
    #[cfg(feature = "lock")]
    #[arg(long)]
    pub force_kill: bool,

    /// schedule deletion on reboot
    #[arg(long)]
    pub on_reboot: bool,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,

    /// force operation bypass protection
    #[arg(long)]
    pub force: bool,

    /// reason for bypass protection
    #[arg(long)]
    pub reason: Option<String>,
}
