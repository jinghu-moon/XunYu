use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::types::{EnvError, EnvResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvCoreConfig {
    #[serde(default)]
    pub snapshot_dir: Option<PathBuf>,
    #[serde(default)]
    pub profile_dir: Option<PathBuf>,
    #[serde(default = "default_max_snapshots")]
    pub max_snapshots: usize,
    #[serde(default = "default_lock_timeout_ms")]
    pub lock_timeout_ms: u64,
    #[serde(default = "default_stale_lock_secs")]
    pub stale_lock_secs: u64,
    #[serde(default = "default_notify_enabled")]
    pub notify_enabled: bool,
    #[serde(default = "default_allow_run")]
    pub allow_run: bool,
    #[serde(default = "default_snapshot_every_secs")]
    pub snapshot_every_secs: u64,
}

const fn default_max_snapshots() -> usize {
    50
}

const fn default_lock_timeout_ms() -> u64 {
    5_000
}

const fn default_stale_lock_secs() -> u64 {
    300
}

const fn default_notify_enabled() -> bool {
    true
}

const fn default_allow_run() -> bool {
    false
}

const fn default_snapshot_every_secs() -> u64 {
    0
}

impl Default for EnvCoreConfig {
    fn default() -> Self {
        Self {
            snapshot_dir: None,
            profile_dir: None,
            max_snapshots: default_max_snapshots(),
            lock_timeout_ms: default_lock_timeout_ms(),
            stale_lock_secs: default_stale_lock_secs(),
            notify_enabled: default_notify_enabled(),
            allow_run: default_allow_run(),
            snapshot_every_secs: default_snapshot_every_secs(),
        }
    }
}

impl EnvCoreConfig {
    pub fn snapshot_dir(&self) -> PathBuf {
        if let Some(p) = &self.snapshot_dir {
            return p.clone();
        }
        default_snapshot_dir()
    }

    pub fn profile_dir(&self) -> PathBuf {
        if let Some(p) = &self.profile_dir {
            return p.clone();
        }
        default_profile_dir()
    }

    pub fn lock_file_path(&self) -> PathBuf {
        self.snapshot_dir().join(".xun.env.lock")
    }
}

pub fn config_file_path() -> PathBuf {
    config_dir_path().join("config.toml")
}

pub fn load_env_config() -> EnvCoreConfig {
    let path = config_file_path();
    match fs::read_to_string(&path) {
        Ok(content) => toml::from_str::<EnvCoreConfig>(&content)
            .unwrap_or_else(|_| load_legacy_json().unwrap_or_default()),
        Err(_) => load_legacy_json().unwrap_or_default(),
    }
}

pub fn save_env_config(cfg: &EnvCoreConfig) -> EnvResult<()> {
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = toml::to_string_pretty(cfg)
        .map_err(|e| EnvError::Other(format!("toml serialize error: {}", e)))?;
    fs::write(path, data)?;
    Ok(())
}

pub fn reset_env_config() -> EnvResult<EnvCoreConfig> {
    let cfg = EnvCoreConfig::default();
    save_env_config(&cfg)?;
    Ok(cfg)
}

pub fn get_config_value(cfg: &EnvCoreConfig, key: &str) -> Option<String> {
    match key {
        "snapshot_dir" => Some(
            cfg.snapshot_dir
                .as_ref()
                .map(|v| v.display().to_string())
                .unwrap_or_default(),
        ),
        "profile_dir" => Some(
            cfg.profile_dir
                .as_ref()
                .map(|v| v.display().to_string())
                .unwrap_or_default(),
        ),
        "max_snapshots" => Some(cfg.max_snapshots.to_string()),
        "general.max_snapshots" => Some(cfg.max_snapshots.to_string()),
        "lock_timeout_ms" => Some(cfg.lock_timeout_ms.to_string()),
        "stale_lock_secs" => Some(cfg.stale_lock_secs.to_string()),
        "notify_enabled" => Some(cfg.notify_enabled.to_string()),
        "allow_run" => Some(cfg.allow_run.to_string()),
        "snapshot_every_secs" => Some(cfg.snapshot_every_secs.to_string()),
        "general.snapshot_every_secs" => Some(cfg.snapshot_every_secs.to_string()),
        _ => None,
    }
}

pub fn set_config_value(cfg: &mut EnvCoreConfig, key: &str, value: &str) -> EnvResult<()> {
    match key {
        "snapshot_dir" => {
            cfg.snapshot_dir = if value.trim().is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            };
            Ok(())
        }
        "profile_dir" => {
            cfg.profile_dir = if value.trim().is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            };
            Ok(())
        }
        "max_snapshots" | "general.max_snapshots" => {
            cfg.max_snapshots = value.parse::<usize>().map_err(|e| {
                EnvError::InvalidInput(format!("max_snapshots must be usize: {}", e))
            })?;
            Ok(())
        }
        "lock_timeout_ms" => {
            cfg.lock_timeout_ms = value.parse::<u64>().map_err(|e| {
                EnvError::InvalidInput(format!("lock_timeout_ms must be u64: {}", e))
            })?;
            Ok(())
        }
        "stale_lock_secs" => {
            cfg.stale_lock_secs = value.parse::<u64>().map_err(|e| {
                EnvError::InvalidInput(format!("stale_lock_secs must be u64: {}", e))
            })?;
            Ok(())
        }
        "notify_enabled" => {
            cfg.notify_enabled = value.parse::<bool>().map_err(|e| {
                EnvError::InvalidInput(format!("notify_enabled must be bool: {}", e))
            })?;
            Ok(())
        }
        "allow_run" => {
            cfg.allow_run = value
                .parse::<bool>()
                .map_err(|e| EnvError::InvalidInput(format!("allow_run must be bool: {}", e)))?;
            Ok(())
        }
        "snapshot_every_secs" | "general.snapshot_every_secs" => {
            cfg.snapshot_every_secs = value.parse::<u64>().map_err(|e| {
                EnvError::InvalidInput(format!("snapshot_every_secs must be u64: {}", e))
            })?;
            Ok(())
        }
        _ => Err(EnvError::InvalidInput(format!(
            "unknown env config key '{}'",
            key
        ))),
    }
}

pub fn ensure_dir(path: &Path) -> EnvResult<()> {
    fs::create_dir_all(path).map_err(EnvError::from)
}

pub fn default_snapshot_dir() -> PathBuf {
    user_home_dir().join(".xun.env.snapshots")
}

pub fn default_profile_dir() -> PathBuf {
    user_home_dir().join(".xun.env.profiles")
}

fn config_dir_path() -> PathBuf {
    if let Ok(raw) = std::env::var("XUN_ENV_CONFIG_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Ok(raw) = std::env::var("ENVMGR_CONFIG_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("xun").join("env");
        }
    }
    user_home_dir().join(".config").join("xun").join("env")
}

fn legacy_config_file_path() -> PathBuf {
    user_home_dir().join(".xun.env.config.json")
}

fn load_legacy_json() -> Option<EnvCoreConfig> {
    let path = legacy_config_file_path();
    let content = fs::read_to_string(path).ok()?;
    let cfg = serde_json::from_str::<EnvCoreConfig>(&content).ok()?;
    let _ = save_env_config(&cfg);
    Some(cfg)
}

fn user_home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
