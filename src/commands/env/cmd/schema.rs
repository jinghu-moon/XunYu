use super::*;

pub(super) fn cmd_validate(manager: &EnvManager, args: EnvValidateCmd) -> CliResult {
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

pub(super) fn cmd_schema(manager: &EnvManager, args: EnvSchemaCmd) -> CliResult {
    match args.cmd {
        EnvSchemaSubCommand::Show(a) => cmd_schema_show(manager, a),
        EnvSchemaSubCommand::AddRequired(a) => cmd_schema_add_required(manager, a),
        EnvSchemaSubCommand::AddRegex(a) => cmd_schema_add_regex(manager, a),
        EnvSchemaSubCommand::AddEnum(a) => cmd_schema_add_enum(manager, a),
        EnvSchemaSubCommand::Remove(a) => cmd_schema_remove(manager, a),
        EnvSchemaSubCommand::Reset(a) => cmd_schema_reset(manager, a),
    }
}

pub(super) fn cmd_schema_show(manager: &EnvManager, args: EnvSchemaShowCmd) -> CliResult {
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

pub(super) fn cmd_schema_add_required(
    manager: &EnvManager,
    args: EnvSchemaAddRequiredCmd,
) -> CliResult {
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

pub(super) fn cmd_schema_add_regex(manager: &EnvManager, args: EnvSchemaAddRegexCmd) -> CliResult {
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

pub(super) fn cmd_schema_add_enum(manager: &EnvManager, args: EnvSchemaAddEnumCmd) -> CliResult {
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

pub(super) fn cmd_schema_remove(manager: &EnvManager, args: EnvSchemaRemoveCmd) -> CliResult {
    let schema = manager.schema_remove(&args.pattern).map_err(map_env_err)?;
    out_println!(
        "ok\tschema.remove\t{}\trules={}",
        args.pattern,
        schema.rules.len()
    );
    Ok(())
}

pub(super) fn cmd_schema_reset(manager: &EnvManager, args: EnvSchemaResetCmd) -> CliResult {
    if !prompt_confirm("Reset all schema rules?", args.yes)? {
        return Err(CliError::new(2, "operation canceled"));
    }
    let schema = manager.schema_reset().map_err(map_env_err)?;
    out_println!("ok\tschema.reset\trules={}", schema.rules.len());
    Ok(())
}
