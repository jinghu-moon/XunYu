use super::*;

pub(super) fn cmd_annotate(manager: &EnvManager, args: EnvAnnotateCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\tschema.reset\trules={}", schema.rules.len());
    let schema = manager.schema_reset().map_err(map_env_err)?;
    }
        return Err(CliError::new(2, "operation canceled"));
    if !prompt_confirm("Reset all schema rules?", args.yes)? {
pub(super) fn cmd_schema_reset(manager: &EnvManager, args: EnvSchemaResetCmd) -> CliResult {

}
    Ok(())
    );
        schema.rules.len()
        args.pattern,
        "ok\tschema.remove\t{}\trules={}",
    out_println!(
    let schema = manager.schema_remove(&args.pattern).map_err(map_env_err)?;
pub(super) fn cmd_schema_remove(manager: &EnvManager, args: EnvSchemaRemoveCmd) -> CliResult {

}
    Ok(())
    );
        schema.rules.len()
        args.pattern,
        "ok\tschema.add-enum\t{}\trules={}",
    out_println!(
        .map_err(map_env_err)?;
        .schema_add_enum(&args.pattern, &args.values, args.warn_only)
    let schema = manager
    }
        return Err(CliError::new(2, "schema add-enum requires values"));
    if args.values.is_empty() {
pub(super) fn cmd_schema_add_enum(manager: &EnvManager, args: EnvSchemaAddEnumCmd) -> CliResult {

}
    Ok(())
    );
        schema.rules.len()
        args.pattern,
        "ok\tschema.add-regex\t{}\trules={}",
    out_println!(
        .map_err(map_env_err)?;
        .schema_add_regex(&args.pattern, &args.regex, args.warn_only)
    let schema = manager
pub(super) fn cmd_schema_add_regex(manager: &EnvManager, args: EnvSchemaAddRegexCmd) -> CliResult {

}
    Ok(())
    );
        schema.rules.len()
        args.pattern,
        "ok\tschema.add-required\t{}\trules={}",
    out_println!(
        .map_err(map_env_err)?;
        .schema_add_required(&args.pattern, args.warn_only)
    let schema = manager
pub(super) fn cmd_schema_add_required(manager: &EnvManager, args: EnvSchemaAddRequiredCmd) -> CliResult {

}
    Ok(())
    }
        );
            }
                flags.join(", ")
            } else {
                "(no constraints)".to_string()
            if flags.is_empty() {
            rule.pattern,
            "  {} => {}",
        out_println!(
        }
            flags.push("warn_only".to_string());
        if rule.warn_only {
        }
            flags.push(format!("enum=[{}]", rule.enum_values.join(",")));
        if !rule.enum_values.is_empty() {
        }
            flags.push(format!("regex={}", regex));
        if let Some(regex) = rule.regex {
        }
            flags.push("required".to_string());
        if rule.required {
        let mut flags = Vec::new();
    for rule in schema.rules {
    out_println!("schema rules={}", schema.rules.len());
    }
        return Ok(());
        );
            serde_json::to_string_pretty(&schema).unwrap_or_default()
            "{}",
        out_println!(
    if args.format.eq_ignore_ascii_case("json") {
    let schema = manager.schema_show().map_err(map_env_err)?;
pub(super) fn cmd_schema_show(manager: &EnvManager, args: EnvSchemaShowCmd) -> CliResult {

}
    }
        EnvSchemaSubCommand::Reset(a) => cmd_schema_reset(manager, a),
        EnvSchemaSubCommand::Remove(a) => cmd_schema_remove(manager, a),
        EnvSchemaSubCommand::AddEnum(a) => cmd_schema_add_enum(manager, a),
        EnvSchemaSubCommand::AddRegex(a) => cmd_schema_add_regex(manager, a),
        EnvSchemaSubCommand::AddRequired(a) => cmd_schema_add_required(manager, a),
        EnvSchemaSubCommand::Show(a) => cmd_schema_show(manager, a),
    match args.cmd {
pub(super) fn cmd_schema(manager: &EnvManager, args: EnvSchemaCmd) -> CliResult {

}
    Ok(())
    }
        return Err(CliError::new(2, "schema validation has warnings"));
    if report.warnings > 0 {
    }
        return Err(CliError::new(1, "schema validation failed"));
    if report.errors > 0 {
    }
        }
            );
                item.message
                name,
                item.pattern,
                item.severity,
                "  [{}] {} {} => {}",
            out_println!(
            let name = item.name.clone().unwrap_or_else(|| "-".to_string());
        for item in &report.violations {
        );
            report.warnings
            report.errors,
            report.total_vars,
            report.scope,
            "validate: scope={} vars={} errors={} warnings={}",
        out_println!(
    } else {
        );
            serde_json::to_string_pretty(&report).unwrap_or_default()
            "{}",
        out_println!(
    if args.format.eq_ignore_ascii_case("json") {
        .map_err(map_env_err)?;
        .validate_schema(scope, args.strict)
    let report = manager
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_validate(manager: &EnvManager, args: EnvValidateCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("{}={}", name, value);
    for (name, value) in pairs {
    }
        ));
            &["Fix: use --format text|json"],
            format!("invalid format '{}'", args.format),
            2,
        return Err(CliError::with_details(
    if !args.format.eq_ignore_ascii_case("text") {
    }
        return Ok(());
        out_println!("{}", serde_json::to_string_pretty(&map).unwrap_or_default());
            .collect::<std::collections::BTreeMap<String, String>>();
            .into_iter()
        let map = pairs
    if args.format.eq_ignore_ascii_case("json") {

        .map_err(map_env_err)?;
        .merged_env_pairs(scope, &env_files, &set_pairs)
    let pairs = manager
    let set_pairs = parse_key_value_items(&args.set, "--set")?;
        .collect::<Vec<_>>();
        .map(std::path::PathBuf::from)
        .into_iter()
        .env_files
    let env_files = args
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_env_merged(manager: &EnvManager, args: EnvMergedCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("{}", data);
    } else {
        out_println!("ok\texport-live\t{}", out_path);
        std::fs::write(&out_path, data).map_err(|e| CliError::new(1, format!("{e}")))?;
    if let Some(out_path) = args.out {
        .map_err(map_env_err)?;
        .export_live(scope, format, &env_files, &set_pairs)
    let data = manager
    let set_pairs = parse_key_value_items(&args.set, "--set")?;
        .collect::<Vec<_>>();
        .map(std::path::PathBuf::from)
        .into_iter()
        .env_files
    let env_files = args
    let format = LiveExportFormat::from_str(&args.format).map_err(map_env_err)?;
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_export_live(manager: &EnvManager, args: EnvExportLiveCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\texport-all\t{}\t{} bytes", out_path, data.len());
    std::fs::write(&out_path, &data).map_err(|e| CliError::new(1, format!("{e}")))?;
        .unwrap_or_else(|| format!("xun-env-{}.zip", scope));
        .out
    let out_path = args
    let data = manager.export_bundle(scope).map_err(map_env_err)?;
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_export_all(manager: &EnvManager, args: EnvExportAllCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("{}", data);
    } else {
        out_println!("ok\texport\t{}", out_path);
        std::fs::write(&out_path, data).map_err(|e| CliError::new(1, format!("{e}")))?;
    if let Some(out_path) = args.out {
    let data = manager.export_vars(scope, format).map_err(map_env_err)?;
    let format = ExportFormat::from_str(&args.format).map_err(map_env_err)?;
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_export(manager: &EnvManager, args: EnvExportCmd) -> CliResult {

}
    Ok(())
    );
        meta.var_count
        meta.scope,
        meta.name,
        "ok\tapply\t{}\t{}\t{} vars",
    out_println!(
        .map_err(map_env_err)?;
        .profile_apply(&args.name, scope_override)
    let meta = manager
    }
        return Err(CliError::new(2, "operation canceled"));
    )? {
        args.yes,
        ),
                .unwrap_or_default()
                .map(|s| format!(" to {}", s))
            scope_override
            args.name,
            "Apply profile '{}'{} ?",
        &format!(
    if !prompt_confirm(
        .transpose()?;
        .map(parse_writable_scope)
        .as_deref()
        .scope
    let scope_override = args
pub(super) fn cmd_apply(manager: &EnvManager, args: EnvApplyCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("  - {}", line);
    for line in result.details {
    );
        result.skipped
        result.renamed,
        result.dry_run,
        "ok\tbatch.rename\tdry_run={}\trenamed={}\tskipped={}",
    out_println!(
        .map_err(map_env_err)?;
        .batch_rename(scope, &args.old, &args.new, args.dry_run)
    let result = manager
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_batch_rename(manager: &EnvManager, args: EnvBatchRenameCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("  - {}", line);
    for line in result.details {
    );
        result.skipped
        result.deleted,
        result.dry_run,
        "ok\tbatch.delete\tdry_run={}\tdeleted={}\tskipped={}",
    out_println!(
        .map_err(map_env_err)?;
        .batch_delete(scope, &args.names, args.dry_run)
    let result = manager
    }
        return Err(CliError::new(2, "batch delete requires at least one name"));
    if args.names.is_empty() {
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_batch_delete(manager: &EnvManager, args: EnvBatchDeleteCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("  - {}", line);
    for line in result.details {
    );
        result.skipped
        result.updated,
        result.added,
        result.dry_run,
        "ok\tbatch.set\tdry_run={}\tadded={}\tupdated={}\tskipped={}",
    out_println!(
        .map_err(map_env_err)?;
        .batch_set(scope, &parsed, args.dry_run)
    let result = manager
    }
        parsed.push((name.to_string(), value.to_string()));
        };
            ));
                format!("invalid item '{}', expected KEY=VALUE", item),
                2,
            return Err(CliError::new(
        let Some(value) = parts.next() else {
        let Some(name) = parts.next() else { continue };
        let mut parts = item.splitn(2, '=');
    for item in args.items {
    let mut parsed = Vec::new();
    }
        ));
            "batch set requires at least one KEY=VALUE item",
            2,
        return Err(CliError::new(
    if args.items.is_empty() {
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_batch_set(manager: &EnvManager, args: EnvBatchSetCmd) -> CliResult {

}
    }
        EnvBatchSubCommand::Rename(a) => cmd_batch_rename(manager, a),
        EnvBatchSubCommand::Rm(a) => cmd_batch_delete(manager, a),
        EnvBatchSubCommand::Set(a) => cmd_batch_set(manager, a),
    match args.cmd {
pub(super) fn cmd_batch(manager: &EnvManager, args: EnvBatchCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("skip\tprofile.delete\t{} (not found)", args.name);
    } else {
        out_println!("ok\tprofile.delete\t{}", args.name);
    if deleted {
    let deleted = manager.profile_delete(&args.name).map_err(map_env_err)?;
    }
        return Err(CliError::new(2, "operation canceled"));
    if !prompt_confirm(&format!("Delete profile '{}' ?", args.name), args.yes)? {
pub(super) fn cmd_profile_delete(manager: &EnvManager, args: EnvProfileDeleteCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("{}", diff::format_diff(&diff, true));
    } else {
        );
            serde_json::to_string_pretty(&diff).unwrap_or_default()
            "{}",
        out_println!(
    if args.format.eq_ignore_ascii_case("json") {
        .map_err(map_env_err)?;
        .profile_diff(&args.name, scope_override)
    let diff = manager
        .transpose()?;
        .map(parse_writable_scope)
        .as_deref()
        .scope
    let scope_override = args
pub(super) fn cmd_profile_diff(manager: &EnvManager, args: EnvProfileDiffCmd) -> CliResult {

}
    Ok(())
    );
        meta.var_count
        meta.scope,
        meta.name,
        "ok\tprofile.apply\t{}\t{}\t{} vars",
    out_println!(
        .map_err(map_env_err)?;
        .profile_apply(&args.name, scope_override)
    let meta = manager
    }
        return Err(CliError::new(2, "operation canceled"));
    )? {
        args.yes,
        ),
                .unwrap_or_default()
                .map(|s| format!(" to {}", s))
            scope_override
            args.name,
            "Apply profile '{}'{} ?",
        &format!(
    if !prompt_confirm(
        .transpose()?;
        .map(parse_writable_scope)
        .as_deref()
        .scope
    let scope_override = args
pub(super) fn cmd_profile_apply(manager: &EnvManager, args: EnvProfileApplyCmd) -> CliResult {

}
    Ok(())
    );
        meta.var_count
        meta.scope,
        meta.name,
        "ok\tprofile.capture\t{}\t{}\t{} vars",
    out_println!(
        .map_err(map_env_err)?;
        .profile_capture(&args.name, scope)
    let meta = manager
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_profile_capture(manager: &EnvManager, args: EnvProfileCaptureCmd) -> CliResult {

}
    Ok(())
    }
        }
            print_table(&table);
            }
                ]);
                    Cell::new(p.var_count),
                    Cell::new(p.created_at),
                    Cell::new(p.scope.to_string()),
                    Cell::new(p.name),
                table.add_row(vec![
            for p in profiles {
            ]);
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Magenta)
                Cell::new("Vars")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Yellow)
                Cell::new("Created")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Green)
                Cell::new("Scope")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Cyan)
                Cell::new("Name")
            table.set_header(vec![
            apply_pretty_table_style(&mut table);
            let mut table = Table::new();
        _ => {
        }
            }
                out_println!("{}\t{}\t{}\t{}", p.name, p.scope, p.created_at, p.var_count);
            for p in profiles {
        ListFormat::Tsv => {
        }
            );
                serde_json::to_string_pretty(&profiles).unwrap_or_default()
                "{}",
            out_println!(
        ListFormat::Json => {
    match parse_format(&args.format)? {
    let profiles = manager.profile_list().map_err(map_env_err)?;
pub(super) fn cmd_profile_list(manager: &EnvManager, args: EnvProfileListCmd) -> CliResult {

}
    }
        EnvProfileSubCommand::Rm(a) => cmd_profile_delete(manager, a),
        EnvProfileSubCommand::Diff(a) => cmd_profile_diff(manager, a),
        EnvProfileSubCommand::Apply(a) => cmd_profile_apply(manager, a),
        EnvProfileSubCommand::Capture(a) => cmd_profile_capture(manager, a),
        EnvProfileSubCommand::List(a) => cmd_profile_list(manager, a),
    match args.cmd {
pub(super) fn cmd_profile(manager: &EnvManager, args: EnvProfileCmd) -> CliResult {

}
    Ok(())
    }
        }
            return Err(CliError::new(code, "doctor reported issues"));
        if code > 0 {
        let code = doctor::doctor_exit_code(&report);
    if !format.eq_ignore_ascii_case("json") {
    }
        out_println!("{}", doctor::report_text(&report));
    } else {
        );
            serde_json::to_string_pretty(&report).unwrap_or_default()
            "{}",
        out_println!(
    if format.eq_ignore_ascii_case("json") {
    };
        manager.doctor_run(scope).map_err(map_env_err)?
    } else {
        manager.check_run(scope).map_err(map_env_err)?
    let report = if use_check_alias {
    }
        return Ok(());
        }
            }
                out_println!("  - {}", line);
            for line in fixed.details {
            out_println!("doctor fixed: {} item(s)", fixed.fixed);
        } else {
            );
                serde_json::to_string_pretty(&fixed).unwrap_or_default()
                "{}",
            out_println!(
        if format.eq_ignore_ascii_case("json") {
        let fixed = manager.doctor_fix(scope).map_err(map_env_err)?;
    if fix {
    let scope = parse_scope(&scope_raw)?;
) -> CliResult {
    use_check_alias: bool,
    format: String,
    fix: bool,
    scope_raw: String,
    manager: &EnvManager,
pub(super) fn cmd_doctor_like(

}
    cmd_doctor_like(manager, args.scope, args.fix, args.format, false)
pub(super) fn cmd_doctor(manager: &EnvManager, args: EnvDoctorCmd) -> CliResult {

}
    Ok(())
    );
        remaining
        removed,
        args.keep,
        "ok\tsnapshot.prune\tkeep={}\tremoved={}\tremaining={}",
    out_println!(
    let remaining = manager.snapshot_list().map_err(map_env_err)?.len();
    let removed = manager.snapshot_prune(args.keep).map_err(map_env_err)?;
pub(super) fn cmd_snapshot_prune(manager: &EnvManager, args: EnvSnapshotPruneCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\tsnapshot.restore\t{}\t{}", restored.id, scope);
        .map_err(map_env_err)?;
        .snapshot_restore(scope, args.id.as_deref(), args.latest)
    let restored = manager
    }
        return Err(CliError::new(2, "operation canceled"));
    )? {
        args.yes,
        ),
            scope
            args.id.clone().unwrap_or_else(|| "latest".to_string()),
            "Restore snapshot ({}) into {} scope? Existing values will be replaced.",
        &format!(
    if !prompt_confirm(
    let scope = parse_scope(&args.scope)?;
    }
        ));
            &["Fix: xun env snapshot restore --latest -y"],
            "restore requires --id <id> or --latest".to_string(),
            2,
        return Err(CliError::with_details(
    if args.id.is_none() && !args.latest {
pub(super) fn cmd_snapshot_restore(manager: &EnvManager, args: EnvSnapshotRestoreCmd) -> CliResult {

}
    Ok(())
    }
        }
            print_table(&table);
            }
                ]);
                    Cell::new(s.description),
                    Cell::new(s.created_at),
                    Cell::new(s.id),
                table.add_row(vec![
            for s in snapshots {
            ]);
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Yellow)
                Cell::new("Description")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Green)
                Cell::new("Created")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Cyan)
                Cell::new("ID")
            table.set_header(vec![
            apply_pretty_table_style(&mut table);
            let mut table = Table::new();
        _ => {
        }
            }
                out_println!("{}\t{}\t{}", s.id, s.created_at, s.description);
            for s in snapshots {
        ListFormat::Tsv => {
        }
            );
                serde_json::to_string_pretty(&snapshots).unwrap_or_default()
                "{}",
            out_println!(
        ListFormat::Json => {
    match parse_format(&args.format)? {
    let snapshots = manager.snapshot_list().map_err(map_env_err)?;
pub(super) fn cmd_snapshot_list(manager: &EnvManager, args: EnvSnapshotListCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\tsnapshot.create\t{}\t{}", meta.id, meta.description);
        .map_err(map_env_err)?;
        .snapshot_create(args.desc.as_deref())
    let meta = manager
pub(super) fn cmd_snapshot_create(manager: &EnvManager, args: EnvSnapshotCreateCmd) -> CliResult {

}
    }
        EnvSnapshotSubCommand::Prune(a) => cmd_snapshot_prune(manager, a),
        EnvSnapshotSubCommand::Restore(a) => cmd_snapshot_restore(manager, a),
        EnvSnapshotSubCommand::List(a) => cmd_snapshot_list(manager, a),
        EnvSnapshotSubCommand::Create(a) => cmd_snapshot_create(manager, a),
    match args.cmd {
pub(super) fn cmd_snapshot(manager: &EnvManager, args: EnvSnapshotCmd) -> CliResult {

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
    print_table(&table);
    }
        table.add_row(vec![Cell::new(item.name), Cell::new(item.note)]);
    for item in items {
    ]);
            .add_attribute(Attribute::Bold),
            .fg(Color::Green)
        Cell::new("Note")
            .add_attribute(Attribute::Bold),
            .fg(Color::Cyan)
        Cell::new("Name")
    table.set_header(vec![
    apply_pretty_table_style(&mut table);
    let mut table = Table::new();
    }
        return Ok(());
        out_println!("(empty)");
    if items.is_empty() {
    }
        return Ok(());
        );
            serde_json::to_string_pretty(&items).unwrap_or_default()
            "{}",
        out_println!(
    if args.format.eq_ignore_ascii_case("json") {
    let items = manager.annotate_list().map_err(map_env_err)?;
pub(super) fn cmd_annotate_list(manager: &EnvManager, args: EnvAnnotateListCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\tannotate.set\t{}\t{}", item.name, item.note);
        .map_err(map_env_err)?;
        .annotate_set(&args.name, &args.note)
    let item = manager
pub(super) fn cmd_annotate_set(manager: &EnvManager, args: EnvAnnotateSetCmd) -> CliResult {

}
    }
        EnvAnnotateSubCommand::List(a) => cmd_annotate_list(manager, a),
        EnvAnnotateSubCommand::Set(a) => cmd_annotate_set(manager, a),
    match args.cmd {
pub(super) fn cmd_annotate(manager: &EnvManager, args: EnvAnnotateCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\tschema.reset\trules={}", schema.rules.len());
    let schema = manager.schema_reset().map_err(map_env_err)?;
    }
        return Err(CliError::new(2, "operation canceled"));
    if !prompt_confirm("Reset all schema rules?", args.yes)? {
pub(super) fn cmd_schema_reset(manager: &EnvManager, args: EnvSchemaResetCmd) -> CliResult {

}
    Ok(())
    );
        schema.rules.len()
        args.pattern,
        "ok\tschema.remove\t{}\trules={}",
    out_println!(
    let schema = manager.schema_remove(&args.pattern).map_err(map_env_err)?;
pub(super) fn cmd_schema_remove(manager: &EnvManager, args: EnvSchemaRemoveCmd) -> CliResult {

}
    Ok(())
    );
        schema.rules.len()
        args.pattern,
        "ok\tschema.add-enum\t{}\trules={}",
    out_println!(
        .map_err(map_env_err)?;
        .schema_add_enum(&args.pattern, &args.values, args.warn_only)
    let schema = manager
    }
        return Err(CliError::new(2, "schema add-enum requires values"));
    if args.values.is_empty() {
pub(super) fn cmd_schema_add_enum(manager: &EnvManager, args: EnvSchemaAddEnumCmd) -> CliResult {

}
    Ok(())
    );
        schema.rules.len()
        args.pattern,
        "ok\tschema.add-regex\t{}\trules={}",
    out_println!(
        .map_err(map_env_err)?;
        .schema_add_regex(&args.pattern, &args.regex, args.warn_only)
    let schema = manager
pub(super) fn cmd_schema_add_regex(manager: &EnvManager, args: EnvSchemaAddRegexCmd) -> CliResult {

}
    Ok(())
    );
        schema.rules.len()
        args.pattern,
        "ok\tschema.add-required\t{}\trules={}",
    out_println!(
        .map_err(map_env_err)?;
        .schema_add_required(&args.pattern, args.warn_only)
    let schema = manager
pub(super) fn cmd_schema_add_required(manager: &EnvManager, args: EnvSchemaAddRequiredCmd) -> CliResult {

}
    Ok(())
    }
        );
            }
                flags.join(", ")
            } else {
                "(no constraints)".to_string()
            if flags.is_empty() {
            rule.pattern,
            "  {} => {}",
        out_println!(
        }
            flags.push("warn_only".to_string());
        if rule.warn_only {
        }
            flags.push(format!("enum=[{}]", rule.enum_values.join(",")));
        if !rule.enum_values.is_empty() {
        }
            flags.push(format!("regex={}", regex));
        if let Some(regex) = rule.regex {
        }
            flags.push("required".to_string());
        if rule.required {
        let mut flags = Vec::new();
    for rule in schema.rules {
    out_println!("schema rules={}", schema.rules.len());
    }
        return Ok(());
        );
            serde_json::to_string_pretty(&schema).unwrap_or_default()
            "{}",
        out_println!(
    if args.format.eq_ignore_ascii_case("json") {
    let schema = manager.schema_show().map_err(map_env_err)?;
pub(super) fn cmd_schema_show(manager: &EnvManager, args: EnvSchemaShowCmd) -> CliResult {

}
    }
        EnvSchemaSubCommand::Reset(a) => cmd_schema_reset(manager, a),
        EnvSchemaSubCommand::Remove(a) => cmd_schema_remove(manager, a),
        EnvSchemaSubCommand::AddEnum(a) => cmd_schema_add_enum(manager, a),
        EnvSchemaSubCommand::AddRegex(a) => cmd_schema_add_regex(manager, a),
        EnvSchemaSubCommand::AddRequired(a) => cmd_schema_add_required(manager, a),
        EnvSchemaSubCommand::Show(a) => cmd_schema_show(manager, a),
    match args.cmd {
pub(super) fn cmd_schema(manager: &EnvManager, args: EnvSchemaCmd) -> CliResult {

}
    Ok(())
    }
        return Err(CliError::new(2, "schema validation has warnings"));
    if report.warnings > 0 {
    }
        return Err(CliError::new(1, "schema validation failed"));
    if report.errors > 0 {
    }
        }
            );
                item.message
                name,
                item.pattern,
                item.severity,
                "  [{}] {} {} => {}",
            out_println!(
            let name = item.name.clone().unwrap_or_else(|| "-".to_string());
        for item in &report.violations {
        );
            report.warnings
            report.errors,
            report.total_vars,
            report.scope,
            "validate: scope={} vars={} errors={} warnings={}",
        out_println!(
    } else {
        );
            serde_json::to_string_pretty(&report).unwrap_or_default()
            "{}",
        out_println!(
    if args.format.eq_ignore_ascii_case("json") {
        .map_err(map_env_err)?;
        .validate_schema(scope, args.strict)
    let report = manager
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_validate(manager: &EnvManager, args: EnvValidateCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("{}={}", name, value);
    for (name, value) in pairs {
    }
        ));
            &["Fix: use --format text|json"],
            format!("invalid format '{}'", args.format),
            2,
        return Err(CliError::with_details(
    if !args.format.eq_ignore_ascii_case("text") {
    }
        return Ok(());
        out_println!("{}", serde_json::to_string_pretty(&map).unwrap_or_default());
            .collect::<std::collections::BTreeMap<String, String>>();
            .into_iter()
        let map = pairs
    if args.format.eq_ignore_ascii_case("json") {

        .map_err(map_env_err)?;
        .merged_env_pairs(scope, &env_files, &set_pairs)
    let pairs = manager
    let set_pairs = parse_key_value_items(&args.set, "--set")?;
        .collect::<Vec<_>>();
        .map(std::path::PathBuf::from)
        .into_iter()
        .env_files
    let env_files = args
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_env_merged(manager: &EnvManager, args: EnvMergedCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("{}", data);
    } else {
        out_println!("ok\texport-live\t{}", out_path);
        std::fs::write(&out_path, data).map_err(|e| CliError::new(1, format!("{e}")))?;
    if let Some(out_path) = args.out {
        .map_err(map_env_err)?;
        .export_live(scope, format, &env_files, &set_pairs)
    let data = manager
    let set_pairs = parse_key_value_items(&args.set, "--set")?;
        .collect::<Vec<_>>();
        .map(std::path::PathBuf::from)
        .into_iter()
        .env_files
    let env_files = args
    let format = LiveExportFormat::from_str(&args.format).map_err(map_env_err)?;
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_export_live(manager: &EnvManager, args: EnvExportLiveCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\texport-all\t{}\t{} bytes", out_path, data.len());
    std::fs::write(&out_path, &data).map_err(|e| CliError::new(1, format!("{e}")))?;
        .unwrap_or_else(|| format!("xun-env-{}.zip", scope));
        .out
    let out_path = args
    let data = manager.export_bundle(scope).map_err(map_env_err)?;
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_export_all(manager: &EnvManager, args: EnvExportAllCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("{}", data);
    } else {
        out_println!("ok\texport\t{}", out_path);
        std::fs::write(&out_path, data).map_err(|e| CliError::new(1, format!("{e}")))?;
    if let Some(out_path) = args.out {
    let data = manager.export_vars(scope, format).map_err(map_env_err)?;
    let format = ExportFormat::from_str(&args.format).map_err(map_env_err)?;
    let scope = parse_scope(&args.scope)?;
pub(super) fn cmd_export(manager: &EnvManager, args: EnvExportCmd) -> CliResult {

}
    Ok(())
    );
        meta.var_count
        meta.scope,
        meta.name,
        "ok\tapply\t{}\t{}\t{} vars",
    out_println!(
        .map_err(map_env_err)?;
        .profile_apply(&args.name, scope_override)
    let meta = manager
    }
        return Err(CliError::new(2, "operation canceled"));
    )? {
        args.yes,
        ),
                .unwrap_or_default()
                .map(|s| format!(" to {}", s))
            scope_override
            args.name,
            "Apply profile '{}'{} ?",
        &format!(
    if !prompt_confirm(
        .transpose()?;
        .map(parse_writable_scope)
        .as_deref()
        .scope
    let scope_override = args
pub(super) fn cmd_apply(manager: &EnvManager, args: EnvApplyCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("  - {}", line);
    for line in result.details {
    );
        result.skipped
        result.renamed,
        result.dry_run,
        "ok\tbatch.rename\tdry_run={}\trenamed={}\tskipped={}",
    out_println!(
        .map_err(map_env_err)?;
        .batch_rename(scope, &args.old, &args.new, args.dry_run)
    let result = manager
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_batch_rename(manager: &EnvManager, args: EnvBatchRenameCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("  - {}", line);
    for line in result.details {
    );
        result.skipped
        result.deleted,
        result.dry_run,
        "ok\tbatch.delete\tdry_run={}\tdeleted={}\tskipped={}",
    out_println!(
        .map_err(map_env_err)?;
        .batch_delete(scope, &args.names, args.dry_run)
    let result = manager
    }
        return Err(CliError::new(2, "batch delete requires at least one name"));
    if args.names.is_empty() {
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_batch_delete(manager: &EnvManager, args: EnvBatchDeleteCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("  - {}", line);
    for line in result.details {
    );
        result.skipped
        result.updated,
        result.added,
        result.dry_run,
        "ok\tbatch.set\tdry_run={}\tadded={}\tupdated={}\tskipped={}",
    out_println!(
        .map_err(map_env_err)?;
        .batch_set(scope, &parsed, args.dry_run)
    let result = manager
    }
        parsed.push((name.to_string(), value.to_string()));
        };
            ));
                format!("invalid item '{}', expected KEY=VALUE", item),
                2,
            return Err(CliError::new(
        let Some(value) = parts.next() else {
        let Some(name) = parts.next() else { continue };
        let mut parts = item.splitn(2, '=');
    for item in args.items {
    let mut parsed = Vec::new();
    }
        ));
            "batch set requires at least one KEY=VALUE item",
            2,
        return Err(CliError::new(
    if args.items.is_empty() {
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_batch_set(manager: &EnvManager, args: EnvBatchSetCmd) -> CliResult {

}
    }
        EnvBatchSubCommand::Rename(a) => cmd_batch_rename(manager, a),
        EnvBatchSubCommand::Rm(a) => cmd_batch_delete(manager, a),
        EnvBatchSubCommand::Set(a) => cmd_batch_set(manager, a),
    match args.cmd {
pub(super) fn cmd_batch(manager: &EnvManager, args: EnvBatchCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("skip\tprofile.delete\t{} (not found)", args.name);
    } else {
        out_println!("ok\tprofile.delete\t{}", args.name);
    if deleted {
    let deleted = manager.profile_delete(&args.name).map_err(map_env_err)?;
    }
        return Err(CliError::new(2, "operation canceled"));
    if !prompt_confirm(&format!("Delete profile '{}' ?", args.name), args.yes)? {
pub(super) fn cmd_profile_delete(manager: &EnvManager, args: EnvProfileDeleteCmd) -> CliResult {

}
    Ok(())
    }
        out_println!("{}", diff::format_diff(&diff, true));
    } else {
        );
            serde_json::to_string_pretty(&diff).unwrap_or_default()
            "{}",
        out_println!(
    if args.format.eq_ignore_ascii_case("json") {
        .map_err(map_env_err)?;
        .profile_diff(&args.name, scope_override)
    let diff = manager
        .transpose()?;
        .map(parse_writable_scope)
        .as_deref()
        .scope
    let scope_override = args
pub(super) fn cmd_profile_diff(manager: &EnvManager, args: EnvProfileDiffCmd) -> CliResult {

}
    Ok(())
    );
        meta.var_count
        meta.scope,
        meta.name,
        "ok\tprofile.apply\t{}\t{}\t{} vars",
    out_println!(
        .map_err(map_env_err)?;
        .profile_apply(&args.name, scope_override)
    let meta = manager
    }
        return Err(CliError::new(2, "operation canceled"));
    )? {
        args.yes,
        ),
                .unwrap_or_default()
                .map(|s| format!(" to {}", s))
            scope_override
            args.name,
            "Apply profile '{}'{} ?",
        &format!(
    if !prompt_confirm(
        .transpose()?;
        .map(parse_writable_scope)
        .as_deref()
        .scope
    let scope_override = args
pub(super) fn cmd_profile_apply(manager: &EnvManager, args: EnvProfileApplyCmd) -> CliResult {

}
    Ok(())
    );
        meta.var_count
        meta.scope,
        meta.name,
        "ok\tprofile.capture\t{}\t{}\t{} vars",
    out_println!(
        .map_err(map_env_err)?;
        .profile_capture(&args.name, scope)
    let meta = manager
    let scope = parse_writable_scope(&args.scope)?;
pub(super) fn cmd_profile_capture(manager: &EnvManager, args: EnvProfileCaptureCmd) -> CliResult {

}
    Ok(())
    }
        }
            print_table(&table);
            }
                ]);
                    Cell::new(p.var_count),
                    Cell::new(p.created_at),
                    Cell::new(p.scope.to_string()),
                    Cell::new(p.name),
                table.add_row(vec![
            for p in profiles {
            ]);
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Magenta)
                Cell::new("Vars")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Yellow)
                Cell::new("Created")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Green)
                Cell::new("Scope")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Cyan)
                Cell::new("Name")
            table.set_header(vec![
            apply_pretty_table_style(&mut table);
            let mut table = Table::new();
        _ => {
        }
            }
                out_println!("{}\t{}\t{}\t{}", p.name, p.scope, p.created_at, p.var_count);
            for p in profiles {
        ListFormat::Tsv => {
        }
            );
                serde_json::to_string_pretty(&profiles).unwrap_or_default()
                "{}",
            out_println!(
        ListFormat::Json => {
    match parse_format(&args.format)? {
    let profiles = manager.profile_list().map_err(map_env_err)?;
pub(super) fn cmd_profile_list(manager: &EnvManager, args: EnvProfileListCmd) -> CliResult {

}
    }
        EnvProfileSubCommand::Rm(a) => cmd_profile_delete(manager, a),
        EnvProfileSubCommand::Diff(a) => cmd_profile_diff(manager, a),
        EnvProfileSubCommand::Apply(a) => cmd_profile_apply(manager, a),
        EnvProfileSubCommand::Capture(a) => cmd_profile_capture(manager, a),
        EnvProfileSubCommand::List(a) => cmd_profile_list(manager, a),
    match args.cmd {
pub(super) fn cmd_profile(manager: &EnvManager, args: EnvProfileCmd) -> CliResult {

}
    Ok(())
    }
        }
            return Err(CliError::new(code, "doctor reported issues"));
        if code > 0 {
        let code = doctor::doctor_exit_code(&report);
    if !format.eq_ignore_ascii_case("json") {
    }
        out_println!("{}", doctor::report_text(&report));
    } else {
        );
            serde_json::to_string_pretty(&report).unwrap_or_default()
            "{}",
        out_println!(
    if format.eq_ignore_ascii_case("json") {
    };
        manager.doctor_run(scope).map_err(map_env_err)?
    } else {
        manager.check_run(scope).map_err(map_env_err)?
    let report = if use_check_alias {
    }
        return Ok(());
        }
            }
                out_println!("  - {}", line);
            for line in fixed.details {
            out_println!("doctor fixed: {} item(s)", fixed.fixed);
        } else {
            );
                serde_json::to_string_pretty(&fixed).unwrap_or_default()
                "{}",
            out_println!(
        if format.eq_ignore_ascii_case("json") {
        let fixed = manager.doctor_fix(scope).map_err(map_env_err)?;
    if fix {
    let scope = parse_scope(&scope_raw)?;
) -> CliResult {
    use_check_alias: bool,
    format: String,
    fix: bool,
    scope_raw: String,
    manager: &EnvManager,
pub(super) fn cmd_doctor_like(

}
    cmd_doctor_like(manager, args.scope, args.fix, args.format, false)
pub(super) fn cmd_doctor(manager: &EnvManager, args: EnvDoctorCmd) -> CliResult {

}
    Ok(())
    );
        remaining
        removed,
        args.keep,
        "ok\tsnapshot.prune\tkeep={}\tremoved={}\tremaining={}",
    out_println!(
    let remaining = manager.snapshot_list().map_err(map_env_err)?.len();
    let removed = manager.snapshot_prune(args.keep).map_err(map_env_err)?;
pub(super) fn cmd_snapshot_prune(manager: &EnvManager, args: EnvSnapshotPruneCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\tsnapshot.restore\t{}\t{}", restored.id, scope);
        .map_err(map_env_err)?;
        .snapshot_restore(scope, args.id.as_deref(), args.latest)
    let restored = manager
    }
        return Err(CliError::new(2, "operation canceled"));
    )? {
        args.yes,
        ),
            scope
            args.id.clone().unwrap_or_else(|| "latest".to_string()),
            "Restore snapshot ({}) into {} scope? Existing values will be replaced.",
        &format!(
    if !prompt_confirm(
    let scope = parse_scope(&args.scope)?;
    }
        ));
            &["Fix: xun env snapshot restore --latest -y"],
            "restore requires --id <id> or --latest".to_string(),
            2,
        return Err(CliError::with_details(
    if args.id.is_none() && !args.latest {
pub(super) fn cmd_snapshot_restore(manager: &EnvManager, args: EnvSnapshotRestoreCmd) -> CliResult {

}
    Ok(())
    }
        }
            print_table(&table);
            }
                ]);
                    Cell::new(s.description),
                    Cell::new(s.created_at),
                    Cell::new(s.id),
                table.add_row(vec![
            for s in snapshots {
            ]);
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Yellow)
                Cell::new("Description")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Green)
                Cell::new("Created")
                    .add_attribute(Attribute::Bold),
                    .fg(Color::Cyan)
                Cell::new("ID")
            table.set_header(vec![
            apply_pretty_table_style(&mut table);
            let mut table = Table::new();
        _ => {
        }
            }
                out_println!("{}\t{}\t{}", s.id, s.created_at, s.description);
            for s in snapshots {
        ListFormat::Tsv => {
        }
            );
                serde_json::to_string_pretty(&snapshots).unwrap_or_default()
                "{}",
            out_println!(
        ListFormat::Json => {
    match parse_format(&args.format)? {
    let snapshots = manager.snapshot_list().map_err(map_env_err)?;
pub(super) fn cmd_snapshot_list(manager: &EnvManager, args: EnvSnapshotListCmd) -> CliResult {

}
    Ok(())
    out_println!("ok\tsnapshot.create\t{}\t{}", meta.id, meta.description);
        .map_err(map_env_err)?;
        .snapshot_create(args.desc.as_deref())
    let meta = manager
pub(super) fn cmd_snapshot_create(manager: &EnvManager, args: EnvSnapshotCreateCmd) -> CliResult {

}
    }
        EnvSnapshotSubCommand::Prune(a) => cmd_snapshot_prune(manager, a),
        EnvSnapshotSubCommand::Restore(a) => cmd_snapshot_restore(manager, a),
        EnvSnapshotSubCommand::List(a) => cmd_snapshot_list(manager, a),
        EnvSnapshotSubCommand::Create(a) => cmd_snapshot_create(manager, a),
    match args.cmd {
pub(super) fn cmd_snapshot(manager: &EnvManager, args: EnvSnapshotCmd) -> CliResult {

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


