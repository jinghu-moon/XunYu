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
    EnvDiffLiveCmd, EnvDoctorCmd, EnvExportCmd, EnvExportLiveCmd, EnvGetCmd, EnvImportCmd,
    EnvListCmd, EnvMergedCmd, EnvPathAddCmd, EnvPathCmd, EnvPathDedupCmd, EnvPathRmCmd,
    EnvPathSubCommand, EnvProfileApplyCmd, EnvProfileCaptureCmd, EnvProfileCmd,
    EnvProfileDeleteCmd, EnvProfileDiffCmd, EnvProfileListCmd, EnvProfileSubCommand, EnvRunCmd,
    EnvSchemaAddEnumCmd, EnvSchemaAddRegexCmd, EnvSchemaAddRequiredCmd, EnvSchemaCmd,
    EnvSchemaRemoveCmd, EnvSchemaResetCmd, EnvSchemaShowCmd, EnvSchemaSubCommand, EnvSearchCmd,
    EnvSetCmd, EnvSnapshotCmd, EnvSnapshotCreateCmd, EnvSnapshotListCmd, EnvSnapshotPruneCmd,
    EnvSnapshotRestoreCmd, EnvSnapshotSubCommand, EnvStatusCmd, EnvSubCommand, EnvTemplateCmd,
    EnvValidateCmd, EnvWatchCmd, EnvExportAllCmd, EnvGraphCmd,
};
use crate::env_core::types::{
    EnvError, EnvScope, ExportFormat, ImportStrategy, LiveExportFormat, ShellExportFormat,
};
use crate::env_core::{EnvManager, diff, doctor};
use crate::model::{ListFormat, parse_list_format};
use crate::output::{
    CliError, CliResult, apply_pretty_table_style, can_interact, prefer_table_output, print_table,
};

pub(crate) fn cmd_env(args: EnvCmd) -> CliResult {
    let manager = EnvManager::new();
    match args.cmd {
        EnvSubCommand::Status(a) => cmd_status(&manager, a),
        EnvSubCommand::List(a) => cmd_list(&manager, a),
        EnvSubCommand::Search(a) => cmd_search(&manager, a),
        EnvSubCommand::Get(a) => cmd_get(&manager, a),
        EnvSubCommand::Set(a) => cmd_set(&manager, a),
        EnvSubCommand::Del(a) => cmd_del(&manager, a),
        EnvSubCommand::Check(a) => cmd_check(&manager, a),
        EnvSubCommand::Path(a) => cmd_path(&manager, a),
        EnvSubCommand::PathDedup(a) => cmd_path_dedup(&manager, a),
        EnvSubCommand::Snapshot(a) => cmd_snapshot(&manager, a),
        EnvSubCommand::Doctor(a) => cmd_doctor(&manager, a),
        EnvSubCommand::Profile(a) => cmd_profile(&manager, a),
        EnvSubCommand::Batch(a) => cmd_batch(&manager, a),
        EnvSubCommand::Apply(a) => cmd_apply(&manager, a),
        EnvSubCommand::Export(a) => cmd_export(&manager, a),
        EnvSubCommand::ExportAll(a) => cmd_export_all(&manager, a),
        EnvSubCommand::ExportLive(a) => cmd_export_live(&manager, a),
        EnvSubCommand::Env(a) => cmd_env_merged(&manager, a),
        EnvSubCommand::Import(a) => cmd_import(&manager, a),
        EnvSubCommand::DiffLive(a) => cmd_diff_live(&manager, a),
        EnvSubCommand::Graph(a) => cmd_graph(&manager, a),
        EnvSubCommand::Validate(a) => cmd_validate(&manager, a),
        EnvSubCommand::Schema(a) => cmd_schema(&manager, a),
        EnvSubCommand::Annotate(a) => cmd_annotate(&manager, a),
        EnvSubCommand::Config(a) => cmd_env_config(&manager, a),
        EnvSubCommand::Audit(a) => cmd_audit(&manager, a),
        EnvSubCommand::Watch(a) => cmd_watch(&manager, a),
        EnvSubCommand::Template(a) => cmd_template(&manager, a),
        EnvSubCommand::Run(a) => cmd_run(&manager, a),
        EnvSubCommand::Tui(_a) => super::tui::run_env_tui(),
    }
}

fn cmd_status(manager: &EnvManager, args: EnvStatusCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let summary = manager.status_overview(scope).map_err(map_env_err)?;

    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        );
        return Ok(());
    }
    if !args.format.eq_ignore_ascii_case("text") {
        return Err(CliError::with_details(
            2,
            format!("invalid format '{}'", args.format),
            &["Fix: use --format text|json"],
        ));
    }

    let na = |v: Option<usize>| -> String {
        v.map(|n| n.to_string()).unwrap_or_else(|| "N/A".to_string())
    };

    out_println!("env status: scope={}", summary.scope);
    out_println!("  vars(total):   {}", na(summary.total_vars));
    out_println!("  vars(user):    {}", na(summary.user_vars));
    out_println!("  vars(system):  {}", na(summary.system_vars));
    out_println!("  snapshots:     {}", summary.snapshots);
    out_println!(
        "  latest-snap:   {} ({})",
        summary
            .latest_snapshot_id
            .as_deref()
            .unwrap_or("none"),
        summary
            .latest_snapshot_at
            .as_deref()
            .unwrap_or("n/a")
    );
    out_println!("  profiles:      {}", summary.profiles);
    out_println!("  schema-rules:  {}", summary.schema_rules);
    out_println!("  annotations:   {}", summary.annotations);
    out_println!("  audit-entries: {}", summary.audit_entries);
    out_println!(
        "  last-audit:    {}",
        summary.last_audit_at.as_deref().unwrap_or("none")
    );
    if !summary.notes.is_empty() {
        out_println!("  notes:");
        for note in summary.notes {
            out_println!("    - {}", note);
        }
    }
    Ok(())
}

fn cmd_list(manager: &EnvManager, args: EnvListCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let vars = manager.list_vars(scope).map_err(map_env_err)?;
    let format = parse_format(&args.format)?;
    match format {
        ListFormat::Json => out_println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "scope": scope,
                "vars": vars
            }))
            .unwrap_or_else(|_| "[]".to_string())
        ),
        ListFormat::Tsv => {
            for v in vars {
                out_println!("{}\t{}\t{}\t{}", v.scope, v.name, v.reg_type, v.raw_value);
            }
        }
        ListFormat::Table | ListFormat::Auto => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            table.set_header(vec![
                Cell::new("Scope")
                    .fg(Color::Cyan)
                    .add_attribute(Attribute::Bold),
                Cell::new("Name")
                    .fg(Color::Green)
                    .add_attribute(Attribute::Bold),
                Cell::new("Type")
                    .fg(Color::Yellow)
                    .add_attribute(Attribute::Bold),
                Cell::new("Value")
                    .fg(Color::Magenta)
                    .add_attribute(Attribute::Bold),
            ]);
            for v in vars {
                table.add_row(vec![
                    Cell::new(v.scope.to_string()),
                    Cell::new(v.name),
                    Cell::new(v.reg_type),
                    Cell::new(v.raw_value),
                ]);
            }
            print_table(&table);
        }
    }
    Ok(())
}

fn cmd_search(manager: &EnvManager, args: EnvSearchCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let vars = manager
        .search_vars(scope, &args.query)
        .map_err(map_env_err)?;
    let format = parse_format(&args.format)?;
    match format {
        ListFormat::Json => out_println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "scope": scope,
                "query": args.query,
                "vars": vars
            }))
            .unwrap_or_else(|_| "[]".to_string())
        ),
        ListFormat::Tsv => {
            for v in vars {
                out_println!("{}\t{}\t{}\t{}", v.scope, v.name, v.reg_type, v.raw_value);
            }
        }
        ListFormat::Table | ListFormat::Auto => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            table.set_header(vec![
                Cell::new("Scope")
                    .fg(Color::Cyan)
                    .add_attribute(Attribute::Bold),
                Cell::new("Name")
                    .fg(Color::Green)
                    .add_attribute(Attribute::Bold),
                Cell::new("Type")
                    .fg(Color::Yellow)
                    .add_attribute(Attribute::Bold),
                Cell::new("Value")
                    .fg(Color::Magenta)
                    .add_attribute(Attribute::Bold),
            ]);
            for v in vars {
                table.add_row(vec![
                    Cell::new(v.scope.to_string()),
                    Cell::new(v.name),
                    Cell::new(v.reg_type),
                    Cell::new(v.raw_value),
                ]);
            }
            print_table(&table);
        }
    }
    Ok(())
}

fn cmd_get(manager: &EnvManager, args: EnvGetCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    let value = manager.get_var(scope, &args.name).map_err(map_env_err)?;
    let Some(v) = value else {
        return Err(CliError::new(
            4,
            format!("environment variable not found: {}", args.name),
        ));
    };
    match parse_format(&args.format)? {
        ListFormat::Json => {
            out_println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
        }
        ListFormat::Tsv => {
            out_println!("{}\t{}\t{}", v.name, v.reg_type, v.raw_value);
        }
        _ => {
            out_println!("{}={}", v.name, v.raw_value);
        }
    }
    Ok(())
}

fn cmd_set(manager: &EnvManager, args: EnvSetCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    manager
        .set_var(scope, &args.name, &args.value, args.no_snapshot)
        .map_err(map_env_err)?;
    out_println!("ok\tset\t{}\t{}", scope, args.name);
    Ok(())
}

fn cmd_del(manager: &EnvManager, args: EnvDelCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    if !prompt_confirm(
        &format!(
            "Delete env var '{}' from {} scope? This operation is destructive.",
            args.name, scope
        ),
        args.yes,
    )? {
        return Err(CliError::new(2, "operation canceled"));
    }
    let deleted = manager.delete_var(scope, &args.name).map_err(map_env_err)?;
    if deleted {
        out_println!("ok\tdelete\t{}\t{}", scope, args.name);
    } else {
        out_println!("skip\tdelete\t{}\t{} (not found)", scope, args.name);
    }
    Ok(())
}

fn cmd_check(manager: &EnvManager, args: EnvCheckCmd) -> CliResult {
    cmd_doctor_like(manager, args.scope, args.fix, args.format, true)
}

fn cmd_path(manager: &EnvManager, args: EnvPathCmd) -> CliResult {
    match args.cmd {
        EnvPathSubCommand::Add(a) => cmd_path_add(manager, a),
        EnvPathSubCommand::Rm(a) => cmd_path_rm(manager, a),
    }
}

fn cmd_path_add(manager: &EnvManager, args: EnvPathAddCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    let head = if args.tail { false } else { args.head };
    let changed = manager
        .path_add(scope, &args.entry, head)
        .map_err(map_env_err)?;
    if changed {
        out_println!(
            "ok\tpath.add\t{}\t{}\t{}",
            scope,
            if head { "head" } else { "tail" },
            args.entry
        );
    } else {
        out_println!("skip\tpath.add\t{}\t{}", scope, args.entry);
    }
    Ok(())
}

fn cmd_path_rm(manager: &EnvManager, args: EnvPathRmCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    let changed = manager
        .path_remove(scope, &args.entry)
        .map_err(map_env_err)?;
    if changed {
        out_println!("ok\tpath.rm\t{}\t{}", scope, args.entry);
    } else {
        out_println!("skip\tpath.rm\t{}\t{}", scope, args.entry);
    }
    Ok(())
}

fn cmd_path_dedup(manager: &EnvManager, args: EnvPathDedupCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    let result = manager
        .path_dedup(scope, args.remove_missing, args.dry_run)
        .map_err(map_env_err)?;
    out_println!(
        "ok\tpath-dedup\tdry_run={}\tremoved={}\tskipped={}",
        result.dry_run,
        result.deleted,
        result.skipped
    );
    for line in result.details {
        out_println!("  - {}", line);
    }
    Ok(())
}

fn cmd_snapshot(manager: &EnvManager, args: EnvSnapshotCmd) -> CliResult {
    match args.cmd {
        EnvSnapshotSubCommand::Create(a) => cmd_snapshot_create(manager, a),
        EnvSnapshotSubCommand::List(a) => cmd_snapshot_list(manager, a),
        EnvSnapshotSubCommand::Restore(a) => cmd_snapshot_restore(manager, a),
        EnvSnapshotSubCommand::Prune(a) => cmd_snapshot_prune(manager, a),
    }
}

fn cmd_snapshot_create(manager: &EnvManager, args: EnvSnapshotCreateCmd) -> CliResult {
    let meta = manager
        .snapshot_create(args.desc.as_deref())
        .map_err(map_env_err)?;
    out_println!("ok\tsnapshot.create\t{}\t{}", meta.id, meta.description);
    Ok(())
}

fn cmd_snapshot_list(manager: &EnvManager, args: EnvSnapshotListCmd) -> CliResult {
    let snapshots = manager.snapshot_list().map_err(map_env_err)?;
    match parse_format(&args.format)? {
        ListFormat::Json => {
            out_println!(
                "{}",
                serde_json::to_string_pretty(&snapshots).unwrap_or_default()
            );
        }
        ListFormat::Tsv => {
            for s in snapshots {
                out_println!("{}\t{}\t{}", s.id, s.created_at, s.description);
            }
        }
        _ => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            table.set_header(vec![
                Cell::new("ID")
                    .fg(Color::Cyan)
                    .add_attribute(Attribute::Bold),
                Cell::new("Created")
                    .fg(Color::Green)
                    .add_attribute(Attribute::Bold),
                Cell::new("Description")
                    .fg(Color::Yellow)
                    .add_attribute(Attribute::Bold),
            ]);
            for s in snapshots {
                table.add_row(vec![
                    Cell::new(s.id),
                    Cell::new(s.created_at),
                    Cell::new(s.description),
                ]);
            }
            print_table(&table);
        }
    }
    Ok(())
}

fn cmd_snapshot_restore(manager: &EnvManager, args: EnvSnapshotRestoreCmd) -> CliResult {
    if args.id.is_none() && !args.latest {
        return Err(CliError::with_details(
            2,
            "restore requires --id <id> or --latest".to_string(),
            &["Fix: xun env snapshot restore --latest -y"],
        ));
    }
    let scope = parse_scope(&args.scope)?;
    if !prompt_confirm(
        &format!(
            "Restore snapshot ({}) into {} scope? Existing values will be replaced.",
            args.id.clone().unwrap_or_else(|| "latest".to_string()),
            scope
        ),
        args.yes,
    )? {
        return Err(CliError::new(2, "operation canceled"));
    }
    let restored = manager
        .snapshot_restore(scope, args.id.as_deref(), args.latest)
        .map_err(map_env_err)?;
    out_println!("ok\tsnapshot.restore\t{}\t{}", restored.id, scope);
    Ok(())
}

fn cmd_snapshot_prune(manager: &EnvManager, args: EnvSnapshotPruneCmd) -> CliResult {
    let removed = manager.snapshot_prune(args.keep).map_err(map_env_err)?;
    let remaining = manager.snapshot_list().map_err(map_env_err)?.len();
    out_println!(
        "ok\tsnapshot.prune\tkeep={}\tremoved={}\tremaining={}",
        args.keep,
        removed,
        remaining
    );
    Ok(())
}

fn cmd_doctor(manager: &EnvManager, args: EnvDoctorCmd) -> CliResult {
    cmd_doctor_like(manager, args.scope, args.fix, args.format, false)
}

fn cmd_doctor_like(
    manager: &EnvManager,
    scope_raw: String,
    fix: bool,
    format: String,
    use_check_alias: bool,
) -> CliResult {
    let scope = parse_scope(&scope_raw)?;
    if fix {
        let fixed = manager.doctor_fix(scope).map_err(map_env_err)?;
        if format.eq_ignore_ascii_case("json") {
            out_println!(
                "{}",
                serde_json::to_string_pretty(&fixed).unwrap_or_default()
            );
        } else {
            out_println!("doctor fixed: {} item(s)", fixed.fixed);
            for line in fixed.details {
                out_println!("  - {}", line);
            }
        }
        return Ok(());
    }
    let report = if use_check_alias {
        manager.check_run(scope).map_err(map_env_err)?
    } else {
        manager.doctor_run(scope).map_err(map_env_err)?
    };
    if format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
    } else {
        out_println!("{}", doctor::report_text(&report));
    }
    if !format.eq_ignore_ascii_case("json") {
        let code = doctor::doctor_exit_code(&report);
        if code > 0 {
            return Err(CliError::new(code, "doctor reported issues"));
        }
    }
    Ok(())
}

fn cmd_profile(manager: &EnvManager, args: EnvProfileCmd) -> CliResult {
    match args.cmd {
        EnvProfileSubCommand::List(a) => cmd_profile_list(manager, a),
        EnvProfileSubCommand::Capture(a) => cmd_profile_capture(manager, a),
        EnvProfileSubCommand::Apply(a) => cmd_profile_apply(manager, a),
        EnvProfileSubCommand::Diff(a) => cmd_profile_diff(manager, a),
        EnvProfileSubCommand::Delete(a) => cmd_profile_delete(manager, a),
    }
}

fn cmd_profile_list(manager: &EnvManager, args: EnvProfileListCmd) -> CliResult {
    let profiles = manager.profile_list().map_err(map_env_err)?;
    match parse_format(&args.format)? {
        ListFormat::Json => {
            out_println!(
                "{}",
                serde_json::to_string_pretty(&profiles).unwrap_or_default()
            );
        }
        ListFormat::Tsv => {
            for p in profiles {
                out_println!("{}\t{}\t{}\t{}", p.name, p.scope, p.created_at, p.var_count);
            }
        }
        _ => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            table.set_header(vec![
                Cell::new("Name")
                    .fg(Color::Cyan)
                    .add_attribute(Attribute::Bold),
                Cell::new("Scope")
                    .fg(Color::Green)
                    .add_attribute(Attribute::Bold),
                Cell::new("Created")
                    .fg(Color::Yellow)
                    .add_attribute(Attribute::Bold),
                Cell::new("Vars")
                    .fg(Color::Magenta)
                    .add_attribute(Attribute::Bold),
            ]);
            for p in profiles {
                table.add_row(vec![
                    Cell::new(p.name),
                    Cell::new(p.scope.to_string()),
                    Cell::new(p.created_at),
                    Cell::new(p.var_count),
                ]);
            }
            print_table(&table);
        }
    }
    Ok(())
}

fn cmd_profile_capture(manager: &EnvManager, args: EnvProfileCaptureCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    let meta = manager
        .profile_capture(&args.name, scope)
        .map_err(map_env_err)?;
    out_println!(
        "ok\tprofile.capture\t{}\t{}\t{} vars",
        meta.name,
        meta.scope,
        meta.var_count
    );
    Ok(())
}

fn cmd_profile_apply(manager: &EnvManager, args: EnvProfileApplyCmd) -> CliResult {
    let scope_override = args
        .scope
        .as_deref()
        .map(parse_writable_scope)
        .transpose()?;
    if !prompt_confirm(
        &format!(
            "Apply profile '{}'{} ?",
            args.name,
            scope_override
                .map(|s| format!(" to {}", s))
                .unwrap_or_default()
        ),
        args.yes,
    )? {
        return Err(CliError::new(2, "operation canceled"));
    }
    let meta = manager
        .profile_apply(&args.name, scope_override)
        .map_err(map_env_err)?;
    out_println!(
        "ok\tprofile.apply\t{}\t{}\t{} vars",
        meta.name,
        meta.scope,
        meta.var_count
    );
    Ok(())
}

fn cmd_profile_diff(manager: &EnvManager, args: EnvProfileDiffCmd) -> CliResult {
    let scope_override = args
        .scope
        .as_deref()
        .map(parse_writable_scope)
        .transpose()?;
    let diff = manager
        .profile_diff(&args.name, scope_override)
        .map_err(map_env_err)?;
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&diff).unwrap_or_default()
        );
    } else {
        out_println!("{}", diff::format_diff(&diff, true));
    }
    Ok(())
}

fn cmd_profile_delete(manager: &EnvManager, args: EnvProfileDeleteCmd) -> CliResult {
    if !prompt_confirm(&format!("Delete profile '{}' ?", args.name), args.yes)? {
        return Err(CliError::new(2, "operation canceled"));
    }
    let deleted = manager.profile_delete(&args.name).map_err(map_env_err)?;
    if deleted {
        out_println!("ok\tprofile.delete\t{}", args.name);
    } else {
        out_println!("skip\tprofile.delete\t{} (not found)", args.name);
    }
    Ok(())
}

fn cmd_batch(manager: &EnvManager, args: EnvBatchCmd) -> CliResult {
    match args.cmd {
        EnvBatchSubCommand::Set(a) => cmd_batch_set(manager, a),
        EnvBatchSubCommand::Delete(a) => cmd_batch_delete(manager, a),
        EnvBatchSubCommand::Rename(a) => cmd_batch_rename(manager, a),
    }
}

fn cmd_batch_set(manager: &EnvManager, args: EnvBatchSetCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    if args.items.is_empty() {
        return Err(CliError::new(
            2,
            "batch set requires at least one KEY=VALUE item",
        ));
    }
    let mut parsed = Vec::new();
    for item in args.items {
        let mut parts = item.splitn(2, '=');
        let Some(name) = parts.next() else { continue };
        let Some(value) = parts.next() else {
            return Err(CliError::new(
                2,
                format!("invalid item '{}', expected KEY=VALUE", item),
            ));
        };
        parsed.push((name.to_string(), value.to_string()));
    }
    let result = manager
        .batch_set(scope, &parsed, args.dry_run)
        .map_err(map_env_err)?;
    out_println!(
        "ok\tbatch.set\tdry_run={}\tadded={}\tupdated={}\tskipped={}",
        result.dry_run,
        result.added,
        result.updated,
        result.skipped
    );
    for line in result.details {
        out_println!("  - {}", line);
    }
    Ok(())
}

fn cmd_batch_delete(manager: &EnvManager, args: EnvBatchDeleteCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    if args.names.is_empty() {
        return Err(CliError::new(2, "batch delete requires at least one name"));
    }
    let result = manager
        .batch_delete(scope, &args.names, args.dry_run)
        .map_err(map_env_err)?;
    out_println!(
        "ok\tbatch.delete\tdry_run={}\tdeleted={}\tskipped={}",
        result.dry_run,
        result.deleted,
        result.skipped
    );
    for line in result.details {
        out_println!("  - {}", line);
    }
    Ok(())
}

fn cmd_batch_rename(manager: &EnvManager, args: EnvBatchRenameCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    let result = manager
        .batch_rename(scope, &args.old, &args.new, args.dry_run)
        .map_err(map_env_err)?;
    out_println!(
        "ok\tbatch.rename\tdry_run={}\trenamed={}\tskipped={}",
        result.dry_run,
        result.renamed,
        result.skipped
    );
    for line in result.details {
        out_println!("  - {}", line);
    }
    Ok(())
}

fn cmd_apply(manager: &EnvManager, args: EnvApplyCmd) -> CliResult {
    let scope_override = args
        .scope
        .as_deref()
        .map(parse_writable_scope)
        .transpose()?;
    if !prompt_confirm(
        &format!(
            "Apply profile '{}'{} ?",
            args.name,
            scope_override
                .map(|s| format!(" to {}", s))
                .unwrap_or_default()
        ),
        args.yes,
    )? {
        return Err(CliError::new(2, "operation canceled"));
    }
    let meta = manager
        .profile_apply(&args.name, scope_override)
        .map_err(map_env_err)?;
    out_println!(
        "ok\tapply\t{}\t{}\t{} vars",
        meta.name,
        meta.scope,
        meta.var_count
    );
    Ok(())
}

fn cmd_export(manager: &EnvManager, args: EnvExportCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let format = ExportFormat::from_str(&args.format).map_err(map_env_err)?;
    let data = manager.export_vars(scope, format).map_err(map_env_err)?;
    if let Some(out_path) = args.out {
        std::fs::write(&out_path, data).map_err(|e| CliError::new(1, format!("{e}")))?;
        out_println!("ok\texport\t{}", out_path);
    } else {
        out_println!("{}", data);
    }
    Ok(())
}

fn cmd_export_all(manager: &EnvManager, args: EnvExportAllCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let data = manager.export_bundle(scope).map_err(map_env_err)?;
    let out_path = args
        .out
        .unwrap_or_else(|| format!("xun-env-{}.zip", scope));
    std::fs::write(&out_path, &data).map_err(|e| CliError::new(1, format!("{e}")))?;
    out_println!("ok\texport-all\t{}\t{} bytes", out_path, data.len());
    Ok(())
}

fn cmd_export_live(manager: &EnvManager, args: EnvExportLiveCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let format = LiveExportFormat::from_str(&args.format).map_err(map_env_err)?;
    let env_files = args
        .env_files
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
    let set_pairs = parse_key_value_items(&args.set, "--set")?;
    let data = manager
        .export_live(scope, format, &env_files, &set_pairs)
        .map_err(map_env_err)?;
    if let Some(out_path) = args.out {
        std::fs::write(&out_path, data).map_err(|e| CliError::new(1, format!("{e}")))?;
        out_println!("ok\texport-live\t{}", out_path);
    } else {
        out_println!("{}", data);
    }
    Ok(())
}

fn cmd_env_merged(manager: &EnvManager, args: EnvMergedCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let env_files = args
        .env_files
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
    let set_pairs = parse_key_value_items(&args.set, "--set")?;
    let pairs = manager
        .merged_env_pairs(scope, &env_files, &set_pairs)
        .map_err(map_env_err)?;

    if args.format.eq_ignore_ascii_case("json") {
        let map = pairs
            .into_iter()
            .collect::<std::collections::BTreeMap<String, String>>();
        out_println!("{}", serde_json::to_string_pretty(&map).unwrap_or_default());
        return Ok(());
    }
    if !args.format.eq_ignore_ascii_case("text") {
        return Err(CliError::with_details(
            2,
            format!("invalid format '{}'", args.format),
            &["Fix: use --format text|json"],
        ));
    }
    for (name, value) in pairs {
        out_println!("{}={}", name, value);
    }
    Ok(())
}

fn cmd_validate(manager: &EnvManager, args: EnvValidateCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let report = manager
        .validate_schema(scope, args.strict)
        .map_err(map_env_err)?;
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
    } else {
        out_println!(
            "validate: scope={} vars={} errors={} warnings={}",
            report.scope,
            report.total_vars,
            report.errors,
            report.warnings
        );
        for item in &report.violations {
            let name = item.name.clone().unwrap_or_else(|| "-".to_string());
            out_println!(
                "  [{}] {} {} => {}",
                item.severity,
                item.pattern,
                name,
                item.message
            );
        }
    }
    if report.errors > 0 {
        return Err(CliError::new(1, "schema validation failed"));
    }
    if report.warnings > 0 {
        return Err(CliError::new(2, "schema validation has warnings"));
    }
    Ok(())
}

fn cmd_schema(manager: &EnvManager, args: EnvSchemaCmd) -> CliResult {
    match args.cmd {
        EnvSchemaSubCommand::Show(a) => cmd_schema_show(manager, a),
        EnvSchemaSubCommand::AddRequired(a) => cmd_schema_add_required(manager, a),
        EnvSchemaSubCommand::AddRegex(a) => cmd_schema_add_regex(manager, a),
        EnvSchemaSubCommand::AddEnum(a) => cmd_schema_add_enum(manager, a),
        EnvSchemaSubCommand::Remove(a) => cmd_schema_remove(manager, a),
        EnvSchemaSubCommand::Reset(a) => cmd_schema_reset(manager, a),
    }
}

fn cmd_schema_show(manager: &EnvManager, args: EnvSchemaShowCmd) -> CliResult {
    let schema = manager.schema_show().map_err(map_env_err)?;
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&schema).unwrap_or_default()
        );
        return Ok(());
    }
    out_println!("schema rules={}", schema.rules.len());
    for rule in schema.rules {
        let mut flags = Vec::new();
        if rule.required {
            flags.push("required".to_string());
        }
        if let Some(regex) = rule.regex {
            flags.push(format!("regex={}", regex));
        }
        if !rule.enum_values.is_empty() {
            flags.push(format!("enum=[{}]", rule.enum_values.join(",")));
        }
        if rule.warn_only {
            flags.push("warn_only".to_string());
        }
        out_println!(
            "  {} => {}",
            rule.pattern,
            if flags.is_empty() {
                "(no constraints)".to_string()
            } else {
                flags.join(", ")
            }
        );
    }
    Ok(())
}

fn cmd_schema_add_required(manager: &EnvManager, args: EnvSchemaAddRequiredCmd) -> CliResult {
    let schema = manager
        .schema_add_required(&args.pattern, args.warn_only)
        .map_err(map_env_err)?;
    out_println!(
        "ok\tschema.add-required\t{}\trules={}",
        args.pattern,
        schema.rules.len()
    );
    Ok(())
}

fn cmd_schema_add_regex(manager: &EnvManager, args: EnvSchemaAddRegexCmd) -> CliResult {
    let schema = manager
        .schema_add_regex(&args.pattern, &args.regex, args.warn_only)
        .map_err(map_env_err)?;
    out_println!(
        "ok\tschema.add-regex\t{}\trules={}",
        args.pattern,
        schema.rules.len()
    );
    Ok(())
}

fn cmd_schema_add_enum(manager: &EnvManager, args: EnvSchemaAddEnumCmd) -> CliResult {
    if args.values.is_empty() {
        return Err(CliError::new(2, "schema add-enum requires values"));
    }
    let schema = manager
        .schema_add_enum(&args.pattern, &args.values, args.warn_only)
        .map_err(map_env_err)?;
    out_println!(
        "ok\tschema.add-enum\t{}\trules={}",
        args.pattern,
        schema.rules.len()
    );
    Ok(())
}

fn cmd_schema_remove(manager: &EnvManager, args: EnvSchemaRemoveCmd) -> CliResult {
    let schema = manager.schema_remove(&args.pattern).map_err(map_env_err)?;
    out_println!(
        "ok\tschema.remove\t{}\trules={}",
        args.pattern,
        schema.rules.len()
    );
    Ok(())
}

fn cmd_schema_reset(manager: &EnvManager, args: EnvSchemaResetCmd) -> CliResult {
    if !prompt_confirm("Reset all schema rules?", args.yes)? {
        return Err(CliError::new(2, "operation canceled"));
    }
    let schema = manager.schema_reset().map_err(map_env_err)?;
    out_println!("ok\tschema.reset\trules={}", schema.rules.len());
    Ok(())
}

fn cmd_annotate(manager: &EnvManager, args: EnvAnnotateCmd) -> CliResult {
    match args.cmd {
        EnvAnnotateSubCommand::Set(a) => cmd_annotate_set(manager, a),
        EnvAnnotateSubCommand::List(a) => cmd_annotate_list(manager, a),
    }
}

fn cmd_annotate_set(manager: &EnvManager, args: EnvAnnotateSetCmd) -> CliResult {
    let item = manager
        .annotate_set(&args.name, &args.note)
        .map_err(map_env_err)?;
    out_println!("ok\tannotate.set\t{}\t{}", item.name, item.note);
    Ok(())
}

fn cmd_annotate_list(manager: &EnvManager, args: EnvAnnotateListCmd) -> CliResult {
    let items = manager.annotate_list().map_err(map_env_err)?;
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&items).unwrap_or_default()
        );
        return Ok(());
    }
    if items.is_empty() {
        out_println!("(empty)");
        return Ok(());
    }
    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Name")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
        Cell::new("Note")
            .fg(Color::Green)
            .add_attribute(Attribute::Bold),
    ]);
    for item in items {
        table.add_row(vec![Cell::new(item.name), Cell::new(item.note)]);
    }
    print_table(&table);
    Ok(())
}

fn cmd_env_config(manager: &EnvManager, args: EnvConfigCmd) -> CliResult {
    match args.cmd {
        EnvConfigSubCommand::Show(a) => cmd_env_config_show(manager, a),
        EnvConfigSubCommand::Path(a) => cmd_env_config_path(manager, a),
        EnvConfigSubCommand::Reset(a) => cmd_env_config_reset(manager, a),
        EnvConfigSubCommand::Get(a) => cmd_env_config_get(manager, a),
        EnvConfigSubCommand::Set(a) => cmd_env_config_set(manager, a),
    }
}

fn cmd_env_config_show(manager: &EnvManager, args: EnvConfigShowCmd) -> CliResult {
    let cfg = manager.env_config_show();
    if args.format.eq_ignore_ascii_case("json") {
        out_println!("{}", serde_json::to_string_pretty(&cfg).unwrap_or_default());
        return Ok(());
    }
    out_println!(
        "snapshot_dir={}",
        cfg.snapshot_dir
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    );
    out_println!(
        "profile_dir={}",
        cfg.profile_dir
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    );
    out_println!("max_snapshots={}", cfg.max_snapshots);
    out_println!("lock_timeout_ms={}", cfg.lock_timeout_ms);
    out_println!("stale_lock_secs={}", cfg.stale_lock_secs);
    out_println!("notify_enabled={}", cfg.notify_enabled);
    out_println!("allow_run={}", cfg.allow_run);
    out_println!("snapshot_every_secs={}", cfg.snapshot_every_secs);
    Ok(())
}

fn cmd_env_config_path(manager: &EnvManager, _args: EnvConfigPathCmd) -> CliResult {
    out_println!("{}", manager.env_config_path().display());
    Ok(())
}

fn cmd_env_config_reset(manager: &EnvManager, args: EnvConfigResetCmd) -> CliResult {
    if !prompt_confirm("Reset env config to defaults?", args.yes)? {
        return Err(CliError::new(2, "operation canceled"));
    }
    let cfg = manager.env_config_reset().map_err(map_env_err)?;
    out_println!(
        "ok\tenv.config.reset\tmax_snapshots={}\tlock_timeout_ms={}\tstale_lock_secs={}\tnotify_enabled={}\tallow_run={}\tsnapshot_every_secs={}",
        cfg.max_snapshots,
        cfg.lock_timeout_ms,
        cfg.stale_lock_secs,
        cfg.notify_enabled,
        cfg.allow_run,
        cfg.snapshot_every_secs
    );
    Ok(())
}

fn cmd_env_config_get(manager: &EnvManager, args: EnvConfigGetCmd) -> CliResult {
    let value = manager.env_config_get(&args.key).map_err(map_env_err)?;
    out_println!("{}", value);
    Ok(())
}

fn cmd_env_config_set(manager: &EnvManager, args: EnvConfigSetCmd) -> CliResult {
    manager
        .env_config_set(&args.key, &args.value)
        .map_err(map_env_err)?;
    out_println!("ok\tenv.config.set\t{}\t{}", args.key, args.value);
    Ok(())
}

fn cmd_audit(manager: &EnvManager, args: EnvAuditCmd) -> CliResult {
    let entries = manager.audit_list(args.limit).map_err(map_env_err)?;
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&entries).unwrap_or_default()
        );
        return Ok(());
    }
    for item in entries {
        out_println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            item.at,
            item.action,
            item.scope,
            item.result,
            item.name.unwrap_or_default(),
            item.message.unwrap_or_default()
        );
    }
    Ok(())
}

fn cmd_watch(manager: &EnvManager, args: EnvWatchCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let interval_ms = args.interval_ms.max(100);
    let json_mode = args.format.eq_ignore_ascii_case("json");
    let mut prev = manager.list_vars(scope).map_err(map_env_err)?;

    loop {
        std::thread::sleep(Duration::from_millis(interval_ms));
        let next = manager.list_vars(scope).map_err(map_env_err)?;
        let changes = manager.watch_diff(scope, &prev, &next);
        for item in changes {
            if json_mode {
                out_println!("{}", serde_json::to_string(&item).unwrap_or_default());
            } else {
                out_println!(
                    "{}\t{}\t{}\t{}\t{}",
                    item.at,
                    item.op,
                    item.scope,
                    item.name,
                    item.new_value.unwrap_or_default()
                );
            }
        }
        prev = next;
        if args.once {
            break;
        }
    }
    Ok(())
}

fn cmd_import(manager: &EnvManager, args: EnvImportCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    let strategy = ImportStrategy::from_str(&args.mode).map_err(map_env_err)?;
    if args.stdin && args.file.is_some() {
        return Err(CliError::with_details(
            2,
            "import does not allow file path and --stdin together".to_string(),
            &["Fix: use only one source, e.g. `xun env import --stdin --scope user`."],
        ));
    }
    if !args.stdin && args.file.is_none() {
        return Err(CliError::with_details(
            2,
            "import requires input file or --stdin".to_string(),
            &[
                "Fix: xun env import ./vars.env --scope user",
                "Fix: Get-Content ./vars.env | xun env import --stdin --scope user",
            ],
        ));
    }
    if matches!(strategy, ImportStrategy::Overwrite) && !args.dry_run {
        if !prompt_confirm(
            "Import with overwrite will replace existing variables. Continue?",
            args.yes,
        )? {
            return Err(CliError::new(2, "operation canceled"));
        }
    }
    let result = if args.stdin {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|e| CliError::new(1, format!("read stdin failed: {}", e)))?;
        if content.trim().is_empty() {
            return Err(CliError::new(2, "stdin content is empty"));
        }
        manager
            .import_content(scope, &content, strategy, args.dry_run)
            .map_err(map_env_err)?
    } else {
        let file = args.file.as_deref().unwrap_or_default();
        manager
            .import_file(scope, Path::new(file), strategy, args.dry_run)
            .map_err(map_env_err)?
    };
    out_println!(
        "ok\timport\tdry_run={}\tadded={}\tupdated={}\tskipped={}",
        result.dry_run,
        result.added,
        result.updated,
        result.skipped
    );
    Ok(())
}

fn cmd_diff_live(manager: &EnvManager, args: EnvDiffLiveCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    if args.snapshot.is_some() && args.since.is_some() {
        return Err(CliError::with_details(
            2,
            "diff-live does not allow using --snapshot and --since together".to_string(),
            &["Fix: use one baseline selector only."],
        ));
    }
    let diff = if let Some(since) = args.since.as_deref() {
        manager.diff_since(scope, since).map_err(map_env_err)?
    } else {
        manager
            .diff_live(scope, args.snapshot.as_deref())
            .map_err(map_env_err)?
    };
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&diff).unwrap_or_default()
        );
    } else {
        out_println!("{}", diff::format_diff(&diff, args.color));
    }
    Ok(())
}

fn cmd_graph(manager: &EnvManager, args: EnvGraphCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let tree = manager
        .dependency_tree(scope, &args.name, args.max_depth)
        .map_err(map_env_err)?;
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&tree).unwrap_or_default()
        );
        return Ok(());
    }
    if !args.format.eq_ignore_ascii_case("text") {
        return Err(CliError::with_details(
            2,
            format!("invalid format '{}'", args.format),
            &["Fix: use --format text|json"],
        ));
    }

    out_println!("dependency graph: scope={} root={}", tree.scope, tree.root);
    for line in &tree.lines {
        out_println!("{}", line);
    }
    if !tree.missing.is_empty() {
        out_println!("missing: {}", tree.missing.join(", "));
    }
    if !tree.cycles.is_empty() {
        out_println!("cycles:");
        for item in tree.cycles {
            out_println!("  - {}", item);
        }
    }
    Ok(())
}

fn cmd_template(manager: &EnvManager, args: EnvTemplateCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    if args.validate_only {
        let report = manager
            .template_validate(scope, &args.input)
            .map_err(map_env_err)?;
        if args.format.eq_ignore_ascii_case("json") {
            out_println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_default()
            );
        } else {
            out_println!("valid: {}", report.valid);
            if !report.references.is_empty() {
                out_println!("references: {}", report.references.join(", "));
            }
            if !report.missing.is_empty() {
                out_println!("missing: {}", report.missing.join(", "));
            }
            for path in &report.cycles {
                out_println!("cycle: {}", path.join(" -> "));
            }
        }
        if !report.valid {
            return Err(CliError::new(2, "template validation failed"));
        }
        return Ok(());
    }

    let result = manager
        .template_expand(scope, &args.input)
        .map_err(map_env_err)?;
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else {
        out_println!("{}", result.expanded);
        if !result.report.missing.is_empty() {
            out_println!("# missing: {}", result.report.missing.join(", "));
        }
        for path in &result.report.cycles {
            out_println!("# cycle: {}", path.join(" -> "));
        }
    }
    Ok(())
}

fn cmd_run(manager: &EnvManager, args: EnvRunCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let env_files = args
        .env_files
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
    let set_pairs = parse_key_value_items(&args.set, "--set")?;

    if let Some(shell_raw) = args.shell {
        let shell = ShellExportFormat::from_str(&shell_raw).map_err(map_env_err)?;
        let rendered = manager
            .render_shell_exports(scope, &env_files, &set_pairs, shell)
            .map_err(map_env_err)?;
        out_println!("{}", rendered);
        return Ok(());
    }

    if args.command.is_empty() {
        return Err(CliError::with_details(
            2,
            "run requires command tokens (recommended after --)".to_string(),
            &["Fix: xun env run -- your-command arg1 arg2"],
        ));
    }

    let result = manager
        .run_command(
            scope,
            &env_files,
            &set_pairs,
            &args.command,
            None,
            args.schema_check,
            args.notify,
            false,
            64 * 1024,
        )
        .map_err(map_env_err)?;
    if result.success {
        return Ok(());
    }
    let code = result.exit_code.unwrap_or(1);
    Err(CliError::new(
        code,
        format!("subcommand exited with non-zero status: {}", code),
    ))
}

fn parse_key_value_items(items: &[String], flag_name: &str) -> CliResult<Vec<(String, String)>> {
    let mut out = Vec::new();
    for item in items {
        let Some((name, value)) = item.split_once('=') else {
            return Err(CliError::with_details(
                2,
                format!("invalid {} item '{}'", flag_name, item),
                &[r#"Fix: use KEY=VALUE, e.g. --set JAVA_HOME=C:\Java\jdk"#],
            ));
        };
        let key = name.trim();
        if key.is_empty() {
            return Err(CliError::new(
                2,
                format!("invalid {} item '{}': empty key", flag_name, item),
            ));
        }
        out.push((key.to_string(), value.to_string()));
    }
    Ok(out)
}

fn parse_scope(raw: &str) -> CliResult<EnvScope> {
    EnvScope::from_str(raw).map_err(map_env_err)
}

fn parse_writable_scope(raw: &str) -> CliResult<EnvScope> {
    let scope = parse_scope(raw)?;
    if !scope.is_writable() {
        return Err(CliError::with_details(
            2,
            format!("scope '{}' is not writable", scope),
            &["Fix: Use --scope user|system for write operations."],
        ));
    }
    Ok(scope)
}

fn parse_format(raw: &str) -> CliResult<ListFormat> {
    let mut format = parse_list_format(raw).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("invalid format '{}'", raw),
            &["Fix: Use auto|table|tsv|json."],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }
    Ok(format)
}

fn map_env_err(err: EnvError) -> CliError {
    CliError::new(err.exit_code(), err.to_string())
}

fn prompt_confirm(prompt: &str, yes: bool) -> CliResult<bool> {
    if yes {
        return Ok(true);
    }
    if !can_interact() {
        return Err(CliError::with_details(
            2,
            "interactive confirmation required".to_string(),
            &["Fix: Run in terminal and confirm, or pass -y."],
        ));
    }
    Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
        .map_err(|e| CliError::new(1, format!("confirmation failed: {}", e)))
}
