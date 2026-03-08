use argh::FromArgs;

use super::defaults::default_output_format;

#[cfg(feature = "fs")]
/// Delete a file or directory.
#[derive(FromArgs)]
#[argh(subcommand, name = "rm")]
pub struct RmCmd {
    /// target path
    #[argh(positional)]
    pub path: String,

    /// unlock file if locked
    #[cfg(feature = "lock")]
    #[argh(switch)]
    pub unlock: bool,

    /// force kill blocking processes
    #[cfg(feature = "lock")]
    #[argh(switch)]
    pub force_kill: bool,

    /// schedule deletion on reboot
    #[argh(switch)]
    pub on_reboot: bool,

    /// dry run
    #[argh(switch)]
    pub dry_run: bool,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,

    /// force operation bypass protection
    #[argh(switch)]
    pub force: bool,

    /// reason for bypass protection
    #[argh(option)]
    pub reason: Option<String>,
}
