use std::fs;
use std::path::PathBuf;

use chrono::Local;

use super::config::{EnvCoreConfig, ensure_dir};
use super::registry;
use super::types::{
    EnvError, EnvProfile, EnvProfileMeta, EnvResult, EnvScope, EnvVar, SnapshotEntry,
};
use super::var_type::infer_var_kind;

pub fn profile_dir(cfg: &EnvCoreConfig) -> EnvResult<PathBuf> {
    let dir = cfg.profile_dir();
    ensure_dir(&dir)?;
    Ok(dir)
}

pub fn list_profiles(cfg: &EnvCoreConfig) -> EnvResult<Vec<EnvProfileMeta>> {
    let dir = profile_dir(cfg)?;
    let mut out = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(v) => v,
        Err(e) => return Err(EnvError::Io(e)),
    };

    for item in entries {
        let item = item.map_err(EnvError::Io)?;
        let path = item.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        let content = match fs::read_to_string(&path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let profile = match serde_json::from_str::<EnvProfile>(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };
        out.push(EnvProfileMeta {
            name: profile.name,
            scope: profile.scope,
            created_at: profile.created_at,
            var_count: profile.vars.len(),
            path,
        });
    }
    out.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
}

pub fn load_profile(cfg: &EnvCoreConfig, name: &str) -> EnvResult<EnvProfile> {
    let path = profile_file_path(cfg, name)?;
    let content = fs::read_to_string(&path).map_err(EnvError::Io)?;
    let profile = serde_json::from_str::<EnvProfile>(&content)?;
    Ok(profile)
}

pub fn save_profile(cfg: &EnvCoreConfig, profile: &EnvProfile) -> EnvResult<EnvProfileMeta> {
    let path = profile_file_path(cfg, &profile.name)?;
    let data = serde_json::to_string_pretty(profile)?;
    fs::write(&path, data).map_err(EnvError::Io)?;
    Ok(EnvProfileMeta {
        name: profile.name.clone(),
        scope: profile.scope,
        created_at: profile.created_at.clone(),
        var_count: profile.vars.len(),
        path,
    })
}

pub fn delete_profile(cfg: &EnvCoreConfig, name: &str) -> EnvResult<bool> {
    let path = profile_file_path(cfg, name)?;
    match fs::remove_file(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(EnvError::Io(e)),
    }
}

pub fn capture_profile(
    cfg: &EnvCoreConfig,
    name: &str,
    scope: EnvScope,
) -> EnvResult<EnvProfileMeta> {
    if !scope.is_writable() {
        return Err(EnvError::ScopeNotWritable(scope));
    }
    let vars = registry::list_scope(scope)?
        .into_iter()
        .map(|v| SnapshotEntry {
            name: v.name,
            raw_value: v.raw_value,
            reg_type: v.reg_type,
        })
        .collect();
    let profile = EnvProfile {
        name: validate_name(name)?,
        scope,
        created_at: Local::now().to_rfc3339(),
        vars,
    };
    save_profile(cfg, &profile)
}

pub fn profile_vars_as_env(
    profile: &EnvProfile,
    scope_override: Option<EnvScope>,
) -> EnvResult<Vec<EnvVar>> {
    let target_scope = scope_override.unwrap_or(profile.scope);
    if !target_scope.is_writable() {
        return Err(EnvError::ScopeNotWritable(target_scope));
    }
    Ok(profile
        .vars
        .iter()
        .map(|v| EnvVar {
            scope: target_scope,
            name: v.name.clone(),
            raw_value: v.raw_value.clone(),
            reg_type: v.reg_type,
            inferred_kind: infer_var_kind(&v.name, &v.raw_value),
        })
        .collect())
}

pub fn profile_file_path(cfg: &EnvCoreConfig, name: &str) -> EnvResult<PathBuf> {
    let normalized = validate_name(name)?;
    Ok(profile_dir(cfg)?.join(format!("{normalized}.json")))
}

fn validate_name(name: &str) -> EnvResult<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(EnvError::InvalidInput(
            "profile name cannot be empty".to_string(),
        ));
    }
    let valid = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if !valid {
        return Err(EnvError::InvalidInput(
            "profile name only allows [a-zA-Z0-9._-]".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}
