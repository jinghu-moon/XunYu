use super::*;

pub(super) fn cmd_env_config(manager: &EnvManager, args: EnvConfigCmd) -> CliResult {
    match args.cmd {
        EnvConfigSubCommand::Show(a) => cmd_env_config_show(manager, a),
        EnvConfigSubCommand::Path(a) => cmd_env_config_path(manager, a),
        EnvConfigSubCommand::Reset(a) => cmd_env_config_reset(manager, a),
        EnvConfigSubCommand::Get(a) => cmd_env_config_get(manager, a),
        EnvConfigSubCommand::Set(a) => cmd_env_config_set(manager, a),
    }
}

pub(super) fn cmd_env_config_show(manager: &EnvManager, args: EnvConfigShowCmd) -> CliResult {
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

pub(super) fn cmd_env_config_path(manager: &EnvManager, _args: EnvConfigPathCmd) -> CliResult {
    out_println!("{}", manager.env_config_path().display());
    Ok(())
}

pub(super) fn cmd_env_config_reset(manager: &EnvManager, args: EnvConfigResetCmd) -> CliResult {
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

pub(super) fn cmd_env_config_get(manager: &EnvManager, args: EnvConfigGetCmd) -> CliResult {
    let value = manager.env_config_get(&args.key).map_err(map_env_err)?;
    out_println!("{}", value);
    Ok(())
}

pub(super) fn cmd_env_config_set(manager: &EnvManager, args: EnvConfigSetCmd) -> CliResult {
    manager
        .env_config_set(&args.key, &args.value)
        .map_err(map_env_err)?;
    out_println!("ok\tenv.config.set\t{}\t{}", args.key, args.value);
    Ok(())
}
