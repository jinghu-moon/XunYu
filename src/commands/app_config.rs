use std::process::Command;

use crate::cli::{ConfigCmd, ConfigEditCmd, ConfigGetCmd, ConfigSetCmd, ConfigSubCommand};
use crate::output::{CliError, CliResult};

fn config_path() -> std::path::PathBuf {
    crate::config::config_path()
}

fn load_json(path: &std::path::Path) -> serde_json::Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn save_json(path: &std::path::Path, v: &serde_json::Value) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    let s = serde_json::to_string_pretty(v).unwrap_or_else(|_| "{}".to_string());
    std::fs::write(&tmp, s)?;
    std::fs::rename(&tmp, path)
}

fn parse_value(raw: &str) -> serde_json::Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return serde_json::Value::String(String::new());
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

fn get_by_dot<'a>(root: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    let mut cur = root;
    for part in key.split('.').filter(|s| !s.is_empty()) {
        match cur {
            serde_json::Value::Object(map) => {
                cur = map.get(part)?;
            }
            _ => return None,
        }
    }
    Some(cur)
}

fn set_by_dot(root: &mut serde_json::Value, key: &str, value: serde_json::Value) -> CliResult {
    let parts: Vec<&str> = key.split('.').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Err(CliError::with_details(
            2,
            "Key is empty.".to_string(),
            &["Fix: Use a dot path, e.g. `proxy.defaultUrl`."],
        ));
    }

    let mut cur = root;
    for part in &parts[..parts.len() - 1] {
        if !cur.is_object() {
            *cur = serde_json::json!({});
        }
        let obj = cur.as_object_mut().expect("object");
        if !obj.contains_key(*part) || !obj[*part].is_object() {
            obj.insert((*part).to_string(), serde_json::json!({}));
        }
        cur = obj.get_mut(*part).expect("child");
    }

    if !cur.is_object() {
        *cur = serde_json::json!({});
    }
    let obj = cur.as_object_mut().expect("object");
    obj.insert(parts[parts.len() - 1].to_string(), value);
    Ok(())
}

pub(crate) fn cmd_config(args: ConfigCmd) -> CliResult {
    match args.cmd {
        ConfigSubCommand::Get(a) => cmd_get(a),
        ConfigSubCommand::Set(a) => cmd_set(a),
        ConfigSubCommand::Edit(a) => cmd_edit(a),
    }
}

fn cmd_get(args: ConfigGetCmd) -> CliResult {
    let path = config_path();
    let v = load_json(&path);
    let Some(val) = get_by_dot(&v, &args.key) else {
        return Err(CliError::with_details(
            2,
            format!("Config key not found: {}", args.key),
            &[
                "Hint: Run `xun config edit` to inspect the config file.",
                "Fix: Use `xun config set <key> <value>` to create it.",
            ],
        ));
    };
    out_println!("{}", val);
    Ok(())
}

fn cmd_set(args: ConfigSetCmd) -> CliResult {
    let path = config_path();
    let mut v = load_json(&path);
    set_by_dot(&mut v, &args.key, parse_value(&args.value))?;
    save_json(&path, &v).map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
    ui_println!("Config updated: {}", path.display());
    Ok(())
}

fn cmd_edit(_args: ConfigEditCmd) -> CliResult {
    let path = config_path();
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "notepad".to_string());
    let status = Command::new(editor)
        .arg(&path)
        .status()
        .map_err(|e| CliError::new(1, format!("Failed to launch editor: {e}")))?;
    if !status.success() {
        return Err(CliError::new(
            status.code().unwrap_or(1),
            "Editor exited with error.".to_string(),
        ));
    }
    Ok(())
}
