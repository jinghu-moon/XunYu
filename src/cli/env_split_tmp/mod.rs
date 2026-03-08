use argh::FromArgs;

use super::defaults::default_output_format;

mod annotations;
mod batch;
mod config;
mod diff_graph;
mod doctor;
mod import_export;
mod path;
mod profile;
mod run;
mod schema;
mod snapshot;
mod status;
mod vars;

pub use annotations::*;
pub use batch::*;
pub use config::*;
pub use diff_graph::*;
pub use doctor::*;
pub use import_export::*;
pub use path::*;
pub use profile::*;
pub use run::*;
pub use schema::*;
pub use snapshot::*;
pub use status::*;
pub use vars::*;

#[derive(FromArgs)]
#[argh(subcommand, name = "env")]
/// Environment variable management.
pub struct EnvCmd {
    #[argh(subcommand)]
    pub cmd: EnvSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvSubCommand {
    Status(EnvStatusCmd),
    List(EnvListCmd),
    Search(EnvSearchCmd),
    Get(EnvGetCmd),
    Set(EnvSetCmd),
    Del(EnvDelCmd),
    Check(EnvCheckCmd),
    Path(EnvPathCmd),
    PathDedup(EnvPathDedupCmd),
    Snapshot(EnvSnapshotCmd),
    Doctor(EnvDoctorCmd),
    Profile(EnvProfileCmd),
    Batch(EnvBatchCmd),
    Apply(EnvApplyCmd),
    Export(EnvExportCmd),
    ExportAll(EnvExportAllCmd),
    ExportLive(EnvExportLiveCmd),
    Env(EnvMergedCmd),
    Import(EnvImportCmd),
    DiffLive(EnvDiffLiveCmd),
    Graph(EnvGraphCmd),
    Validate(EnvValidateCmd),
    Schema(EnvSchemaCmd),
    Annotate(EnvAnnotateCmd),
    Config(EnvConfigCmd),
    Audit(EnvAuditCmd),
    Watch(EnvWatchCmd),
    Template(EnvTemplateCmd),
    Run(EnvRunCmd),
    Tui(EnvTuiCmd),
}
