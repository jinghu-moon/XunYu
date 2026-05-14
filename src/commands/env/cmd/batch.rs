use super::*;

pub(super) fn cmd_batch(manager: &EnvManager, args: EnvBatchCmd) -> CliResult {
    match args.cmd {
        EnvBatchSubCommand::Set(a) => cmd_batch_set(manager, a),
        EnvBatchSubCommand::Rm(a) => cmd_batch_delete(manager, a),
        EnvBatchSubCommand::Rename(a) => cmd_batch_rename(manager, a),
    }
}

pub(super) fn cmd_batch_set(manager: &EnvManager, args: EnvBatchSetCmd) -> CliResult {
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

pub(super) fn cmd_batch_delete(manager: &EnvManager, args: EnvBatchDeleteCmd) -> CliResult {
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

pub(super) fn cmd_batch_rename(manager: &EnvManager, args: EnvBatchRenameCmd) -> CliResult {
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
