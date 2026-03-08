use super::*;

pub(super) fn cmd_list(manager: &EnvManager, args: EnvListCmd) -> CliResult {
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

pub(super) fn cmd_search(manager: &EnvManager, args: EnvSearchCmd) -> CliResult {
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

pub(super) fn cmd_get(manager: &EnvManager, args: EnvGetCmd) -> CliResult {
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

pub(super) fn cmd_set(manager: &EnvManager, args: EnvSetCmd) -> CliResult {
    let scope = parse_writable_scope(&args.scope)?;
    manager
        .set_var(scope, &args.name, &args.value, args.no_snapshot)
        .map_err(map_env_err)?;
    out_println!("ok\tset\t{}\t{}", scope, args.name);
    Ok(())
}

pub(super) fn cmd_del(manager: &EnvManager, args: EnvDelCmd) -> CliResult {
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
