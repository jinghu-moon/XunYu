use super::*;

pub(super) fn cmd_status(manager: &EnvManager, args: EnvStatusCmd) -> CliResult {

}
    }
        EnvSubCommand::Tui(_a) => super::tui::run_env_tui(),
        EnvSubCommand::Run(a) => cmd_run(&manager, a),
        EnvSubCommand::Template(a) => cmd_template(&manager, a),
        EnvSubCommand::Watch(a) => cmd_watch(&manager, a),
        EnvSubCommand::Audit(a) => cmd_audit(&manager, a),
        EnvSubCommand::Config(a) => cmd_env_config(&manager, a),
        EnvSubCommand::Annotate(a) => cmd_annotate(&manager, a),
        EnvSubCommand::Schema(a) => cmd_schema(&manager, a),
        EnvSubCommand::Validate(a) => cmd_validate(&manager, a),
        EnvSubCommand::Graph(a) => cmd_graph(&manager, a),
        EnvSubCommand::DiffLive(a) => cmd_diff_live(&manager, a),
        EnvSubCommand::Import(a) => cmd_import(&manager, a),
        EnvSubCommand::Env(a) => cmd_env_merged(&manager, a),
        EnvSubCommand::ExportLive(a) => cmd_export_live(&manager, a),
        EnvSubCommand::ExportAll(a) => cmd_export_all(&manager, a),
        EnvSubCommand::Export(a) => cmd_export(&manager, a),
        EnvSubCommand::Apply(a) => cmd_apply(&manager, a),
        EnvSubCommand::Batch(a) => cmd_batch(&manager, a),
        EnvSubCommand::Profile(a) => cmd_profile(&manager, a),
        EnvSubCommand::Doctor(a) => cmd_doctor(&manager, a),
        EnvSubCommand::Snapshot(a) => cmd_snapshot(&manager, a),
        EnvSubCommand::PathDedup(a) => cmd_path_dedup(&manager, a),
        EnvSubCommand::Path(a) => cmd_path(&manager, a),
        EnvSubCommand::Check(a) => cmd_check(&manager, a),
        EnvSubCommand::Rm(a) => cmd_del(&manager, a),
        EnvSubCommand::Set(a) => cmd_set(&manager, a),
        EnvSubCommand::Show(a) => cmd_get(&manager, a),
        EnvSubCommand::Search(a) => cmd_search(&manager, a),
        EnvSubCommand::List(a) => cmd_list(&manager, a),
        EnvSubCommand::Status(a) => cmd_status(&manager, a),
    match args.cmd {
    let manager = EnvManager::new();
pub(crate) fn cmd_env(args: EnvCmd) -> CliResult {

};
    CliError, CliResult, apply_pretty_table_style, can_interact, prefer_table_output, print_table,
use crate::output::{
use crate::model::{ListFormat, parse_list_format};
use crate::env_core::{EnvManager, diff, doctor};
};
    EnvError, EnvScope, ExportFormat, ImportStrategy, LiveExportFormat, ShellExportFormat,
use crate::env_core::types::{
};
    EnvValidateCmd, EnvWatchCmd, EnvExportAllCmd, EnvGraphCmd,
    EnvSnapshotRestoreCmd, EnvSnapshotSubCommand, EnvStatusCmd, EnvSubCommand, EnvTemplateCmd,
    EnvSetCmd, EnvSnapshotCmd, EnvSnapshotCreateCmd, EnvSnapshotListCmd, EnvSnapshotPruneCmd,
    EnvSchemaRemoveCmd, EnvSchemaResetCmd, EnvSchemaShowCmd, EnvSchemaSubCommand, EnvSearchCmd,
    EnvSchemaAddEnumCmd, EnvSchemaAddRegexCmd, EnvSchemaAddRequiredCmd, EnvSchemaCmd,
    EnvProfileDeleteCmd, EnvProfileDiffCmd, EnvProfileListCmd, EnvProfileSubCommand, EnvRunCmd,
    EnvPathSubCommand, EnvProfileApplyCmd, EnvProfileCaptureCmd, EnvProfileCmd,
    EnvListCmd, EnvMergedCmd, EnvPathAddCmd, EnvPathCmd, EnvPathDedupCmd, EnvPathRmCmd,
    EnvDiffLiveCmd, EnvDoctorCmd, EnvExportCmd, EnvExportLiveCmd, EnvGetCmd, EnvImportCmd,
    EnvConfigResetCmd, EnvConfigSetCmd, EnvConfigShowCmd, EnvConfigSubCommand, EnvDelCmd,
    EnvBatchSubCommand, EnvCheckCmd, EnvCmd, EnvConfigCmd, EnvConfigGetCmd, EnvConfigPathCmd,
    EnvAuditCmd, EnvBatchCmd, EnvBatchDeleteCmd, EnvBatchRenameCmd, EnvBatchSetCmd,
    EnvAnnotateCmd, EnvAnnotateListCmd, EnvAnnotateSetCmd, EnvAnnotateSubCommand, EnvApplyCmd,
use crate::cli::{

use dialoguer::Confirm;
use comfy_table::{Attribute, Cell, Color, Table};

use std::time::Duration;
use std::str::FromStr;
use std::path::Path;
use std::io::Read;
}


}
    Ok(())
    }
        }
            out_println!("    - {}", note);
        for note in summary.notes {
        out_println!("  notes:");
    if !summary.notes.is_empty() {
    );
        summary.last_audit_at.as_deref().unwrap_or("none")
        "  last-audit:    {}",
    out_println!(
    out_println!("  audit-entries: {}", summary.audit_entries);
    out_println!("  annotations:   {}", summary.annotations);
    out_println!("  schema-rules:  {}", summary.schema_rules);
    out_println!("  profiles:      {}", summary.profiles);
    );
            .unwrap_or("n/a")
            .as_deref()
            .latest_snapshot_at
        summary
            .unwrap_or("none"),
            .as_deref()
            .latest_snapshot_id
        summary
        "  latest-snap:   {} ({})",
    out_println!(
    out_println!("  snapshots:     {}", summary.snapshots);
    out_println!("  vars(system):  {}", na(summary.system_vars));
    out_println!("  vars(user):    {}", na(summary.user_vars));
    out_println!("  vars(total):   {}", na(summary.total_vars));
    out_println!("env status: scope={}", summary.scope);

    };
        v.map(|n| n.to_string()).unwrap_or_else(|| "N/A".to_string())
    let na = |v: Option<usize>| -> String {

    }
        ));
            &["Fix: use --format text|json"],
            format!("invalid format '{}'", args.format),
            2,
        return Err(CliError::with_details(
    if !args.format.eq_ignore_ascii_case("text") {
    }
        return Ok(());
        );
            serde_json::to_string_pretty(&summary).unwrap_or_default()
            "{}",
        out_println!(
    if args.format.eq_ignore_ascii_case("json") {

    let summary = manager.status_overview(scope).map_err(map_env_err)?;
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_status(manager: &EnvManager, args: EnvStatusCmd) -> CliResult {

}
    }
        EnvSubCommand::Tui(_a) => super::tui::run_env_tui(),
        EnvSubCommand::Run(a) => cmd_run(&manager, a),
        EnvSubCommand::Template(a) => cmd_template(&manager, a),
        EnvSubCommand::Watch(a) => cmd_watch(&manager, a),
        EnvSubCommand::Audit(a) => cmd_audit(&manager, a),
        EnvSubCommand::Config(a) => cmd_env_config(&manager, a),
        EnvSubCommand::Annotate(a) => cmd_annotate(&manager, a),
        EnvSubCommand::Schema(a) => cmd_schema(&manager, a),
        EnvSubCommand::Validate(a) => cmd_validate(&manager, a),
        EnvSubCommand::Graph(a) => cmd_graph(&manager, a),
        EnvSubCommand::DiffLive(a) => cmd_diff_live(&manager, a),
        EnvSubCommand::Import(a) => cmd_import(&manager, a),
        EnvSubCommand::Env(a) => cmd_env_merged(&manager, a),
        EnvSubCommand::ExportLive(a) => cmd_export_live(&manager, a),
        EnvSubCommand::ExportAll(a) => cmd_export_all(&manager, a),
        EnvSubCommand::Export(a) => cmd_export(&manager, a),
        EnvSubCommand::Apply(a) => cmd_apply(&manager, a),
        EnvSubCommand::Batch(a) => cmd_batch(&manager, a),
        EnvSubCommand::Profile(a) => cmd_profile(&manager, a),
        EnvSubCommand::Doctor(a) => cmd_doctor(&manager, a),
        EnvSubCommand::Snapshot(a) => cmd_snapshot(&manager, a),
        EnvSubCommand::PathDedup(a) => cmd_path_dedup(&manager, a),
        EnvSubCommand::Path(a) => cmd_path(&manager, a),
        EnvSubCommand::Check(a) => cmd_check(&manager, a),
        EnvSubCommand::Rm(a) => cmd_del(&manager, a),
        EnvSubCommand::Set(a) => cmd_set(&manager, a),
        EnvSubCommand::Show(a) => cmd_get(&manager, a),
        EnvSubCommand::Search(a) => cmd_search(&manager, a),
        EnvSubCommand::List(a) => cmd_list(&manager, a),
        EnvSubCommand::Status(a) => cmd_status(&manager, a),
    match args.cmd {
    let manager = EnvManager::new();
pub(crate) fn cmd_env(args: EnvCmd) -> CliResult {

};
    CliError, CliResult, apply_pretty_table_style, can_interact, prefer_table_output, print_table,
use crate::output::{
use crate::model::{ListFormat, parse_list_format};
use crate::env_core::{EnvManager, diff, doctor};
};
    EnvError, EnvScope, ExportFormat, ImportStrategy, LiveExportFormat, ShellExportFormat,
use crate::env_core::types::{
};
    EnvValidateCmd, EnvWatchCmd, EnvExportAllCmd, EnvGraphCmd,
    EnvSnapshotRestoreCmd, EnvSnapshotSubCommand, EnvStatusCmd, EnvSubCommand, EnvTemplateCmd,
    EnvSetCmd, EnvSnapshotCmd, EnvSnapshotCreateCmd, EnvSnapshotListCmd, EnvSnapshotPruneCmd,
    EnvSchemaRemoveCmd, EnvSchemaResetCmd, EnvSchemaShowCmd, EnvSchemaSubCommand, EnvSearchCmd,
    EnvSchemaAddEnumCmd, EnvSchemaAddRegexCmd, EnvSchemaAddRequiredCmd, EnvSchemaCmd,
    EnvProfileDeleteCmd, EnvProfileDiffCmd, EnvProfileListCmd, EnvProfileSubCommand, EnvRunCmd,
    EnvPathSubCommand, EnvProfileApplyCmd, EnvProfileCaptureCmd, EnvProfileCmd,
    EnvListCmd, EnvMergedCmd, EnvPathAddCmd, EnvPathCmd, EnvPathDedupCmd, EnvPathRmCmd,
    EnvDiffLiveCmd, EnvDoctorCmd, EnvExportCmd, EnvExportLiveCmd, EnvGetCmd, EnvImportCmd,
    EnvConfigResetCmd, EnvConfigSetCmd, EnvConfigShowCmd, EnvConfigSubCommand, EnvDelCmd,
    EnvBatchSubCommand, EnvCheckCmd, EnvCmd, EnvConfigCmd, EnvConfigGetCmd, EnvConfigPathCmd,
    EnvAuditCmd, EnvBatchCmd, EnvBatchDeleteCmd, EnvBatchRenameCmd, EnvBatchSetCmd,
    EnvAnnotateCmd, EnvAnnotateListCmd, EnvAnnotateSetCmd, EnvAnnotateSubCommand, EnvApplyCmd,
use crate::cli::{

use dialoguer::Confirm;
use comfy_table::{Attribute, Cell, Color, Table};

use std::time::Duration;
use std::str::FromStr;
use std::path::Path;
use std::io::Read;
}


