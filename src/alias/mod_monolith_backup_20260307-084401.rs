pub(crate) mod apppaths;
pub(crate) mod config;
pub(crate) mod error;
pub(crate) mod output;
pub(crate) mod scanner;
pub(crate) mod shell;
pub(crate) mod shim_gen;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use comfy_table::{Cell, Color, Table};

use crate::alias::config::{AliasMode, AppAlias, Config, ShellAlias};
use crate::alias::output::{fuzzy_score, parse_selection};
use crate::alias::scanner::ScanSource;
use crate::alias::shell::ShellBackend;
use crate::alias::shell::cmd::CmdBackend;
use crate::alias::shell::ps::PsBackend;
#[cfg(feature = "alias-shell-extra")]
use crate::alias::shell::{bash::BashBackend, nu::NuBackend};
use crate::cli::*;
use crate::output::{apply_pretty_table_style, print_table};

pub(crate) fn cmd_alias(args: AliasCmd) -> Result<()> {
    let ctx = AliasCtx::from_cli(&args);
    match args.cmd {
        AliasSubCommand::Setup(cmd) => cmd_setup(&ctx, cmd),
        AliasSubCommand::Add(cmd) => cmd_add(&ctx, cmd),
        AliasSubCommand::Rm(cmd) => cmd_rm(&ctx, cmd),
        AliasSubCommand::Ls(cmd) => cmd_ls(&ctx, cmd),
        AliasSubCommand::Find(cmd) => cmd_find(&ctx, cmd),
        AliasSubCommand::Which(cmd) => cmd_which(&ctx, &cmd.name, false),
        AliasSubCommand::Sync(_) => cmd_sync(&ctx),
        AliasSubCommand::Export(cmd) => cmd_export(&ctx, cmd),
        AliasSubCommand::Import(cmd) => cmd_import(&ctx, cmd),
        AliasSubCommand::App(cmd) => match cmd.cmd {
            AliasAppSubCommand::Add(c) => cmd_app_add(&ctx, c),
            AliasAppSubCommand::Rm(c) => cmd_app_rm(&ctx, c),
            AliasAppSubCommand::Ls(c) => cmd_app_ls(&ctx, c),
            AliasAppSubCommand::Scan(c) => cmd_app_scan(&ctx, c),
            AliasAppSubCommand::Which(c) => cmd_which(&ctx, &c.name, true),
            AliasAppSubCommand::Sync(_) => cmd_app_sync(&ctx),
        },
    }
}

struct AliasCtx {
    config_path: PathBuf,
    shims_dir: PathBuf,
    template_path: PathBuf,
    template_gui_path: PathBuf,
    config_dir: PathBuf,
}

impl AliasCtx {
    fn from_cli(cli: &AliasCmd) -> Self {
        let override_path = cli.config.as_deref().map(Path::new);
        let config_path = config::config_path(override_path);
        let shims_dir = config::shims_dir(&config_path);
        let template_path = config::shim_template_path(&config_path);
        let template_gui_path = config::shim_gui_template_path(&config_path);
        let config_dir = config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Self {
            config_path,
            shims_dir,
            template_path,
            template_gui_path,
            config_dir,
        }
    }

    fn load(&self) -> Result<Config> {
        config::load(&self.config_path)
    }

    fn save(&self, cfg: &Config) -> Result<()> {
        config::save(&self.config_path, cfg)
    }

    fn sync_shims(&self, cfg: &Config) -> Result<()> {
        let entries = shim_gen::config_to_sync_entries(cfg);
        let report = shim_gen::sync_all(
            &entries,
            &self.shims_dir,
            &self.template_path,
            &self.template_gui_path,
        )?;
        for (name, err) in report.errors {
            ui_println!("Warning: shim sync failed [{name}]: {err}");
        }
        Ok(())
    }

    fn sync_shells(&self, cfg: &Config, setup: Option<&AliasSetupCmd>) -> Result<()> {
        let mut skip_cmd = false;
        let mut skip_ps = false;
        let mut skip_bash = false;
        let mut skip_nu = false;
        if let Some(setup) = setup {
            skip_cmd = setup.no_cmd;
            skip_ps = setup.no_ps;
            skip_bash = setup.no_bash || setup.core_only;
            skip_nu = setup.no_nu || setup.core_only;
        }

        #[allow(unused_mut)]
        let mut backends: Vec<(bool, Box<dyn ShellBackend>)> = vec![
            (!skip_cmd, Box::new(CmdBackend::new(&self.config_dir))),
            (!skip_ps, Box::new(PsBackend::new(None))),
        ];
        #[cfg(feature = "alias-shell-extra")]
        {
            backends.push((!skip_bash, Box::new(BashBackend::new(None))));
            backends.push((!skip_nu, Box::new(NuBackend::new(None))));
        }
        #[cfg(not(feature = "alias-shell-extra"))]
        {
            let _ = (skip_bash, skip_nu);
        }

        for (enabled, backend) in backends {
            if !enabled {
                continue;
            }
            match backend.update(cfg) {
                Ok(shell::UpdateResult::Written { path }) => {
                    ui_println!("Updated {} profile: {}", backend.name(), path.display());
                }
                Ok(shell::UpdateResult::Skipped { reason }) => {
                    ui_println!("Skipped {}: {reason}", backend.name());
                }
                Err(err) => {
                    ui_println!("Warning: {} backend update failed: {err}", backend.name());
                }
            }
        }
        Ok(())
    }
}

fn cmd_setup(ctx: &AliasCtx, args: AliasSetupCmd) -> Result<()> {
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

fn cmd_add(ctx: &AliasCtx, args: AliasAddCmd) -> Result<()> {
    let mut cfg = ctx.load()?;
    if cfg.name_exists(&args.name) && !args.force {
        bail!(
            "Alias already exists: {} (use --force to overwrite)",
            args.name
        );
    }
    let mode = args.mode.parse::<AliasMode>().map_err(anyhow::Error::msg)?;
    cfg.alias.insert(
        args.name,
        ShellAlias {
            command: args.command,
            desc: args.desc,
            tags: split_csv_multi(&args.tag),
            shells: split_csv_multi(&args.shell),
            mode,
        },
    );
    ctx.save(&cfg)?;
    ctx.sync_shims(&cfg)?;
    ctx.sync_shells(&cfg, None)?;
    ui_println!("Alias added.");
    Ok(())
}

fn cmd_rm(ctx: &AliasCtx, args: AliasRmCmd) -> Result<()> {
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

fn cmd_ls(ctx: &AliasCtx, args: AliasLsCmd) -> Result<()> {
    let cfg = ctx.load()?;
    let show_cmd = args.r#type.as_deref().map(|v| v == "cmd").unwrap_or(true);
    let show_app = args.r#type.as_deref().map(|v| v == "app").unwrap_or(true);

    if args.json {
        let mut out = serde_json::Map::new();
        if show_cmd {
            out.insert("alias".to_string(), serde_json::to_value(&cfg.alias)?);
        }
        if show_app {
            out.insert("app".to_string(), serde_json::to_value(&cfg.app)?);
        }
        out_println!("{}", serde_json::Value::Object(out));
        return Ok(());
    }

    if show_cmd {
        let mut table = Table::new();
        apply_pretty_table_style(&mut table);
        table.set_header(["Name", "Command", "Mode", "Shells", "Desc"]);
        for (name, alias) in &cfg.alias {
            if let Some(tag) = &args.tag {
                if !alias.tags.iter().any(|t| t == tag) {
                    continue;
                }
            }
            table.add_row(vec![
                Cell::new(name),
                Cell::new(&alias.command),
                Cell::new(format!("{:?}", alias.mode).to_ascii_lowercase()),
                Cell::new(if alias.shells.is_empty() {
                    "all".to_string()
                } else {
                    alias.shells.join(",")
                }),
                Cell::new(alias.desc.clone().unwrap_or_default()),
            ]);
        }
        print_table(&table);
    }

    if show_app {
        let mut table = Table::new();
        apply_pretty_table_style(&mut table);
        table.set_header(["Name", "Executable", "Args", "AppPaths", "Desc"]);
        for (name, app) in &cfg.app {
            if let Some(tag) = &args.tag {
                if !app.tags.iter().any(|t| t == tag) {
                    continue;
                }
            }
            table.add_row(vec![
                Cell::new(name),
                Cell::new(&app.exe),
                Cell::new(app.args.clone().unwrap_or_default()),
                Cell::new(if app.register_apppaths { "yes" } else { "no" }),
                Cell::new(app.desc.clone().unwrap_or_default()),
            ]);
        }
        print_table(&table);
    }
    Ok(())
}

fn cmd_find(ctx: &AliasCtx, args: AliasFindCmd) -> Result<()> {
    let cfg = ctx.load()?;
    let kw = args.keyword;
    let mut rows: Vec<(i32, String, String, String)> = Vec::new();

    for (name, alias) in &cfg.alias {
        let score = fuzzy_score(
            name,
            &alias.command,
            alias.desc.as_deref().unwrap_or(""),
            &kw,
        );
        if score > 0 {
            rows.push((
                score,
                format!("[cmd] {name}"),
                alias.command.clone(),
                alias.desc.clone().unwrap_or_default(),
            ));
        }
    }
    for (name, app) in &cfg.app {
        let score = fuzzy_score(name, &app.exe, app.desc.as_deref().unwrap_or(""), &kw);
        if score > 0 {
            rows.push((
                score,
                format!("[app] {name}"),
                app.exe.clone(),
                app.desc.clone().unwrap_or_default(),
            ));
        }
    }

    rows.sort_by(|a, b| b.0.cmp(&a.0));
    if rows.is_empty() {
        ui_println!("No alias matched keyword: {kw}");
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(["Type/Name", "Target", "Desc", "Score"]);
    for (score, name, target, desc) in rows {
        table.add_row(vec![name, target, desc, score.to_string()]);
    }
    print_table(&table);
    Ok(())
}

fn cmd_which(ctx: &AliasCtx, name: &str, app_only: bool) -> Result<()> {
    let cfg = ctx.load()?;
    if app_only {
        if let Some(app) = cfg.app.get(name) {
            let shim = shim_gen::app_alias_to_shim(app);
            print_which(ctx, name, &app.exe, &shim);
            return Ok(());
        }
        bail!("App alias not found: {name}");
    }
    if let Some(alias) = cfg.alias.get(name) {
        let shim = shim_gen::shell_alias_to_shim(alias);
        print_which(ctx, name, &alias.command, &shim);
        return Ok(());
    }
    if let Some(app) = cfg.app.get(name) {
        let shim = shim_gen::app_alias_to_shim(app);
        print_which(ctx, name, &app.exe, &shim);
        return Ok(());
    }
    bail!("Alias not found: {name}")
}

fn print_which(ctx: &AliasCtx, name: &str, target: &str, shim_text: &str) {
    ui_println!("Name: {name}");
    ui_println!("Target: {target}");
    ui_println!(
        "Shim exe : {}",
        ctx.shims_dir.join(format!("{name}.exe")).display()
    );
    ui_println!(
        "Shim file: {}",
        ctx.shims_dir.join(format!("{name}.shim")).display()
    );
    ui_println!(".shim content:\n{shim_text}");
}

fn cmd_sync(ctx: &AliasCtx) -> Result<()> {
    let cfg = ctx.load()?;
    ctx.sync_shims(&cfg)?;
    ctx.sync_shells(&cfg, None)?;
    let (registered, removed) = apppaths::sync_apppaths(&cfg)?;
    ui_println!("App Paths synced: +{registered} / -{removed}");
    Ok(())
}

fn cmd_export(ctx: &AliasCtx, args: AliasExportCmd) -> Result<()> {
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

fn cmd_import(ctx: &AliasCtx, args: AliasImportCmd) -> Result<()> {
    let text =
        fs::read_to_string(&args.file).with_context(|| format!("Failed to read {}", args.file))?;
    let src: Config =
        toml::from_str(&text).with_context(|| format!("Invalid TOML: {}", args.file))?;
    let mut dst = ctx.load()?;
    let mut added = 0usize;
    let mut skipped = 0usize;

    for (name, alias) in src.alias {
        if dst.name_exists(&name) && !args.force {
            skipped += 1;
            continue;
        }
        dst.alias.insert(name, alias);
        added += 1;
    }
    for (name, app) in src.app {
        if dst.name_exists(&name) && !args.force {
            skipped += 1;
            continue;
        }
        dst.app.insert(name, app);
        added += 1;
    }
    ctx.save(&dst)?;
    ctx.sync_shims(&dst)?;
    ctx.sync_shells(&dst, None)?;
    ui_println!("Imported aliases: added={added}, skipped={skipped}");
    Ok(())
}

fn cmd_app_add(ctx: &AliasCtx, args: AliasAppAddCmd) -> Result<()> {
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

fn cmd_app_rm(ctx: &AliasCtx, args: AliasAppRmCmd) -> Result<()> {
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

fn cmd_app_ls(ctx: &AliasCtx, args: AliasAppLsCmd) -> Result<()> {
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

fn cmd_app_scan(ctx: &AliasCtx, args: AliasAppScanCmd) -> Result<()> {
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

fn cmd_app_sync(ctx: &AliasCtx) -> Result<()> {
    let cfg = ctx.load()?;
    let all_entries = shim_gen::config_to_sync_entries(&cfg);
    let report = shim_gen::sync_all(
        &all_entries,
        &ctx.shims_dir,
        &ctx.template_path,
        &ctx.template_gui_path,
    )?;
    for (name, err) in report.errors {
        ui_println!("Warning: app sync error [{name}]: {err}");
    }
    let (registered, removed) = apppaths::sync_apppaths(&cfg)?;
    ui_println!("App aliases synced: apppaths +{registered} / -{removed}");
    Ok(())
}

fn split_csv_multi(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        for part in value.split(',') {
            let part = part.trim();
            if !part.is_empty() {
                out.push(part.to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}
