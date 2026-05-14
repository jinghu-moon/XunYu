use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
/// PATH operations.
pub struct EnvPathCmd {
    #[command(subcommand)]
    pub cmd: EnvPathSubCommand,
}

#[derive(Args, Debug, Clone)]
/// Deduplicate PATH entries.
pub struct EnvPathDedupCmd {
    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// remove missing directories while deduping
    #[arg(long)]
    pub remove_missing: bool,

    /// preview only, do not write
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum EnvPathSubCommand {
    Add(EnvPathAddCmd),
    Rm(EnvPathRmCmd),
}

#[derive(Args, Debug, Clone)]
/// Add one PATH entry.
pub struct EnvPathAddCmd {
    /// path entry
    pub entry: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// insert at the front
    #[arg(long)]
    pub head: bool,

    /// insert at the end
    #[arg(long)]
    pub tail: bool,
}

#[derive(Args, Debug, Clone)]
/// Remove one PATH entry.
pub struct EnvPathRmCmd {
    /// path entry
    pub entry: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,
}
