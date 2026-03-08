use super::*;

pub(super) fn cmd_path(manager: &EnvManager, args: EnvPathCmd) -> CliResult {

}
    cmd_doctor_like(manager, args.scope, args.fix, args.format, true)
pub(super) fn cmd_check(manager: &EnvManager, args: EnvCheckCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("skip\tdelete\t{}\t{} (not found)", scope, args.name);
    } else {
        out_println!("ok\tdelete\t{}\t{}", scope, args.name);
    if deleted {
    let deleted = manager.delete_var(scope, &args.name).map_err(map_env_err)?;
    }
        return Err(CliError::new(2, "operation canceled"));
    )? {
        args.yes,
        ),
            args.name, scope
            "Delete env var '{}' from {} scope? This operation is destructive.",
        &format!(
    if !prompt_confirm(
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_del(manager: &EnvManager, args: EnvDelCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\tset\t{}\t{}", scope, args.name);
        .map_err(map_env_err)?;
        .set_var(scope, &args.name, &args.value, args.no_snapshot)
    manager
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_set(manager: &EnvManager, args: EnvSetCmd) -> CliResult {

}
    Ok(())
    }
        }
            out_println!("{}={}", v.name, v.raw_value);
        _ => {
        }
            out_println!("{}\t{}\t{}", v.name, v.reg_type, v.raw_value);
        ListFormat::Tsv => {
        }
            out_println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
        ListFormat::Json => {
    match parse_format(&args.format)? {
    };
        ));
            format!("environment variable not found: {}", args.name),
            4,
        return Err(CliError::new(
    let Some(v) = value else {
    let value = manager.get_var(scope, &args.name).map_err(map_env_err)?;
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_get(manager: &EnvManager, args: EnvGetCmd) -> CliResult {

}
    Ok(())
    }
        }
            print_table(&table);
            }
                ]);
                    Cell::new(v.raw_value),
                    Cell::new(v.reg_type),
                    Cell::new(v.name),
                    Cell::new(v.scope.to_string()),
                table.add_row(vec![
            for v in vars {
            ]);
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Magenta)
                Cell::new("Value")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Yellow)
                Cell::new("Type")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Green)
                Cell::new("Name")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Cyan)
                Cell::new("Scope")
            table.set_header(vec![
            apply_pretty_table_style(&mut table);
            let mut table = Table::new();
        ListFormat::Table | ListFormat::Auto => {
        }
            }
                out_println!("{}\t{}\t{}\t{}", v.scope, v.name, v.reg_type, v.raw_value);
            for v in vars {
        ListFormat::Tsv => {
        ),
            .unwrap_or_else(|_| "[]".to_string())
            }))
                "vars": vars
                "query": args.query,
                "scope": scope,
            serde_json::to_string_pretty(&serde_json::json!({
            "{}",
        ListFormat::Json => out_println!(
    match format {
    let format = parse_format(&args.format)?;
        .map_err(map_env_err)?;
        .search_vars(scope, &args.query)
    let vars = manager
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_search(manager: &EnvManager, args: EnvSearchCmd) -> CliResult {

}
    Ok(())
    }
        }
            print_table(&table);
            }
                ]);
                    Cell::new(v.raw_value),
                    Cell::new(v.reg_type),
                    Cell::new(v.name),
                    Cell::new(v.scope.to_string()),
                table.add_row(vec![
            for v in vars {
            ]);
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Magenta)
                Cell::new("Value")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Yellow)
                Cell::new("Type")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Green)
                Cell::new("Name")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Cyan)
                Cell::new("Scope")
            table.set_header(vec![
            apply_pretty_table_style(&mut table);
            let mut table = Table::new();
        ListFormat::Table | ListFormat::Auto => {
        }
            }
                out_println!("{}\t{}\t{}\t{}", v.scope, v.name, v.reg_type, v.raw_value);
            for v in vars {
        ListFormat::Tsv => {
        ),
            .unwrap_or_else(|_| "[]".to_string())
            }))
                "vars": vars
                "scope": scope,
            serde_json::to_string_pretty(&serde_json::json!({
            "{}",
        ListFormat::Json => out_println!(
    match format {
    let format = parse_format(&args.format)?;
    let vars = manager.list_vars(scope).map_err(map_env_err)?;
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_list(manager: &EnvManager, args: EnvListCmd) -> CliResult {

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
        EnvSubCommand::Del(a) => cmd_del(&manager, a),
        EnvSubCommand::Set(a) => cmd_set(&manager, a),
        EnvSubCommand::Get(a) => cmd_get(&manager, a),
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
        out_println!("  - {}", line);
    for line in result.details {
    );
        result.skipped
        result.deleted,
        result.dry_run,
        "ok\tpath-dedup\tdry_run={}\tremoved={}\tskipped={}",
    out_println!(
        .map_err(map_env_err)?;
        .path_dedup(scope, args.remove_missing, args.dry_run)
    let result = manager
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_path_dedup(manager: &EnvManager, args: EnvPathDedupCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("skip\tpath.rm\t{}\t{}", scope, args.entry);
    } else {
        out_println!("ok\tpath.rm\t{}\t{}", scope, args.entry);
    if changed {
        .map_err(map_env_err)?;
        .path_remove(scope, &args.entry)
    let changed = manager
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_path_rm(manager: &EnvManager, args: EnvPathRmCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("skip\tpath.add\t{}\t{}", scope, args.entry);
    } else {
        );
            args.entry
            if head { "head" } else { "tail" },
            scope,
            "ok\tpath.add\t{}\t{}\t{}",
        out_println!(
    if changed {
        .map_err(map_env_err)?;
        .path_add(scope, &args.entry, head)
    let changed = manager
    let head = if args.tail { false } else { args.head };
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_path_add(manager: &EnvManager, args: EnvPathAddCmd) -> CliResult {

}
    }
        EnvPathSubCommand::Rm(a) => cmd_path_rm(manager, a),
        EnvPathSubCommand::Add(a) => cmd_path_add(manager, a),
    match args.cmd {
pub(super) fn cmd_path(manager: &EnvManager, args: EnvPathCmd) -> CliResult {

}
    cmd_doctor_like(manager, args.scope, args.fix, args.format, true)
pub(super) fn cmd_check(manager: &EnvManager, args: EnvCheckCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("skip\tdelete\t{}\t{} (not found)", scope, args.name);
    } else {
        out_println!("ok\tdelete\t{}\t{}", scope, args.name);
    if deleted {
    let deleted = manager.delete_var(scope, &args.name).map_err(map_env_err)?;
    }
        return Err(CliError::new(2, "operation canceled"));
    )? {
        args.yes,
        ),
            args.name, scope
            "Delete env var '{}' from {} scope? This operation is destructive.",
        &format!(
    if !prompt_confirm(
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_del(manager: &EnvManager, args: EnvDelCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\tset\t{}\t{}", scope, args.name);
        .map_err(map_env_err)?;
        .set_var(scope, &args.name, &args.value, args.no_snapshot)
    manager
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_set(manager: &EnvManager, args: EnvSetCmd) -> CliResult {

}
    Ok(())
    }
        }
            out_println!("{}={}", v.name, v.raw_value);
        _ => {
        }
            out_println!("{}\t{}\t{}", v.name, v.reg_type, v.raw_value);
        ListFormat::Tsv => {
        }
            out_println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
        ListFormat::Json => {
    match parse_format(&args.format)? {
    };
        ));
            format!("environment variable not found: {}", args.name),
            4,
        return Err(CliError::new(
    let Some(v) = value else {
    let value = manager.get_var(scope, &args.name).map_err(map_env_err)?;
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_get(manager: &EnvManager, args: EnvGetCmd) -> CliResult {

}
    Ok(())
    }
        }
            print_table(&table);
            }
                ]);
                    Cell::new(v.raw_value),
                    Cell::new(v.reg_type),
                    Cell::new(v.name),
                    Cell::new(v.scope.to_string()),
                table.add_row(vec![
            for v in vars {
            ]);
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Magenta)
                Cell::new("Value")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Yellow)
                Cell::new("Type")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Green)
                Cell::new("Name")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Cyan)
                Cell::new("Scope")
            table.set_header(vec![
            apply_pretty_table_style(&mut table);
            let mut table = Table::new();
        ListFormat::Table | ListFormat::Auto => {
        }
            }
                out_println!("{}\t{}\t{}\t{}", v.scope, v.name, v.reg_type, v.raw_value);
            for v in vars {
        ListFormat::Tsv => {
        ),
            .unwrap_or_else(|_| "[]".to_string())
            }))
                "vars": vars
                "query": args.query,
                "scope": scope,
            serde_json::to_string_pretty(&serde_json::json!({
            "{}",
        ListFormat::Json => out_println!(
    match format {
    let format = parse_format(&args.format)?;
        .map_err(map_env_err)?;
        .search_vars(scope, &args.query)
    let vars = manager
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_search(manager: &EnvManager, args: EnvSearchCmd) -> CliResult {

}
    Ok(())
    }
        }
            print_table(&table);
            }
                ]);
                    Cell::new(v.raw_value),
                    Cell::new(v.reg_type),
                    Cell::new(v.name),
                    Cell::new(v.scope.to_string()),
                table.add_row(vec![
            for v in vars {
            ]);
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Magenta)
                Cell::new("Value")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Yellow)
                Cell::new("Type")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Green)
                Cell::new("Name")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Cyan)
                Cell::new("Scope")
            table.set_header(vec![
            apply_pretty_table_style(&mut table);
            let mut table = Table::new();
        ListFormat::Table | ListFormat::Auto => {
        }
            }
                out_println!("{}\t{}\t{}\t{}", v.scope, v.name, v.reg_type, v.raw_value);
            for v in vars {
        ListFormat::Tsv => {
        ),
            .unwrap_or_else(|_| "[]".to_string())
            }))
                "vars": vars
                "scope": scope,
            serde_json::to_string_pretty(&serde_json::json!({
            "{}",
        ListFormat::Json => out_println!(
    match format {
    let format = parse_format(&args.format)?;
    let vars = manager.list_vars(scope).map_err(map_env_err)?;
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_list(manager: &EnvManager, args: EnvListCmd) -> CliResult {

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
        EnvSubCommand::Del(a) => cmd_del(&manager, a),
        EnvSubCommand::Set(a) => cmd_set(&manager, a),
        EnvSubCommand::Get(a) => cmd_get(&manager, a),
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


