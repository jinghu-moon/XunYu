use super::*;

pub(super) fn cmd_audit(manager: &EnvManager, args: EnvAuditCmd) -> CliResult {
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

pub(super) fn cmd_watch(manager: &EnvManager, args: EnvWatchCmd) -> CliResult {
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


pub(super) fn cmd_template(manager: &EnvManager, args: EnvTemplateCmd) -> CliResult {
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

pub(super) fn cmd_run(manager: &EnvManager, args: EnvRunCmd) -> CliResult {
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


