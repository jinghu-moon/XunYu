use argh::FromArgs;

use super::defaults::default_output_format;

#[cfg(feature = "protect")]
/// Manage protection rules.
#[derive(FromArgs)]
#[argh(subcommand, name = "protect")]
pub struct ProtectCmd {
    #[argh(subcommand)]
    pub cmd: ProtectSubCommand,
}

#[cfg(feature = "protect")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum ProtectSubCommand {
    Set(ProtectSetCmd),
    Clear(ProtectClearCmd),
    Status(ProtectStatusCmd),
}

#[cfg(feature = "protect")]
/// Set a protection rule.
#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
pub struct ProtectSetCmd {
    /// path to protect
    #[argh(positional)]
    pub path: String,

    /// actions to deny (e.g. delete,move,rename)
    #[argh(option, default = "String::from(\"delete,move,rename\")")]
    pub deny: String,

    /// requirements to bypass (e.g. force,reason)
    #[argh(option, default = "String::from(\"force,reason\")")]
    pub require: String,

    /// apply NTFS ACL Deny Delete rule (deep Windows protection)
    #[argh(switch)]
    pub system_acl: bool,
}

#[cfg(feature = "protect")]
/// Clear a protection rule.
#[derive(FromArgs)]
#[argh(subcommand, name = "clear")]
pub struct ProtectClearCmd {
    /// path to clear protection
    #[argh(positional)]
    pub path: String,

    /// remove NTFS ACL Deny Delete rule as well
    #[argh(switch)]
    pub system_acl: bool,
}

#[cfg(feature = "protect")]
/// Show protection status.
#[derive(FromArgs)]
#[argh(subcommand, name = "status")]
pub struct ProtectStatusCmd {
    /// filter by path prefix
    #[argh(positional)]
    pub path: Option<String>,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}
