use super::*;

pub(super) fn cmd_export(manager: &EnvManager, args: EnvExportCmd) -> CliResult {
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

pub(super) fn cmd_export_all(manager: &EnvManager, args: EnvExportAllCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let data = manager.export_bundle(scope).map_err(map_env_err)?;
    let out_path = args.out.unwrap_or_else(|| format!("xun-env-{}.zip", scope));
    std::fs::write(&out_path, &data).map_err(|e| CliError::new(1, format!("{e}")))?;
    out_println!("ok\texport-all\t{}\t{} bytes", out_path, data.len());
    Ok(())
}

pub(super) fn cmd_export_live(manager: &EnvManager, args: EnvExportLiveCmd) -> CliResult {
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

pub(super) fn cmd_env_merged(manager: &EnvManager, args: EnvMergedCmd) -> CliResult {
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

pub(super) fn cmd_import(manager: &EnvManager, args: EnvImportCmd) -> CliResult {
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
    if matches!(strategy, ImportStrategy::Overwrite)
        && !args.dry_run
        && !prompt_confirm(
            "Import with overwrite will replace existing variables. Continue?",
            args.yes,
        )?
    {
        return Err(CliError::new(2, "operation canceled"));
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
