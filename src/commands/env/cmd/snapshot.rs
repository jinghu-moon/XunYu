use super::*;

pub(super) fn cmd_snapshot(manager: &EnvManager, args: EnvSnapshotCmd) -> CliResult {
    match args.cmd {
        EnvSnapshotSubCommand::Create(a) => cmd_snapshot_create(manager, a),
        EnvSnapshotSubCommand::List(a) => cmd_snapshot_list(manager, a),
        EnvSnapshotSubCommand::Restore(a) => cmd_snapshot_restore(manager, a),
        EnvSnapshotSubCommand::Prune(a) => cmd_snapshot_prune(manager, a),
    }
}

pub(super) fn cmd_snapshot_create(manager: &EnvManager, args: EnvSnapshotCreateCmd) -> CliResult {
    let meta = manager
        .snapshot_create(args.desc.as_deref())
        .map_err(map_env_err)?;
    out_println!("ok\tsnapshot.create\t{}\t{}", meta.id, meta.description);
    Ok(())
}

pub(super) fn cmd_snapshot_list(manager: &EnvManager, args: EnvSnapshotListCmd) -> CliResult {
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

pub(super) fn cmd_snapshot_restore(manager: &EnvManager, args: EnvSnapshotRestoreCmd) -> CliResult {
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

pub(super) fn cmd_snapshot_prune(manager: &EnvManager, args: EnvSnapshotPruneCmd) -> CliResult {
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
