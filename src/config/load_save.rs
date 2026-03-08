use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::model::GlobalConfig;

pub(super) fn load_config() -> GlobalConfig {
    load_config_from_path(&config_path())
}

#[cfg(feature = "redirect")]
pub(super) fn load_config_strict() -> Result<GlobalConfig, String> {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        Err(_) => Ok(GlobalConfig::default()),
    }
}

pub(super) fn load_config_from_path(path: &Path) -> GlobalConfig {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => GlobalConfig::default(),
    }
}

pub(super) fn save_config(cfg: &GlobalConfig) -> Result<(), std::io::Error> {
    save_config_to_path(cfg, &config_path())
}

pub(super) fn save_config_to_path(cfg: &GlobalConfig, path: &Path) -> Result<(), std::io::Error> {
    let tmp = path.with_extension("tmp");
    let s = serde_json::to_string_pretty(cfg)?;
    fs::write(&tmp, s)?;
    fs::rename(&tmp, path)
}

pub(super) fn config_path() -> PathBuf {
    let xun_config = env::var("XUN_CONFIG").ok();
    let userprofile = env::var("USERPROFILE").ok();
    config_path_from_env(xun_config.as_deref(), userprofile.as_deref())
}

pub(super) fn config_path_from_env(xun_config: Option<&str>, userprofile: Option<&str>) -> PathBuf {
    if let Some(p) = xun_config {
        return PathBuf::from(p);
    }
    let home = userprofile.unwrap_or(".");
    PathBuf::from(home).join(".xun.config.json")
}
