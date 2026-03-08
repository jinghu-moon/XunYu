use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::config::{EnvCoreConfig, ensure_dir};
use super::types::{EnvError, EnvResult};

struct LockRecord {
    pid: u32,
    ts: u64,
    holder: String,
}

impl LockRecord {
    fn current(holder: &str) -> Self {
        Self {
            pid: std::process::id(),
            ts: now_secs(),
            holder: holder.to_string(),
        }
    }

    fn encode(&self) -> String {
        format!("pid={} ts={} holder={}\n", self.pid, self.ts, self.holder)
    }

    fn decode(raw: &str) -> Option<Self> {
        let mut pid = None;
        let mut ts = None;
        let mut holder = String::from("unknown");
        for token in raw.split_whitespace() {
            if let Some(v) = token.strip_prefix("pid=") {
                pid = v.parse::<u32>().ok();
            } else if let Some(v) = token.strip_prefix("ts=") {
                ts = v.parse::<u64>().ok();
            } else if let Some(v) = token.strip_prefix("holder=") {
                holder = v.to_string();
            }
        }
        Some(Self {
            pid: pid?,
            ts: ts?,
            holder,
        })
    }
}

pub struct EnvLockGuard {
    lock_path: std::path::PathBuf,
    #[allow(dead_code)]
    file: std::fs::File,
}

impl Drop for EnvLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

pub fn acquire_lock(cfg: &EnvCoreConfig, holder: &str) -> EnvResult<EnvLockGuard> {
    let lock_path = cfg.lock_file_path();
    if let Some(parent) = lock_path.parent() {
        ensure_dir(parent)?;
    }

    let deadline = Instant::now() + Duration::from_millis(cfg.lock_timeout_ms.max(1));
    let record = LockRecord::current(holder);
    loop {
        match try_create_lock_file(&lock_path, &record) {
            Ok(file) => {
                return Ok(EnvLockGuard { lock_path, file });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                cleanup_stale_lock(&lock_path, cfg.stale_lock_secs);
                if Instant::now() >= deadline {
                    return Err(EnvError::LockFailed(format!(
                        "timeout waiting lock '{}'",
                        lock_path.display()
                    )));
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(EnvError::Io(e)),
        }
    }
}

pub fn try_with_lock<T, F>(cfg: &EnvCoreConfig, holder: &str, f: F) -> EnvResult<T>
where
    F: FnOnce() -> EnvResult<T>,
{
    let _guard = acquire_lock(cfg, holder)?;
    f()
}

fn try_create_lock_file(path: &Path, record: &LockRecord) -> std::io::Result<std::fs::File> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(record.encode().as_bytes())?;
    file.flush()?;
    Ok(file)
}

fn cleanup_stale_lock(path: &Path, stale_secs: u64) {
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return,
    };
    let Some(record) = LockRecord::decode(&raw) else {
        return;
    };
    if now_secs().saturating_sub(record.ts) < stale_secs {
        return;
    }
    let _ = fs::remove_file(path);
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_roundtrip() {
        let rec = LockRecord::current("test");
        let parsed = LockRecord::decode(&rec.encode()).expect("parse");
        assert_eq!(parsed.pid, rec.pid);
        assert_eq!(parsed.ts, rec.ts);
        assert_eq!(parsed.holder, rec.holder);
    }
}
