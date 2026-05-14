use super::*;

pub(super) fn cmd_profile(manager: &EnvManager, args: EnvProfileCmd) -> CliResult {
    match args.cmd {
        EnvProfileSubCommand::List(a) => cmd_profile_list(manager, a),
        EnvProfileSubCommand::Capture(a) => cmd_profile_capture(manager, a),
        EnvProfileSubCommand::Apply(a) => cmd_profile_apply(manager, a),
        EnvProfileSubCommand::Diff(a) => cmd_profile_diff(manager, a),
        EnvProfileSubCommand::Rm(a) => cmd_profile_delete(manager, a),
    }
}

pub(super) fn cmd_profile_list(manager: &EnvManager, args: EnvProfileListCmd) -> CliResult {
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

pub(super) fn cmd_profile_capture(manager: &EnvManager, args: EnvProfileCaptureCmd) -> CliResult {
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

pub(super) fn cmd_profile_apply(manager: &EnvManager, args: EnvProfileApplyCmd) -> CliResult {
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

pub(super) fn cmd_profile_diff(manager: &EnvManager, args: EnvProfileDiffCmd) -> CliResult {
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

pub(super) fn cmd_profile_delete(manager: &EnvManager, args: EnvProfileDeleteCmd) -> CliResult {
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

pub(super) fn cmd_apply(manager: &EnvManager, args: EnvApplyCmd) -> CliResult {
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
