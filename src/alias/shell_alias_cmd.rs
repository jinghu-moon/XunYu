use super::context::{AliasCtx, split_csv_multi};
use super::*;

pub(super) fn cmd_setup(ctx: &AliasCtx, args: AliasSetupCmd) -> Result<()> {
    fs::create_dir_all(&ctx.config_dir)
        .with_context(|| format!("Failed to create {}", ctx.config_dir.display()))?;
    fs::create_dir_all(&ctx.shims_dir)
        .with_context(|| format!("Failed to create {}", ctx.shims_dir.display()))?;
    shim_gen::deploy_shim_templates(&ctx.template_path, &ctx.template_gui_path)?;

    if !ctx.config_path.exists() {
        ctx.save(&Config::default())?;
    }
    let cfg = ctx.load()?;
    ctx.sync_shims(&cfg)?;
    ctx.sync_shells(&cfg, Some(&args))?;
    ui_println!("Alias setup completed.");
    ui_println!("Config: {}", ctx.config_path.display());
    ui_println!("Shims : {}", ctx.shims_dir.display());
    ui_println!("Template(console): {}", ctx.template_path.display());
    ui_println!("Template(gui)    : {}", ctx.template_gui_path.display());
    Ok(())
}

pub(super) fn cmd_add(ctx: &AliasCtx, args: AliasAddCmd) -> Result<()> {
    let t_total = std::time::Instant::now();
    let mut cfg = ctx.load()?;
    config::validate_alias_name(&args.name)?;
    if cfg.name_exists(&args.name) && !args.force {
        bail!(
            "Alias already exists: {} (use --force to overwrite)",
            args.name
        );
    }
    if let Some(old_app) = cfg.app.remove(&args.name)
        && old_app.register_apppaths
    {
        let _ = apppaths::unregister(&args.name);
    }
    let mode = args.mode.parse::<AliasMode>().map_err(anyhow::Error::msg)?;
    cfg.alias.insert(
        args.name.clone(),
        ShellAlias {
            command: args.command,
            desc: args.desc,
            tags: split_csv_multi(&args.tag),
            shells: split_csv_multi(&args.shell),
            mode,
        },
    );
    ctx.save(&cfg)?;
    if let Some(alias) = cfg.alias.get(&args.name) {
        ctx.sync_shell_alias_shim(&args.name, alias)?;
    }
    ctx.sync_shells(&cfg, None)?;
    context::t_print_total("cmd_add", t_total);
    ui_println!("Alias added.");
    Ok(())
}

pub(super) fn cmd_rm(ctx: &AliasCtx, args: AliasRmCmd) -> Result<()> {
    if args.names.is_empty() {
        bail!("No alias names provided.");
    }
    let mut cfg = ctx.load()?;
    for name in args.names {
        let mut touched = false;
        if cfg.alias.remove(&name).is_some() {
            touched = true;
        }
        if cfg.app.remove(&name).is_some() {
            touched = true;
        }
        if touched {
            let _ = shim_gen::remove_shim(&ctx.shims_dir, &name);
            let _ = apppaths::unregister(&name);
            ui_println!("Removed: {name}");
        } else {
            ui_println!("Not found: {name}");
        }
    }
    ctx.save(&cfg)?;
    ctx.sync_shells(&cfg, None)?;
    Ok(())
}

pub(super) fn cmd_export(ctx: &AliasCtx, args: AliasExportCmd) -> Result<()> {
    let cfg = ctx.load()?;
    let text = toml::to_string_pretty(&cfg)?;
    if let Some(path) = args.output {
        fs::write(&path, text.as_bytes()).with_context(|| format!("Failed to write {path}"))?;
        ui_println!("Exported aliases to {path}");
    } else {
        out_println!("{text}");
    }
    Ok(())
}

pub(super) fn cmd_import(ctx: &AliasCtx, args: AliasImportCmd) -> Result<()> {
    let t_total = std::time::Instant::now();
    let text =
        fs::read_to_string(&args.file).with_context(|| format!("Failed to read {}", args.file))?;
    let src: Config =
        toml::from_str(&text).with_context(|| format!("Invalid TOML: {}", args.file))?;
    let mut dst = ctx.load()?;
    let mut added = 0usize;
    let mut skipped = 0usize;
    let touched_capacity = src.alias.len() + src.app.len();
    let app_alias_capacity = src.app.len();
    let mut touched_entries = Vec::with_capacity(touched_capacity);
    let mut app_gui_cache = std::collections::HashMap::with_capacity(app_alias_capacity);

    for (name, alias) in src.alias {
        if dst.name_exists(&name) && !args.force {
            skipped += 1;
            continue;
        }
        dst.app.remove(&name);
        dst.alias.insert(name.clone(), alias);
        if let Some(alias) = dst.alias.get(&name) {
            touched_entries.push(shim_gen::shell_alias_to_sync_entry(&name, alias));
        }
        added += 1;
    }
    for (name, app) in src.app {
        if dst.name_exists(&name) && !args.force {
            skipped += 1;
            continue;
        }
        dst.alias.remove(&name);
        dst.app.insert(name.clone(), app);
        if let Some(alias) = dst.app.get(&name) {
            let use_gui_template = *app_gui_cache
                .entry(alias.exe.clone())
                .or_insert_with(|| shim_gen::app_alias_uses_gui_template(alias));
            touched_entries.push(shim_gen::app_alias_to_sync_entry_with_gui(
                &name,
                alias,
                use_gui_template,
            ));
        }
        added += 1;
    }

    if added == 0 {
        ui_println!("Imported aliases: added={added}, skipped={skipped}");
        return Ok(());
    }

    ctx.save(&dst)?;
    ctx.sync_selected_shims(&touched_entries)?;
    ctx.sync_shells(&dst, None)?;
    context::t_print_total("cmd_import", t_total);
    ui_println!("Imported aliases: added={added}, skipped={skipped}");
    Ok(())
}
