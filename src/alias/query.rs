use super::context::AliasCtx;
use super::*;

pub(super) fn cmd_ls(ctx: &AliasCtx, args: AliasLsArgs) -> Result<()> {
    let cfg = ctx.load()?;
    let show_cmd = args.r#type.as_deref().map(|v| v == "cmd").unwrap_or(true);
    let show_app = args.r#type.as_deref().map(|v| v == "app").unwrap_or(true);

    if args.json {
        let mut out = serde_json::Map::new();
        if show_cmd {
            let filtered: std::collections::BTreeMap<_, _> = cfg
                .alias
                .iter()
                .filter(|(_, a)| {
                    args.tag
                        .as_deref()
                        .map(|t| a.tags.iter().any(|tag| tag == t))
                        .unwrap_or(true)
                })
                .collect();
            out.insert("alias".to_string(), serde_json::to_value(&filtered)?);
        }
        if show_app {
            let filtered: std::collections::BTreeMap<_, _> = cfg
                .app
                .iter()
                .filter(|(_, a)| {
                    args.tag
                        .as_deref()
                        .map(|t| a.tags.iter().any(|tag| tag == t))
                        .unwrap_or(true)
                })
                .collect();
            out.insert("app".to_string(), serde_json::to_value(&filtered)?);
        }
        out_println!("{}", serde_json::Value::Object(out));
        return Ok(());
    }

    if show_cmd {
        let mut table = Table::new();
        apply_pretty_table_style(&mut table);
        table.set_header(["Name", "Command", "Mode", "Shells", "Desc"]);
        for (name, alias) in &cfg.alias {
            if let Some(tag) = &args.tag
                && !alias.tags.iter().any(|t| t == tag)
            {
                continue;
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
            if let Some(tag) = &args.tag
                && !app.tags.iter().any(|t| t == tag)
            {
                continue;
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

pub(super) fn cmd_find(ctx: &AliasCtx, args: AliasFindArgs) -> Result<()> {
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

pub(super) fn cmd_which(ctx: &AliasCtx, name: &str, app_only: bool) -> Result<()> {
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
