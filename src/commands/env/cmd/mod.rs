#![allow(unused_imports)]

mod annotations;
mod batch;
mod common;
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

pub(super) use common::*;

use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use comfy_table::{Attribute, Cell, Color, Table};
use dialoguer::Confirm;

use crate::cli::{
    EnvAnnotateCmd, EnvAnnotateListCmd, EnvAnnotateSetCmd, EnvAnnotateSubCommand, EnvApplyCmd,
    EnvAuditCmd, EnvBatchCmd, EnvBatchDeleteCmd, EnvBatchRenameCmd, EnvBatchSetCmd,
    EnvBatchSubCommand, EnvCheckCmd, EnvCmd, EnvConfigCmd, EnvConfigGetCmd, EnvConfigPathCmd,
    EnvConfigResetCmd, EnvConfigSetCmd, EnvConfigShowCmd, EnvConfigSubCommand, EnvDelCmd,
    EnvDiffLiveCmd, EnvDoctorCmd, EnvExportAllCmd, EnvExportCmd, EnvExportLiveCmd, EnvGetCmd,
    EnvGraphCmd, EnvImportCmd, EnvListCmd, EnvMergedCmd, EnvPathAddCmd, EnvPathCmd,
    EnvPathDedupCmd, EnvPathRmCmd, EnvPathSubCommand, EnvProfileApplyCmd, EnvProfileCaptureCmd,
    EnvProfileCmd, EnvProfileDeleteCmd, EnvProfileDiffCmd, EnvProfileListCmd, EnvProfileSubCommand,
    EnvRunCmd, EnvSchemaAddEnumCmd, EnvSchemaAddRegexCmd, EnvSchemaAddRequiredCmd, EnvSchemaCmd,
    EnvSchemaRemoveCmd, EnvSchemaResetCmd, EnvSchemaShowCmd, EnvSchemaSubCommand, EnvSearchCmd,
    EnvSetCmd, EnvSnapshotCmd, EnvSnapshotCreateCmd, EnvSnapshotListCmd, EnvSnapshotPruneCmd,
    EnvSnapshotRestoreCmd, EnvSnapshotSubCommand, EnvStatusCmd, EnvSubCommand, EnvTemplateCmd,
    EnvValidateCmd, EnvWatchCmd,
};
use crate::env_core::types::{
    EnvError, EnvScope, ExportFormat, ImportStrategy, LiveExportFormat, ShellExportFormat,
};
use crate::env_core::{EnvManager, diff, doctor as env_doctor};
use crate::model::{ListFormat, parse_list_format};
use crate::output::{
    CliError, CliResult, apply_pretty_table_style, can_interact, prefer_table_output, print_table,
};

pub(crate) fn cmd_env(args: EnvCmd) -> CliResult {
    let manager = EnvManager::new();
    match args.cmd {
        EnvSubCommand::Status(a) => status::cmd_status(&manager, a),
        EnvSubCommand::List(a) => vars::cmd_list(&manager, a),
        EnvSubCommand::Search(a) => vars::cmd_search(&manager, a),
        EnvSubCommand::Get(a) => vars::cmd_get(&manager, a),
        EnvSubCommand::Set(a) => vars::cmd_set(&manager, a),
        EnvSubCommand::Del(a) => vars::cmd_del(&manager, a),
        EnvSubCommand::Check(a) => doctor::cmd_check(&manager, a),
        EnvSubCommand::Path(a) => path::cmd_path(&manager, a),
        EnvSubCommand::PathDedup(a) => path::cmd_path_dedup(&manager, a),
        EnvSubCommand::Snapshot(a) => snapshot::cmd_snapshot(&manager, a),
        EnvSubCommand::Doctor(a) => doctor::cmd_doctor(&manager, a),
        EnvSubCommand::Profile(a) => profile::cmd_profile(&manager, a),
        EnvSubCommand::Batch(a) => batch::cmd_batch(&manager, a),
        EnvSubCommand::Apply(a) => profile::cmd_apply(&manager, a),
        EnvSubCommand::Export(a) => import_export::cmd_export(&manager, a),
        EnvSubCommand::ExportAll(a) => import_export::cmd_export_all(&manager, a),
        EnvSubCommand::ExportLive(a) => import_export::cmd_export_live(&manager, a),
        EnvSubCommand::Env(a) => import_export::cmd_env_merged(&manager, a),
        EnvSubCommand::Import(a) => import_export::cmd_import(&manager, a),
        EnvSubCommand::DiffLive(a) => diff_graph::cmd_diff_live(&manager, a),
        EnvSubCommand::Graph(a) => diff_graph::cmd_graph(&manager, a),
        EnvSubCommand::Validate(a) => schema::cmd_validate(&manager, a),
        EnvSubCommand::Schema(a) => schema::cmd_schema(&manager, a),
        EnvSubCommand::Annotate(a) => annotations::cmd_annotate(&manager, a),
        EnvSubCommand::Config(a) => config::cmd_env_config(&manager, a),
        EnvSubCommand::Audit(a) => run::cmd_audit(&manager, a),
        EnvSubCommand::Watch(a) => run::cmd_watch(&manager, a),
        EnvSubCommand::Template(a) => run::cmd_template(&manager, a),
        EnvSubCommand::Run(a) => run::cmd_run(&manager, a),
        EnvSubCommand::Tui(_a) => super::tui::run_env_tui(),
    }
}
