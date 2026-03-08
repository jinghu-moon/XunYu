use super::*;

pub struct EnvPathCmd {
    #[argh(subcommand)]
    pub cmd: EnvPathSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "path-dedup")]
/// Deduplicate PATH entries.
pub struct EnvPathDedupCmd {
    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// remove missing directories while deduping
    #[argh(switch)]
    pub remove_missing: bool,

    /// preview only, do not write
    #[argh(switch)]
    pub dry_run: bool,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvPathSubCommand {
    Add(EnvPathAddCmd),
    Rm(EnvPathRmCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
/// Add one PATH entry.
pub struct EnvPathAddCmd {
    /// path entry
    #[argh(positional)]
    pub entry: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// insert at the front
    #[argh(switch)]
    pub head: bool,

    /// insert at the end
    #[argh(switch)]
    pub tail: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "rm")]
/// Remove one PATH entry.
pub struct EnvPathRmCmd {
    /// path entry
    #[argh(positional)]
    pub entry: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "snapshot")]
/// Snapshot operations.

