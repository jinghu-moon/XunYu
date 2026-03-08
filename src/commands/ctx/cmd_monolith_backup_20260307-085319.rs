use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;

use comfy_table::{Attribute, Cell, Color, Table};

use crate::cli::{
    CtxCmd, CtxDelCmd, CtxListCmd, CtxOffCmd, CtxRenameCmd, CtxSetCmd, CtxShowCmd, CtxSubCommand,
    CtxUseCmd,
};
use crate::commands::proxy::config::{del_proxy, load_proxy_state, set_proxy};
use crate::ctx_store::{
    CtxProxyMode, CtxProxyState, CtxSession, ctx_store_path, load_session, load_store,
    save_session, save_store, session_path_from_env,
};
use crate::model::{ListFormat, parse_list_format};
use crate::output::{
    CliError, CliResult, apply_pretty_table_style, emit_warning, prefer_table_output, print_table,
};
use crate::util::parse_tags;

use super::DEFAULT_NOPROXY;
use super::env::{load_env_file, parse_env_kv};
use super::proxy::{
    apply_proxy_updates, emit_proxy_off, emit_proxy_set, normalize_proxy_url, proxy_summary,
};
use super::session::active_profile_name;
use super::validate::validate_name;

pub(crate) fn cmd_ctx(args: CtxCmd) -> CliResult {
    match args.cmd {
        CtxSubCommand::Set(a) => cmd_set(a),
        CtxSubCommand::Use(a) => cmd_use(a),
        CtxSubCommand::Off(a) => cmd_off(a),
        CtxSubCommand::List(a) => cmd_list(a),
        CtxSubCommand::Show(a) => cmd_show(a),
        CtxSubCommand::Del(a) => cmd_del(a),
        CtxSubCommand::Rename(a) => cmd_rename(a),
    }
}

fn cmd_set(args: CtxSetCmd) -> CliResult {
    validate_name(&args.name)?;

    let path = ctx_store_path();
    let mut store = load_store(&path);
    let is_new = !store.profiles.contains_key(&args.name);

    let mut profile = store.profiles.get(&args.name).cloned().unwrap_or_default();

    if is_new && args.path.is_none() && profile.path.trim().is_empty() {
        return Err(CliError::with_details(
            2,
            "Missing --path for new profile.".to_string(),
            &["Fix: Use `xun ctx set <name> --path <dir>`."],
        ));
    }

    if let Some(p) = args.path.as_ref() {
        profile.path = p.clone();
    }
    if profile.path.trim().is_empty() {
        return Err(CliError::with_details(
            2,
            "Profile path is empty.".to_string(),
            &["Fix: Use `xun ctx set <name> --path <dir>`."],
        ));
    }

    apply_proxy_updates(&mut profile.proxy, &args)?;

    if let Some(tag_raw) = args.tag {
        if tag_raw.trim() == "-" {
            profile.tags.clear();
        } else {
            profile.tags = parse_tags(&tag_raw);
        }
    }

    let mut env_updates = Vec::new();
    if let Some(file) = args.env_file {
        let items = load_env_file(Path::new(&file))?;
        env_updates.extend(items);
    }
    for raw in args.env {
        env_updates.push(parse_env_kv(&raw)?);
    }
    if !env_updates.is_empty() {
        for (k, v) in env_updates {
            profile.env.insert(k, v);
        }
    }

    store.profiles.insert(args.name.clone(), profile.clone());

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    save_store(&path, &store)
        .map_err(|e| CliError::new(1, format!("Failed to save ctx store: {e}")))?;

    if !Path::new(&profile.path).exists() {
        emit_warning(
            format!("Path does not exist: {}", profile.path),
            &["Hint: Create the path first, or double-check the spelling."],
        );
    }

    if is_new {
        ui_println!("Saved ctx '{}' -> {}", args.name, profile.path);
    } else {
        ui_println!("Updated ctx '{}' -> {}", args.name, profile.path);
    }
    Ok(())
}

fn cmd_list(args: CtxListCmd) -> CliResult {
    let path = ctx_store_path();
    let store = load_store(&path);
    let active = active_profile_name();

    let mut items: Vec<(&String, &crate::ctx_store::CtxProfile)> = store.profiles.iter().collect();
    items.sort_by_key(|(k, _)| k.to_lowercase());

    let mut format = parse_list_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }

    if format == ListFormat::Json {
        let arr: Vec<serde_json::Value> = items
            .iter()
            .map(|(name, p)| {
                serde_json::json!({
                    "name": name,
                    "path": p.path,
                    "tags": p.tags,
                    "proxy": p.proxy,
                    "env": p.env,
                    "active": active.as_deref() == Some(name.as_str()),
                })
            })
            .collect();
        out_println!(
            "{}",
            serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
        );
        return Ok(());
    }

    if format == ListFormat::Tsv {
        for (name, p) in items {
            let tags = p.tags.join(",");
            let proxy = proxy_summary(&p.proxy);
            let active_mark = if active.as_deref() == Some(name.as_str()) {
                "yes"
            } else {
                ""
            };
            out_println!("{name}\t{}\t{tags}\t{proxy}\t{active_mark}", p.path);
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Name")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Path")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
        Cell::new("Tags")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Proxy")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Active")
            .add_attribute(Attribute::Bold)
            .fg(Color::Blue),
    ]);

    for (name, p) in items {
        let tags = if p.tags.is_empty() {
            Cell::new("-")
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        } else {
            Cell::new(p.tags.join(",")).fg(Color::Yellow)
        };
        let active_mark = if active.as_deref() == Some(name.as_str()) {
            Cell::new("yes").fg(Color::Green)
        } else {
            Cell::new("")
        };
        table.add_row(vec![
            Cell::new(name.as_str())
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new(p.path.as_str()).fg(Color::DarkGrey),
            tags,
            Cell::new(proxy_summary(&p.proxy)).fg(Color::Green),
            active_mark,
        ]);
    }
    print_table(&table);
    Ok(())
}

fn cmd_show(args: CtxShowCmd) -> CliResult {
    let name = match args.name {
        Some(n) => n,
        None => active_profile_name().ok_or_else(|| {
            CliError::with_details(
                2,
                "No active profile.".to_string(),
                &[
                    "Hint: Run `xun ctx list` to see available profiles.",
                    "Fix: Activate one with `xun ctx use <name>`.",
                ],
            )
        })?,
    };

    let path = ctx_store_path();
    let store = load_store(&path);
    let profile = store.profiles.get(&name).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Profile not found: {}", name),
            &["Hint: Run `xun ctx list` to see existing profiles."],
        )
    })?;

    let mut format = parse_list_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }

    if format == ListFormat::Json {
        let obj = serde_json::json!({
            "name": name,
            "path": profile.path,
            "tags": profile.tags,
            "proxy": profile.proxy,
            "env": profile.env,
        });
        out_println!(
            "{}",
            serde_json::to_string(&obj).unwrap_or_else(|_| "{}".to_string())
        );
        return Ok(());
    }

    if format == ListFormat::Tsv {
        out_println!("name\t{name}");
        out_println!("path\t{}", profile.path);
        out_println!("tags\t{}", profile.tags.join(","));
        out_println!("proxy\t{}", proxy_summary(&profile.proxy));
        if let Some(url) = &profile.proxy.url {
            out_println!("proxy_url\t{url}");
        }
        if let Some(np) = &profile.proxy.noproxy {
            out_println!("no_proxy\t{np}");
        }
        if !profile.env.is_empty() {
            for (k, v) in &profile.env {
                out_println!("env\t{}={}", k, v);
            }
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Key")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Value")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
    ]);
    table.add_row(vec![Cell::new("name"), Cell::new(name.as_str())]);
    table.add_row(vec![Cell::new("path"), Cell::new(profile.path.as_str())]);
    let tags_value = if profile.tags.is_empty() {
        "-".to_string()
    } else {
        profile.tags.join(",")
    };
    table.add_row(vec![Cell::new("tags"), Cell::new(tags_value)]);
    table.add_row(vec![
        Cell::new("proxy"),
        Cell::new(proxy_summary(&profile.proxy)),
    ]);
    if let Some(url) = &profile.proxy.url {
        table.add_row(vec![Cell::new("proxy_url"), Cell::new(url.as_str())]);
    }
    if let Some(np) = &profile.proxy.noproxy {
        table.add_row(vec![Cell::new("no_proxy"), Cell::new(np.as_str())]);
    }
    if profile.env.is_empty() {
        table.add_row(vec![Cell::new("env"), Cell::new("-")]);
    } else {
        for (k, v) in &profile.env {
            table.add_row(vec![Cell::new("env"), Cell::new(format!("{k}={v}"))]);
        }
    }
    print_table(&table);
    Ok(())
}

fn cmd_del(args: CtxDelCmd) -> CliResult {
    let path = ctx_store_path();
    let mut store = load_store(&path);
    if store.profiles.remove(&args.name).is_none() {
        emit_warning(
            format!("Profile '{}' not found.", args.name),
            &["Hint: Run `xun ctx list` to see existing profiles."],
        );
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    save_store(&path, &store)
        .map_err(|e| CliError::new(1, format!("Failed to save ctx store: {e}")))?;
    ui_println!("Deleted ctx '{}'.", args.name);
    Ok(())
}

fn cmd_rename(args: CtxRenameCmd) -> CliResult {
    validate_name(&args.new)?;
    let path = ctx_store_path();
    let mut store = load_store(&path);
    if args.old == args.new {
        emit_warning("Same name, nothing to do.", &[]);
        return Ok(());
    }
    if !store.profiles.contains_key(&args.old) {
        emit_warning(
            format!("Profile '{}' not found.", args.old),
            &["Hint: Run `xun ctx list` to see existing profiles."],
        );
        return Ok(());
    }
    if store.profiles.contains_key(&args.new) {
        emit_warning(
            format!("Profile '{}' already exists.", args.new),
            &["Fix: Choose a different name, or delete the existing one first."],
        );
        return Ok(());
    }
    if let Some(profile) = store.profiles.remove(&args.old) {
        store.profiles.insert(args.new.clone(), profile);
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    save_store(&path, &store)
        .map_err(|e| CliError::new(1, format!("Failed to save ctx store: {e}")))?;
    ui_println!("Renamed ctx '{}' -> '{}'.", args.old, args.new);
    Ok(())
}

fn cmd_use(args: CtxUseCmd) -> CliResult {
    let store_path = ctx_store_path();
    let store = load_store(&store_path);
    let profile = store.profiles.get(&args.name).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Profile not found: {}", args.name),
            &["Hint: Run `xun ctx list` to see existing profiles."],
        )
    })?;

    if !Path::new(&profile.path).exists() {
        return Err(CliError::with_details(
            2,
            format!("Path not found: {}", profile.path),
            &[
                "Hint: Run `xun ctx show <name>` to check profile details.",
                "Fix: Update with `xun ctx set <name> --path <dir>`.",
            ],
        ));
    }

    let session_path = session_path_from_env().ok_or_else(|| {
        CliError::with_details(
            2,
            "XUN_CTX_STATE is not set.".to_string(),
            &[
                "Hint: Load shell integration with `xun init <shell>`.",
                "Fix: Re-open your shell after running `xun init`.",
            ],
        )
    })?;

    let previous_dir = env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| ".".to_string());

    let mut previous_env: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut env_keys: HashSet<String> = HashSet::new();
    env_keys.insert("XUN_DEFAULT_TAG".to_string());
    env_keys.insert("XUN_CTX".to_string());
    for k in profile.env.keys() {
        env_keys.insert(k.clone());
    }
    for key in env_keys {
        previous_env.insert(key.clone(), env::var(&key).ok());
    }

    let env_proxy = env::var("HTTP_PROXY")
        .or_else(|_| env::var("http_proxy"))
        .ok()
        .filter(|v| !v.trim().is_empty());
    let env_noproxy = env::var("NO_PROXY")
        .or_else(|_| env::var("no_proxy"))
        .ok()
        .filter(|v| !v.trim().is_empty());

    let previous_proxy = if let Some(url) = env_proxy {
        Some(CtxProxyState {
            url,
            noproxy: env_noproxy,
        })
    } else {
        load_proxy_state().map(|p| CtxProxyState {
            url: p.url,
            noproxy: p.noproxy,
        })
    };

    let proxy_changed = matches!(profile.proxy.mode, CtxProxyMode::Set | CtxProxyMode::Off);
    let session = CtxSession {
        active: args.name.clone(),
        previous_dir,
        previous_env,
        previous_proxy,
        proxy_changed,
    };
    if let Some(parent) = session_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    save_session(&session_path, &session)
        .map_err(|e| CliError::new(1, format!("Failed to write ctx session: {e}")))?;

    out_println!("__CD__:{}", profile.path);

    match profile.proxy.mode {
        CtxProxyMode::Set => {
            let url = profile.proxy.url.as_ref().ok_or_else(|| {
                CliError::with_details(
                    2,
                    format!("Profile '{}' has proxy mode set but no url.", args.name),
                    &["Fix: Update with `xun ctx set <name> --proxy <url>`."],
                )
            })?;
            let url = normalize_proxy_url(url);
            let noproxy = profile.proxy.noproxy.as_deref().unwrap_or(DEFAULT_NOPROXY);
            emit_proxy_set(&url, noproxy);
            set_proxy(&url, noproxy, None, None);
        }
        CtxProxyMode::Off => {
            emit_proxy_off();
            del_proxy(None, None);
        }
        CtxProxyMode::Keep => {}
    }

    if profile.tags.is_empty() {
        out_println!("__ENV_UNSET__:XUN_DEFAULT_TAG");
    } else {
        out_println!("__ENV_SET__:XUN_DEFAULT_TAG={}", profile.tags.join(","));
    }
    out_println!("__ENV_SET__:XUN_CTX={}", args.name);

    for (k, v) in &profile.env {
        out_println!("__ENV_SET__:{k}={v}");
    }

    Ok(())
}

fn cmd_off(_args: CtxOffCmd) -> CliResult {
    let Some(session_path) = session_path_from_env() else {
        ui_println!("No active profile.");
        return Ok(());
    };
    let Some(session) = load_session(&session_path) else {
        ui_println!("No active profile.");
        return Ok(());
    };

    if session.proxy_changed {
        if let Some(prev) = &session.previous_proxy {
            let url = normalize_proxy_url(&prev.url);
            let noproxy = prev.noproxy.as_deref().unwrap_or(DEFAULT_NOPROXY);
            emit_proxy_set(&url, noproxy);
            set_proxy(&url, noproxy, None, None);
        } else {
            emit_proxy_off();
            del_proxy(None, None);
        }
    }

    for (k, v) in &session.previous_env {
        match v {
            Some(val) => out_println!("__ENV_SET__:{k}={val}"),
            None => out_println!("__ENV_UNSET__:{k}"),
        }
    }

    if !session.previous_dir.trim().is_empty() {
        if Path::new(&session.previous_dir).exists() {
            out_println!("__CD__:{}", session.previous_dir);
        } else {
            emit_warning(
                format!("Previous directory not found: {}", session.previous_dir),
                &["Hint: The directory may have been moved or deleted."],
            );
        }
    }

    out_println!("__ENV_UNSET__:{}", crate::ctx_store::CTX_STATE_ENV);
    let _ = fs::remove_file(&session_path);
    Ok(())
}
