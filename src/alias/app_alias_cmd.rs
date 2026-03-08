use super::context::{AliasCtx, split_csv_multi};
use super::*;

pub(super) fn cmd_app_add(ctx: &AliasCtx, args: AliasAppAddCmd) -> Result<()> {
    let mut cfg = ctx.load()?;
    if cfg.name_exists(&args.name) && !args.force {
        bail!("Alias already exists: {} (use --force)", args.name);
    }
    cfg.app.insert(
        args.name.clone(),
        AppAlias {
            exe: args.exe.clone(),
            args: args.args.clone(),
            desc: args.desc.clone(),
            tags: split_csv_multi(&args.tag),
            register_apppaths: !args.no_apppaths,
        },
    );
    ctx.save(&cfg)?;
    ctx.sync_shims(&cfg)?;
    if !args.no_apppaths {
        let _ = apppaths::register(&args.name, &args.exe);
    }
    ctx.sync_shells(&cfg, None)?;
    ui_println!("App alias added: {}", args.name);
    Ok(())
}

pub(super) fn cmd_app_rm(ctx: &AliasCtx, args: AliasAppRmCmd) -> Result<()> {
    if args.names.is_empty() {
        bail!("No app alias names provided.");
    }
    let mut cfg = ctx.load()?;
    for name in args.names {
        if cfg.app.remove(&name).is_some() {
            let _ = shim_gen::remove_shim(&ctx.shims_dir, &name);
            let _ = apppaths::unregister(&name);
            ui_println!("Removed app alias: {name}");
        } else {
            ui_println!("App alias not found: {name}");
        }
    }
    ctx.save(&cfg)?;
    ctx.sync_shells(&cfg, None)?;
    Ok(())
}

pub(super) fn cmd_app_ls(ctx: &AliasCtx, args: AliasAppLsCmd) -> Result<()> {
    let cfg = ctx.load()?;
    if args.json {
        out_println!("{}", serde_json::to_string_pretty(&cfg.app)?);
        return Ok(());
    }
    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(["Name", "Executable", "Args", "AppPaths", "Desc"]);
    for (name, app) in cfg.app {
        table.add_row(vec![
            Cell::new(name),
            Cell::new(app.exe),
            Cell::new(app.args.unwrap_or_default()),
            Cell::new(if app.register_apppaths { "yes" } else { "no" }).fg(
                if app.register_apppaths {
                    Color::Green
                } else {
                    Color::Yellow
                },
            ),
            Cell::new(app.desc.unwrap_or_default()),
        ]);
    }
    print_table(&table);
    Ok(())
}

pub(super) fn cmd_app_scan(ctx: &AliasCtx, args: AliasAppScanCmd) -> Result<()> {
    let source = ScanSource::from_str(&args.source)
        .ok_or_else(|| anyhow::anyhow!("Invalid source: {}", args.source))?;
    let entries = scanner::scan(source, args.filter.as_deref(), args.no_cache);

    if args.json {
        out_println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }
    if entries.is_empty() {
        ui_println!("No applications discovered.");
        return Ok(());
    }

    for (idx, entry) in entries.iter().enumerate() {
        ui_println!(
            "{:>3}. {} <- {} ({})",
            idx + 1,
            entry.name,
            entry.display_name,
            entry.source
        );
        ui_println!("     {}", entry.exe_path);
    }

    let selected = if args.all {
        (0..entries.len()).collect::<Vec<_>>()
    } else {
        print!("Select entries (1,3-5,a): ");
        let _ = io::stdout().flush();
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        parse_selection(&input, entries.len())
    };
    if selected.is_empty() {
        ui_println!("No selection.");
        return Ok(());
    }

    let mut cfg = ctx.load()?;
    let mut added = 0usize;
    for idx in selected {
        let entry = &entries[idx];
        if cfg.name_exists(&entry.name) {
            continue;
        }
        cfg.app.insert(
            entry.name.clone(),
            AppAlias {
                exe: entry.exe_path.clone(),
                args: None,
                desc: Some(entry.display_name.clone()),
                tags: Vec::new(),
                register_apppaths: true,
            },
        );
        added += 1;
    }
    if added == 0 {
        ui_println!("No new app aliases added.");
        return Ok(());
    }
    ctx.save(&cfg)?;
    ctx.sync_shims(&cfg)?;
    ctx.sync_shells(&cfg, None)?;
    ui_println!("Added app aliases: {added}");
    Ok(())
}
