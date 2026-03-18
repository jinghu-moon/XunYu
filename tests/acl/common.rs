#![allow(dead_code)]

use csv::ReaderBuilder;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::common::*;

// ── 命令构造 ─────────────────────────────────────────────────────────────────

pub fn acl_cmd(env: &TestEnv) -> Command {
    let mut cmd = env.cmd();
    let local = env.root.join("LocalAppData");
    let desktop = env.root.join("Desktop");
    let _ = fs::create_dir_all(&local);
    let _ = fs::create_dir_all(&desktop);
    cmd.env("LOCALAPPDATA", &local);
    cmd
}

pub fn setup_acl_dir(env: &TestEnv, name: &str) -> PathBuf {
    let dir = env.root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("sample.txt"), b"data").unwrap();
    dir
}

pub fn stderr_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

pub fn str_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

// ── 审计日志读取 ─────────────────────────────────────────────────────────────

pub fn acl_audit_path(env: &TestEnv) -> PathBuf {
    env.root
        .join("LocalAppData")
        .join("xun")
        .join("acl_audit.jsonl")
}

pub fn read_audit_actions(env: &TestEnv) -> Vec<String> {
    let path = acl_audit_path(env);
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
        .collect()
}

pub fn read_audit_paths_for_action(env: &TestEnv, action: &str) -> Vec<String> {
    let path = acl_audit_path(env);
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|v| v.get("action").and_then(|a| a.as_str()) == Some(action))
        .filter_map(|v| v.get("path").and_then(|p| p.as_str()).map(|s| s.to_string()))
        .collect()
}

// ── CSV / 导出工具 ────────────────────────────────────────────────────────────

pub fn count_acl_backups(dir: &Path) -> usize {
    let entries = match fs::read_dir(dir) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension().map(|v| v == "json").unwrap_or(false)
                && e.file_name().to_string_lossy().starts_with("ACL_")
        })
        .count()
}

pub fn find_csv_with_prefix(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(prefix) && name.ends_with(".csv") {
            return Some(entry.path());
        }
    }
    None
}

pub fn read_csv_rows(path: &Path) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .unwrap();
    for record in reader.records().flatten() {
        rows.push(record.iter().map(|v| v.to_string()).collect());
    }
    rows
}

pub fn csv_rows_contain(rows: &[Vec<String>], needle: &str) -> bool {
    rows.iter()
        .any(|row| row.iter().any(|cell| cell.contains(needle)))
}

pub fn has_acl_row(
    rows: &[Vec<String>],
    ace_type: &str,
    source: &str,
    principal: &str,
    rights: &str,
    inherit: &str,
    propagation: &str,
    orphan: &str,
) -> bool {
    rows.iter().any(|row| {
        row.get(0).map(|v| v == ace_type).unwrap_or(false)
            && row.get(1).map(|v| v == source).unwrap_or(false)
            && row.get(2).map(|v| v == principal).unwrap_or(false)
            && row.get(3).map(|v| v == rights).unwrap_or(false)
            && row.get(5).map(|v| v == inherit).unwrap_or(false)
            && row.get(6).map(|v| v == propagation).unwrap_or(false)
            && row.get(7).map(|v| v == orphan).unwrap_or(false)
    })
}

pub fn export_acl_rows(env: &TestEnv, path: &Path, label: &str) -> Vec<Vec<String>> {
    let dest = env.root.join(format!("{label}.csv"));
    run_ok(acl_cmd(env).args([
        "acl", "view", "-p", &str_path(path),
        "--export", &str_path(&dest),
    ]));
    read_csv_rows(&dest)
}

pub fn backup_acl_to(env: &TestEnv, path: &Path, dest: &Path) {
    run_ok(acl_cmd(env).args([
        "acl", "backup", "-p", &str_path(path),
        "-o", &str_path(dest),
    ]));
}

pub fn restore_acl(env: &TestEnv, path: &Path, backup: &Path) {
    run_ok(acl_cmd(env).args([
        "acl", "restore", "-p", &str_path(path),
        "--from", &str_path(backup), "-y",
    ]));
}

pub fn owner_from_summary(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        if let Some(rest) = line.strip_prefix("Owner: ") {
            let owner = rest.split(" | ").next().unwrap_or(rest);
            Some(owner.to_string())
        } else {
            None
        }
    })
}

// ── 备份条目解析 ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct BackupEntry {
    pub raw_sid: String,
    pub ace_type: String,
    pub rights_mask: u32,
    pub inheritance: u64,
    pub propagation: u64,
    pub is_inherited: bool,
}

pub fn normalize_rights_mask(mask: u32) -> u32 {
    mask & !0x0010_0000
}

pub fn backup_entries_from_file(path: &Path) -> Vec<BackupEntry> {
    use serde_json::Value;
    let raw = fs::read_to_string(path).unwrap();
    let v: Value = serde_json::from_str(&raw).unwrap();
    match v
        .get("acl")
        .and_then(|acl| acl.get("entries"))
        .and_then(|entries| entries.as_array())
    {
        Some(entries) => entries
            .iter()
            .filter_map(|entry| {
                Some(BackupEntry {
                    raw_sid: entry.get("raw_sid")?.as_str()?.to_string(),
                    ace_type: entry.get("ace_type")?.as_str()?.to_string(),
                    rights_mask: entry.get("rights_mask")?.as_u64()? as u32,
                    inheritance: entry.get("inheritance")?.as_u64()?,
                    propagation: entry.get("propagation")?.as_u64()?,
                    is_inherited: entry.get("is_inherited")?.as_bool()?,
                })
            })
            .collect(),
        None => Vec::new(),
    }
}

pub fn backup_keys_from_entries(entries: &[BackupEntry]) -> HashSet<String> {
    entries.iter().map(backup_key_for_entry).collect()
}

pub fn backup_key_for_entry(entry: &BackupEntry) -> String {
    let rights_mask = normalize_rights_mask(entry.rights_mask) as u64;
    format!(
        "{}|{}|{}|{}|{}|{}",
        entry.raw_sid, entry.ace_type, rights_mask,
        entry.inheritance, entry.propagation, entry.is_inherited
    )
}

pub fn allow_mask_for_sid(entries: &[BackupEntry], raw_sid: &str) -> u32 {
    entries
        .iter()
        .filter(|e| e.raw_sid == raw_sid && e.ace_type == "Allow")
        .fold(0u32, |acc, e| acc | normalize_rights_mask(e.rights_mask))
}

pub fn rights_mask_for_label(label: &str) -> u32 {
    let mask = match label {
        "Read" => 1_179_785,
        "Write" => 278,
        "Modify" => 1_245_631,
        other => panic!("unexpected rights label: {other}"),
    };
    mask & !0x0010_0000
}

// ── 权限检查 ─────────────────────────────────────────────────────────────────

#[allow(deprecated)]
pub fn is_admin() -> bool {
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}
