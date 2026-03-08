use argh::FromArgs;

use super::defaults::default_output_format;

#[cfg(feature = "lock")]
/// File locking and unlocking.
#[derive(FromArgs)]
#[argh(subcommand, name = "lock")]
pub struct LockCmd {
    #[argh(subcommand)]
    pub cmd: LockSubCommand,
}

#[cfg(feature = "lock")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum LockSubCommand {
    Who(LockWhoCmd),
}

#[cfg(feature = "lock")]
/// Show processes locking a file.
#[derive(FromArgs)]
#[argh(subcommand, name = "who")]
pub struct LockWhoCmd {
    /// target path
    #[argh(positional)]
    pub path: String,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[cfg(feature = "lock")]
/// Move a file or directory.
#[derive(FromArgs)]
#[argh(subcommand, name = "mv")]
pub struct MvCmd {
    /// source path
    #[argh(positional)]
    pub src: String,

    /// destination path
    #[argh(positional)]
    pub dst: String,

    /// unlock file if locked
    #[argh(switch)]
    pub unlock: bool,

    /// force kill blocking processes
    #[argh(switch)]
    pub force_kill: bool,

    /// dry run
    #[argh(switch)]
    pub dry_run: bool,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,

    /// force operation bypass protection
    #[argh(switch)]
    pub force: bool,

    /// reason for bypass protection
    #[argh(option)]
    pub reason: Option<String>,
}

#[cfg(feature = "lock")]
/// Rename a file or directory.
#[derive(FromArgs)]
#[argh(subcommand, name = "ren")]
pub struct RenFileCmd {
    /// source path
    #[argh(positional)]
    pub src: String,

    /// destination path
    #[argh(positional)]
    pub dst: String,

    /// unlock file if locked
    #[argh(switch)]
    pub unlock: bool,

    /// force kill blocking processes
    #[argh(switch)]
    pub force_kill: bool,

    /// dry run
    #[argh(switch)]
    pub dry_run: bool,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,

    /// force operation bypass protection
    #[argh(switch)]
    pub force: bool,

    /// reason for bypass protection
    #[argh(option)]
    pub reason: Option<String>,
}
