use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockInfo {
    pub pid: u32,
    pub hostname: String,
    pub username: String,
    pub command: String,
    pub started_at: u64,
    pub heartbeat_at: u64,
    pub tool_version: String,
    pub write_start_offset: u64,
}

#[derive(Debug)]
pub struct LockFile {
    path: PathBuf,
    info: LockInfo,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LockError {
    #[error("container locked: {0}")]
    ContainerLocked(String),
    #[error("lock I/O error: {0}")]
    Io(String),
    #[error("lock parse error: {0}")]
    Parse(String),
}

impl LockFile {
    pub fn acquire_write_lock(
        container_path: &Path,
        command: impl Into<String>,
        write_start_offset: u64,
    ) -> Result<Self, LockError> {
        let path = lock_path_for(container_path);
        if path.exists() {
            return Err(LockError::ContainerLocked(path.display().to_string()));
        }

        let info = LockInfo {
            pid: std::process::id(),
            hostname: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown-host".to_string()),
            username: std::env::var("USERNAME").unwrap_or_else(|_| "unknown-user".to_string()),
            command: command.into(),
            started_at: now_unix_secs(),
            heartbeat_at: now_unix_secs(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            write_start_offset,
        };
        let raw =
            serde_json::to_string_pretty(&info).map_err(|err| LockError::Parse(err.to_string()))?;
        fs::write(&path, raw).map_err(|err| LockError::Io(err.to_string()))?;
        Ok(Self { path, info })
    }

    pub fn release(self) -> Result<(), LockError> {
        fs::remove_file(self.path).map_err(|err| LockError::Io(err.to_string()))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn info(&self) -> &LockInfo {
        &self.info
    }
}

pub fn read_lock_info(path: &Path) -> Result<LockInfo, LockError> {
    let raw = fs::read_to_string(path).map_err(|err| LockError::Io(err.to_string()))?;
    serde_json::from_str(&raw).map_err(|err| LockError::Parse(err.to_string()))
}

pub fn lock_path_for(container_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.lock", container_path.display()))
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
