use super::*;

pub(super) fn cmd_path(manager: &EnvManager, args: EnvPathCmd) -> CliResult {
    match args.cmd {
        EnvPathSubCommand::Add(a) => cmd_path_add(manager, a),
        EnvPathSubCommand::Rm(a) => cmd_path_rm(manager, a),
    }
}

pub(super) fn cmd_path_add(manager: &EnvManager, args: EnvPathAddCmd) -> CliResult {
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

pub(super) fn cmd_path_rm(manager: &EnvManager, args: EnvPathRmCmd) -> CliResult {
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

pub(super) fn cmd_path_dedup(manager: &EnvManager, args: EnvPathDedupCmd) -> CliResult {
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
