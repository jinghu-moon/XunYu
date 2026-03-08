use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use super::config::{EnvCoreConfig, config_file_path};
use super::types::{EnvAuditEntry, EnvResult};

pub fn audit_file_path(_cfg: &EnvCoreConfig) -> PathBuf {
    config_file_path().with_file_name(".xun.env.audit.jsonl")
}

pub fn append_audit(cfg: &EnvCoreConfig, entry: &EnvAuditEntry) -> EnvResult<()> {
    let path = audit_file_path(cfg);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(entry)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

pub fn list_audit(cfg: &EnvCoreConfig, limit: usize) -> EnvResult<Vec<EnvAuditEntry>> {
    let path = audit_file_path(cfg);
    let file = match OpenOptions::new().read(true).open(path) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let reader = BufReader::new(file);
    let mut entries = Vec::<EnvAuditEntry>::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<EnvAuditEntry>(&line) {
            entries.push(entry);
        }
    }
    if limit == 0 || entries.len() <= limit {
        return Ok(entries);
    }
    Ok(entries.split_off(entries.len() - limit))
}
