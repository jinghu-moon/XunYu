//! Alias 业务逻辑服务
//!
//! 封装别名管理操作，支持 CommandSpec 实现。
//! 10 个顶层子命令 + 6 个嵌套 app 子命令。

use std::path::{Path, PathBuf};

use crate::alias::config::{self, AppAlias, Config, ShellAlias};
use crate::alias::shim_gen;
use crate::xun_core::error::XunError;
use crate::xun_core::value::Value;

// ── 辅助 ──────────────────────────────────────────────────────────

/// 计算 alias 配置相关的所有路径。
fn alias_paths(config_override: Option<&str>) -> (PathBuf, PathBuf, PathBuf, PathBuf, PathBuf) {
    let override_path = config_override.map(Path::new);
    let config_path = config::config_path(override_path);
    let shims_dir = config::shims_dir(&config_path);
    let template_path = config::shim_template_path(&config_path);
    let template_gui_path = config::shim_gui_template_path(&config_path);
    let config_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    (config_path, shims_dir, template_path, template_gui_path, config_dir)
}

fn load_config(config_path: &Path) -> Result<Config, XunError> {
    config::migrate_legacy_if_needed(config_path)
        .map_err(|e| XunError::user(format!("legacy migration failed: {e}")))?;
    config::load(config_path)
        .map_err(|e| XunError::user(format!("failed to load alias config: {e}")))
}

fn save_config(config_path: &Path, cfg: &Config) -> Result<(), XunError> {
    config::save(config_path, cfg)
        .map_err(|e| XunError::user(format!("failed to save alias config: {e}")))
}

fn sync_shims(
    cfg: &Config,
    shims_dir: &Path,
    template_path: &Path,
    template_gui_path: &Path,
) -> Result<shim_gen::SyncReport, XunError> {
    let entries = shim_gen::config_to_sync_entries(cfg);
    shim_gen::sync_all(&entries, shims_dir, template_path, template_gui_path)
        .map_err(|e| XunError::user(format!("shim sync failed: {e}")))
}

fn sync_shells(cfg: &Config, config_dir: &Path) -> Result<(), XunError> {
    use crate::alias::shell::ShellBackend;
    use crate::alias::shell::cmd::CmdBackend;
    use crate::alias::shell::ps::PsBackend;

    let backends: Vec<Box<dyn ShellBackend>> = vec![
        Box::new(CmdBackend::new(config_dir)),
        Box::new(PsBackend::new(None)),
    ];
    for backend in backends {
        match backend.update(cfg) {
            Ok(_) => {}
            Err(err) => eprintln!("Warning: {} backend update failed: {err}", backend.name()),
        }
    }
    Ok(())
}

// ── Setup ─────────────────────────────────────────────────────────

/// 设置 alias 运行时（shim 模板 + shell 注入）。
pub fn setup_alias(
    config_override: Option<&str>,
    _no_cmd: bool,
    _no_ps: bool,
    _no_bash: bool,
    _no_nu: bool,
    _core_only: bool,
) -> Result<Value, XunError> {
    let (config_path, shims_dir, template_path, template_gui_path, config_dir) =
        alias_paths(config_override);

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| XunError::user(format!("failed to create config dir: {e}")))?;
    std::fs::create_dir_all(&shims_dir)
        .map_err(|e| XunError::user(format!("failed to create shims dir: {e}")))?;

    shim_gen::deploy_shim_templates(&template_path, &template_gui_path)
        .map_err(|e| XunError::user(format!("template deploy failed: {e}")))?;

    if !config_path.exists() {
        save_config(&config_path, &Config::default())?;
    }

    let cfg = load_config(&config_path)?;
    let report = sync_shims(&cfg, &shims_dir, &template_path, &template_gui_path)?;
    sync_shells(&cfg, &config_dir)?;

    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("config_path".into(), Value::String(config_path.display().to_string()));
    rec.insert("shims_dir".into(), Value::String(shims_dir.display().to_string()));
    rec.insert("shims_created".into(), Value::Int(report.created.len() as i64));
    rec.insert("shims_removed".into(), Value::Int(report.removed.len() as i64));
    Ok(Value::Record(rec))
}

// ── Add ───────────────────────────────────────────────────────────

/// 添加 shell 别名。
pub fn add_alias(
    config_override: Option<&str>,
    name: &str,
    command: &str,
    mode: &str,
    desc: Option<&str>,
    tags: &[String],
    shells: &[String],
    force: bool,
) -> Result<Value, XunError> {
    config::validate_alias_name(name)
        .map_err(|e| XunError::user(format!("invalid alias name: {e}")))?;

    let alias_mode: config::AliasMode = mode.parse().map_err(|e: String| XunError::user(e))?;

    let (config_path, shims_dir, template_path, template_gui_path, config_dir) =
        alias_paths(config_override);
    let mut cfg = load_config(&config_path)?;

    if cfg.alias.contains_key(name) && !force {
        return Err(XunError::user(format!(
            "alias '{name}' already exists; use --force to overwrite"
        )));
    }

    let alias = ShellAlias {
        command: command.to_string(),
        desc: desc.map(String::from),
        tags: tags.to_vec(),
        shells: shells.to_vec(),
        mode: alias_mode,
    };

    shim_gen::sync_shell_alias(&shims_dir, &template_path, &template_gui_path, name, &alias)
        .map_err(|e| XunError::user(format!("shim sync failed: {e}")))?;

    cfg.alias.insert(name.to_string(), alias);
    save_config(&config_path, &cfg)?;
    sync_shells(&cfg, &config_dir)?;

    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("name".into(), Value::String(name.to_string()));
    rec.insert("command".into(), Value::String(command.to_string()));
    Ok(Value::Record(rec))
}

// ── Rm ────────────────────────────────────────────────────────────

/// 删除别名。
pub fn rm_alias(config_override: Option<&str>, names: &[String]) -> Result<Value, XunError> {
    let (config_path, shims_dir, _template_path, _template_gui_path, config_dir) =
        alias_paths(config_override);
    let mut cfg = load_config(&config_path)?;

    let mut removed = 0usize;
    for name in names {
        if cfg.alias.remove(name).is_some() || cfg.app.remove(name).is_some() {
            shim_gen::remove_shim(&shims_dir, name)
                .map_err(|e| XunError::user(format!("shim removal failed: {e}")))?;
            removed += 1;
        }
    }

    save_config(&config_path, &cfg)?;
    sync_shells(&cfg, &config_dir)?;

    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("removed".into(), Value::Int(removed as i64));
    Ok(Value::Record(rec))
}

// ── List ──────────────────────────────────────────────────────────

/// 列出别名。
pub fn list_alias(
    config_override: Option<&str>,
    type_filter: Option<&str>,
    tag: Option<&str>,
    json: bool,
) -> Result<Value, XunError> {
    let (config_path, ..) = alias_paths(config_override);
    let cfg = load_config(&config_path)?;

    let show_cmd = type_filter.map(|t| t == "cmd").unwrap_or(true);
    let show_app = type_filter.map(|t| t == "app").unwrap_or(true);

    if json {
        let mut map = serde_json::Map::new();
        if show_cmd {
            let filtered: std::collections::BTreeMap<_, _> = cfg
                .alias
                .iter()
                .filter(|(_, a)| {
                    tag.map(|t| a.tags.iter().any(|tg| tg == t))
                        .unwrap_or(true)
                })
                .collect();
            map.insert("alias".to_string(), serde_json::to_value(&filtered).unwrap_or_default());
        }
        if show_app {
            let filtered: std::collections::BTreeMap<_, _> = cfg
                .app
                .iter()
                .filter(|(_, a)| {
                    tag.map(|t| a.tags.iter().any(|tg| tg == t))
                        .unwrap_or(true)
                })
                .collect();
            map.insert("app".to_string(), serde_json::to_value(&filtered).unwrap_or_default());
        }
        let json_str = serde_json::to_string_pretty(&map)
            .map_err(|e| XunError::user(format!("json serialize failed: {e}")))?;
        return Ok(Value::String(json_str));
    }

    let mut items = Vec::new();
    if show_cmd {
        for (name, a) in &cfg.alias {
            if tag.map(|t| a.tags.iter().any(|tg| tg == t)).unwrap_or(true) {
                let mut rec = crate::xun_core::value::Record::new();
                rec.insert("name".into(), Value::String(name.clone()));
                rec.insert("command".into(), Value::String(a.command.clone()));
                rec.insert("mode".into(), Value::String(format!("{:?}", a.mode)));
                rec.insert("desc".into(), Value::String(a.desc.clone().unwrap_or_default()));
                rec.insert("tags".into(), Value::String(a.tags.join(",")));
                items.push(Value::Record(rec));
            }
        }
    }
    if show_app {
        for (name, a) in &cfg.app {
            if tag.map(|t| a.tags.iter().any(|tg| tg == t)).unwrap_or(true) {
                let mut rec = crate::xun_core::value::Record::new();
                rec.insert("name".into(), Value::String(name.clone()));
                rec.insert("exe".into(), Value::String(a.exe.clone()));
                rec.insert("desc".into(), Value::String(a.desc.clone().unwrap_or_default()));
                rec.insert("tags".into(), Value::String(a.tags.join(",")));
                items.push(Value::Record(rec));
            }
        }
    }

    Ok(Value::List(items))
}

// ── Find ──────────────────────────────────────────────────────────

/// 模糊搜索别名。
pub fn find_alias(config_override: Option<&str>, keyword: &str) -> Result<Value, XunError> {
    let (config_path, ..) = alias_paths(config_override);
    let cfg = load_config(&config_path)?;

    let kw = keyword.to_ascii_lowercase();
    let mut items = Vec::new();

    for (name, a) in &cfg.alias {
        let score = crate::alias::output::fuzzy_score(name, &a.command, a.desc.as_deref().unwrap_or(""), &kw);
        if score > 0 {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("name".into(), Value::String(name.clone()));
            rec.insert("command".into(), Value::String(a.command.clone()));
            rec.insert("score".into(), Value::Int(score as i64));
            items.push(Value::Record(rec));
        }
    }
    for (name, a) in &cfg.app {
        let score = crate::alias::output::fuzzy_score(name, &a.exe, a.desc.as_deref().unwrap_or(""), &kw);
        if score > 0 {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("name".into(), Value::String(name.clone()));
            rec.insert("exe".into(), Value::String(a.exe.clone()));
            rec.insert("score".into(), Value::Int(score as i64));
            items.push(Value::Record(rec));
        }
    }

    items.sort_by(|a, b| {
        let sa = if let Value::Record(r) = a { r.get("score").and_then(|v| if let Value::Int(n) = v { Some(*n) } else { None }).unwrap_or(0) } else { 0 };
        let sb = if let Value::Record(r) = b { r.get("score").and_then(|v| if let Value::Int(n) = v { Some(*n) } else { None }).unwrap_or(0) } else { 0 };
        sb.cmp(&sa)
    });

    Ok(Value::List(items))
}

// ── Which ─────────────────────────────────────────────────────────

/// 查看别名详情。
pub fn which_alias(config_override: Option<&str>, name: &str) -> Result<Value, XunError> {
    let (config_path, shims_dir, ..) = alias_paths(config_override);
    let cfg = load_config(&config_path)?;

    if let Some(a) = cfg.alias.get(name) {
        let mut rec = crate::xun_core::value::Record::new();
        rec.insert("type".into(), Value::String("shell".to_string()));
        rec.insert("name".into(), Value::String(name.to_string()));
        rec.insert("command".into(), Value::String(a.command.clone()));
        rec.insert("mode".into(), Value::String(format!("{:?}", a.mode)));
        rec.insert("desc".into(), Value::String(a.desc.clone().unwrap_or_default()));
        rec.insert("tags".into(), Value::String(a.tags.join(",")));
        rec.insert("shells".into(), Value::String(a.shells.join(",")));
        let exe_path = shims_dir.join(format!("{name}.exe"));
        rec.insert("shim_path".into(), Value::String(exe_path.display().to_string()));
        rec.insert("shim_exists".into(), Value::Bool(exe_path.exists()));
        return Ok(Value::Record(rec));
    }

    if let Some(a) = cfg.app.get(name) {
        let mut rec = crate::xun_core::value::Record::new();
        rec.insert("type".into(), Value::String("app".to_string()));
        rec.insert("name".into(), Value::String(name.to_string()));
        rec.insert("exe".into(), Value::String(a.exe.clone()));
        if let Some(args) = &a.args {
            rec.insert("args".into(), Value::String(args.clone()));
        }
        rec.insert("desc".into(), Value::String(a.desc.clone().unwrap_or_default()));
        rec.insert("tags".into(), Value::String(a.tags.join(",")));
        let exe_path = shims_dir.join(format!("{name}.exe"));
        rec.insert("shim_path".into(), Value::String(exe_path.display().to_string()));
        rec.insert("shim_exists".into(), Value::Bool(exe_path.exists()));
        return Ok(Value::Record(rec));
    }

    Err(XunError::NotFound(format!("alias '{name}' not found")))
}

// ── Sync ──────────────────────────────────────────────────────────

/// 同步 shim + shell + app paths。
pub fn sync_alias(config_override: Option<&str>) -> Result<Value, XunError> {
    let (config_path, shims_dir, template_path, template_gui_path, config_dir) =
        alias_paths(config_override);
    let cfg = load_config(&config_path)?;

    let report = sync_shims(&cfg, &shims_dir, &template_path, &template_gui_path)?;
    sync_shells(&cfg, &config_dir)?;
    let (registered, removed) = crate::alias::apppaths::sync_apppaths(&cfg)
        .map_err(|e| XunError::user(format!("apppaths sync failed: {e}")))?;

    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("shims_created".into(), Value::Int(report.created.len() as i64));
    rec.insert("shims_removed".into(), Value::Int(report.removed.len() as i64));
    rec.insert("apppaths_registered".into(), Value::Int(registered as i64));
    rec.insert("apppaths_removed".into(), Value::Int(removed as i64));
    Ok(Value::Record(rec))
}

// ── Export ────────────────────────────────────────────────────────

/// 导出别名配置。
pub fn export_alias(config_override: Option<&str>, output: Option<&str>) -> Result<Value, XunError> {
    let (config_path, ..) = alias_paths(config_override);
    let cfg = load_config(&config_path)?;

    let content = toml::to_string_pretty(&cfg)
        .map_err(|e| XunError::user(format!("toml serialize failed: {e}")))?;

    if let Some(path) = output {
        std::fs::write(path, &content)
            .map_err(|e| XunError::user(format!("failed to write {path}: {e}")))?;
        let mut rec = crate::xun_core::value::Record::new();
        rec.insert("path".into(), Value::String(path.to_string()));
        rec.insert("size".into(), Value::Int(content.len() as i64));
        Ok(Value::Record(rec))
    } else {
        Ok(Value::String(content))
    }
}

// ── Import ────────────────────────────────────────────────────────

/// 导入别名配置。
pub fn import_alias(
    config_override: Option<&str>,
    file: &str,
    force: bool,
) -> Result<Value, XunError> {
    let (config_path, shims_dir, template_path, template_gui_path, config_dir) =
        alias_paths(config_override);

    let content = std::fs::read_to_string(file)
        .map_err(|e| XunError::user(format!("failed to read {file}: {e}")))?;
    let imported: Config = toml::from_str(&content)
        .map_err(|e| XunError::user(format!("toml parse failed: {e}")))?;

    let mut cfg = load_config(&config_path)?;

    let mut added = 0usize;
    let mut skipped = 0usize;

    for (name, alias) in imported.alias {
        if cfg.alias.contains_key(&name) && !force {
            skipped += 1;
            continue;
        }
        cfg.alias.insert(name, alias);
        added += 1;
    }
    for (name, alias) in imported.app {
        if cfg.app.contains_key(&name) && !force {
            skipped += 1;
            continue;
        }
        cfg.app.insert(name, alias);
        added += 1;
    }

    save_config(&config_path, &cfg)?;
    let _ = sync_shims(&cfg, &shims_dir, &template_path, &template_gui_path)?;
    sync_shells(&cfg, &config_dir)?;

    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("added".into(), Value::Int(added as i64));
    rec.insert("skipped".into(), Value::Int(skipped as i64));
    Ok(Value::Record(rec))
}

// ── App Add ───────────────────────────────────────────────────────

/// 添加应用别名。
pub fn app_add(
    config_override: Option<&str>,
    name: &str,
    exe: &str,
    args: Option<&str>,
    desc: Option<&str>,
    tags: &[String],
    no_apppaths: bool,
    force: bool,
) -> Result<Value, XunError> {
    config::validate_alias_name(name)
        .map_err(|e| XunError::user(format!("invalid alias name: {e}")))?;

    let (config_path, shims_dir, template_path, template_gui_path, _config_dir) =
        alias_paths(config_override);
    let mut cfg = load_config(&config_path)?;

    if cfg.app.contains_key(name) && !force {
        return Err(XunError::user(format!(
            "app alias '{name}' already exists; use --force to overwrite"
        )));
    }

    let alias = AppAlias {
        exe: exe.to_string(),
        args: args.map(String::from),
        desc: desc.map(String::from),
        tags: tags.to_vec(),
        register_apppaths: !no_apppaths,
    };

    shim_gen::sync_app_alias(&shims_dir, &template_path, &template_gui_path, name, &alias)
        .map_err(|e| XunError::user(format!("shim sync failed: {e}")))?;

    cfg.app.insert(name.to_string(), alias);
    save_config(&config_path, &cfg)?;

    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("name".into(), Value::String(name.to_string()));
    rec.insert("exe".into(), Value::String(exe.to_string()));
    Ok(Value::Record(rec))
}

// ── App Rm ────────────────────────────────────────────────────────

/// 删除应用别名。
pub fn app_rm(config_override: Option<&str>, names: &[String]) -> Result<Value, XunError> {
    rm_alias(config_override, names)
}

// ── App List ──────────────────────────────────────────────────────

/// 列出应用别名。
pub fn app_ls(config_override: Option<&str>, json: bool) -> Result<Value, XunError> {
    list_alias(config_override, Some("app"), None, json)
}

// ── App Scan ──────────────────────────────────────────────────────

/// 扫描已安装应用。
pub fn app_scan(
    config_override: Option<&str>,
    source: &str,
    filter: Option<&str>,
    json: bool,
    add_all: bool,
    no_cache: bool,
) -> Result<Value, XunError> {
    let scan_source = crate::alias::scanner::ScanSource::from_str(source)
        .ok_or_else(|| XunError::user(format!("invalid scan source: {source}")))?;

    let entries = crate::alias::scanner::scan(scan_source, filter, no_cache);

    if add_all {
        let (config_path, shims_dir, template_path, template_gui_path, _config_dir) =
            alias_paths(config_override);
        let mut cfg = load_config(&config_path)?;

        let mut added = 0usize;
        for entry in &entries {
            let alias_name = crate::alias::scanner::auto_alias(&entry.display_name);
            if cfg.app.contains_key(&alias_name) {
                continue;
            }
            let alias = AppAlias {
                exe: entry.exe_path.clone(),
                args: None,
                desc: Some(entry.display_name.clone()),
                tags: vec![],
                register_apppaths: true,
            };
            cfg.app.insert(alias_name, alias);
            added += 1;
        }

        save_config(&config_path, &cfg)?;
        let _ = sync_shims(&cfg, &shims_dir, &template_path, &template_gui_path)?;

        let mut rec = crate::xun_core::value::Record::new();
        rec.insert("scanned".into(), Value::Int(entries.len() as i64));
        rec.insert("added".into(), Value::Int(added as i64));
        return Ok(Value::Record(rec));
    }

    if json {
        let json_str = serde_json::to_string_pretty(&entries)
            .map_err(|e| XunError::user(format!("json serialize failed: {e}")))?;
        return Ok(Value::String(json_str));
    }

    let items: Vec<Value> = entries
        .iter()
        .map(|e| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("name".into(), Value::String(e.name.clone()));
            rec.insert("display_name".into(), Value::String(e.display_name.clone()));
            rec.insert("exe_path".into(), Value::String(e.exe_path.clone()));
            Value::Record(rec)
        })
        .collect();

    Ok(Value::List(items))
}

// ── App Which ─────────────────────────────────────────────────────

/// 查看应用别名详情。
pub fn app_which(config_override: Option<&str>, name: &str) -> Result<Value, XunError> {
    which_alias(config_override, name)
}

// ── App Sync ──────────────────────────────────────────────────────

/// 同步应用别名。
pub fn app_sync(config_override: Option<&str>) -> Result<Value, XunError> {
    let (config_path, shims_dir, template_path, template_gui_path, _config_dir) =
        alias_paths(config_override);
    let cfg = load_config(&config_path)?;

    let entries = shim_gen::config_to_sync_entries(&cfg);
    let report = shim_gen::sync_all(&entries, &shims_dir, &template_path, &template_gui_path)
        .map_err(|e| XunError::user(format!("sync failed: {e}")))?;

    let (registered, removed) = crate::alias::apppaths::sync_apppaths(&cfg)
        .map_err(|e| XunError::user(format!("apppaths sync failed: {e}")))?;

    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("shims_created".into(), Value::Int(report.created.len() as i64));
    rec.insert("shims_removed".into(), Value::Int(report.removed.len() as i64));
    rec.insert("apppaths_registered".into(), Value::Int(registered as i64));
    rec.insert("apppaths_removed".into(), Value::Int(removed as i64));
    Ok(Value::Record(rec))
}
