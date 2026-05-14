use super::super::defaults::default_output_format;
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
/// Snapshot operations.
pub struct EnvSnapshotCmd {
    #[command(subcommand)]
    pub cmd: EnvSnapshotSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum EnvSnapshotSubCommand {
    Create(EnvSnapshotCreateCmd),
    List(EnvSnapshotListCmd),
    Restore(EnvSnapshotRestoreCmd),
    Prune(EnvSnapshotPruneCmd),
}

#[derive(Args, Debug, Clone)]
/// Create a snapshot.
pub struct EnvSnapshotCreateCmd {
    /// snapshot description
    #[arg(long)]
    pub desc: Option<String>,
}

#[derive(Args, Debug, Clone)]
/// List snapshots.
pub struct EnvSnapshotListCmd {
    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Restore a snapshot.
pub struct EnvSnapshotRestoreCmd {
    /// snapshot id
    #[arg(long)]
    pub id: Option<String>,

    /// restore latest snapshot
    #[arg(long)]
    pub latest: bool,

    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug, Clone)]
/// Prune old snapshots, keep latest N.
pub struct EnvSnapshotPruneCmd {
    /// how many latest snapshots to keep
    #[arg(long, default_value_t = 50)]
    pub keep: usize,
}
