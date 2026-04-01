use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use super::model::GlobalConfig;

static CONFIG_CACHE: OnceLock<Mutex<Option<CachedConfig>>> = OnceLock::new();

#[derive(Clone)]
struct CachedConfig {
    path: PathBuf,
    mtime: Option<SystemTime>,
    cfg: GlobalConfig,
}

pub(super) fn load_config() -> GlobalConfig {
    let path = config_path();
    let mtime = file_mtime(&path);
    let cache = CONFIG_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock()
        && let Some(cached) = guard.as_ref()
        && cached.path == path
        && cached.mtime == mtime
    {
        return cached.cfg.clone();
    }

    let cfg = load_config_from_path(&path);
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(CachedConfig {
            path,
            mtime,
            cfg: cfg.clone(),
        });
    }
    cfg
}

#[cfg(feature = "redirect")]
pub(super) fn load_config_strict() -> Result<GlobalConfig, String> {
    let path = config_path();
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
        Err(_) => Ok(GlobalConfig::default()),
    }
}

pub(super) fn load_config_from_path(path: &Path) -> GlobalConfig {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
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
    fs::rename(&tmp, path)?;
    if let Ok(mut guard) = CONFIG_CACHE.get_or_init(|| Mutex::new(None)).lock() {
        *guard = Some(CachedConfig {
            path: path.to_path_buf(),
            mtime: file_mtime(path),
            cfg: cfg.clone(),
        });
    }
    Ok(())
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

fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|meta| meta.modified()).ok()
}
