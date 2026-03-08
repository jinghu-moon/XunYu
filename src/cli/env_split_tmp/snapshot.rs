use super::*;

pub struct EnvSnapshotCmd {
    #[argh(subcommand)]
    pub cmd: EnvSnapshotSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvSnapshotSubCommand {
    Create(EnvSnapshotCreateCmd),
    List(EnvSnapshotListCmd),
    Restore(EnvSnapshotRestoreCmd),
    Prune(EnvSnapshotPruneCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "create")]
/// Create a snapshot.
pub struct EnvSnapshotCreateCmd {
    /// snapshot description
    #[argh(option)]
    pub desc: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// List snapshots.
pub struct EnvSnapshotListCmd {
    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "restore")]
/// Restore a snapshot.
pub struct EnvSnapshotRestoreCmd {
    /// snapshot id
    #[argh(option)]
    pub id: Option<String>,

    /// restore latest snapshot
    #[argh(switch)]
    pub latest: bool,

    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "prune")]
/// Prune old snapshots, keep latest N.
pub struct EnvSnapshotPruneCmd {
    /// how many latest snapshots to keep
    #[argh(option, default = "50")]
    pub keep: usize,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "doctor")]
/// Run environment health checks.

