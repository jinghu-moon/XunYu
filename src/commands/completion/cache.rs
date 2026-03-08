use crate::ctx_store::{ctx_store_path, load_store};
use crate::model::Entry;
use crate::store::{db_path, load};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

#[derive(Default)]
struct CompletionCache {
    db_path: Option<PathBuf>,
    db_mtime: Option<SystemTime>,
    visits_mtime: Option<SystemTime>,
    db: Arc<std::collections::BTreeMap<String, Entry>>,
    config_path: Option<PathBuf>,
    config_mtime: Option<SystemTime>,
    config_keys: Arc<Vec<String>>,
    profile_names: Arc<Vec<String>>,
    ctx_path: Option<PathBuf>,
    ctx_mtime: Option<SystemTime>,
    ctx_profiles: Arc<Vec<String>>,
    #[cfg(feature = "redirect")]
    audit_path: Option<PathBuf>,
    #[cfg(feature = "redirect")]
    audit_mtime: Option<SystemTime>,
    #[cfg(feature = "redirect")]
    audit_txs: Arc<Vec<String>>,
}

static CACHE: OnceLock<Mutex<CompletionCache>> = OnceLock::new();

fn cache() -> &'static Mutex<CompletionCache> {
    CACHE.get_or_init(|| Mutex::new(CompletionCache::default()))
}

pub(super) fn cached_db() -> Arc<std::collections::BTreeMap<String, Entry>> {
    let path = db_path();
    let visits = visit_log_path(&path);
    let db_mtime = file_mtime(&path);
    let visits_mtime = file_mtime(&visits);

    let mut cache = cache().lock().expect("completion cache");
    let path_changed = cache.db_path.as_ref().map(|p| p != &path).unwrap_or(true);
    let mtime_changed = cache.db_mtime != db_mtime || cache.visits_mtime != visits_mtime;
    if path_changed || mtime_changed {
        let db = load(&path);
        cache.db = Arc::new(db);
        cache.db_path = Some(path);
        cache.db_mtime = db_mtime;
        cache.visits_mtime = visits_mtime;
    }
    cache.db.clone()
}

pub(super) fn cached_config_keys_and_profiles() -> (Arc<Vec<String>>, Arc<Vec<String>>) {
    let path = crate::config::config_path();
    let mtime = file_mtime(&path);

    let mut cache = cache().lock().expect("completion cache");
    let path_changed = cache
        .config_path
        .as_ref()
        .map(|p| p != &path)
        .unwrap_or(true);
    let mtime_changed = cache.config_mtime != mtime;
    if path_changed || mtime_changed {
        let content = fs::read_to_string(&path).unwrap_or_default();
        let value: serde_json::Value =
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}));
        let mut keys = Vec::new();
        collect_keys("", &value, &mut keys);
        keys.sort();

        let mut profiles = Vec::new();
        if let Some(obj) = value
            .get("redirect")
            .and_then(|v| v.get("profiles"))
            .and_then(|v| v.as_object())
        {
            for k in obj.keys() {
                profiles.push(k.to_string());
            }
            profiles.sort();
        }

        cache.config_keys = Arc::new(keys);
        cache.profile_names = Arc::new(profiles);
        cache.config_path = Some(path);
        cache.config_mtime = mtime;
    }
    (cache.config_keys.clone(), cache.profile_names.clone())
}

pub(super) fn cached_ctx_profiles() -> Arc<Vec<String>> {
    let path = ctx_store_path();
    let mtime = file_mtime(&path);

    let mut cache = cache().lock().expect("completion cache");
    let path_changed = cache.ctx_path.as_ref().map(|p| p != &path).unwrap_or(true);
    let mtime_changed = cache.ctx_mtime != mtime;
    if path_changed || mtime_changed {
        let store = load_store(&path);
        let mut profiles: Vec<String> = store.profiles.keys().cloned().collect();
        profiles.sort();
        cache.ctx_profiles = Arc::new(profiles);
        cache.ctx_path = Some(path);
        cache.ctx_mtime = mtime;
    }
    cache.ctx_profiles.clone()
}

#[cfg(feature = "redirect")]
pub(super) fn cached_audit_txs() -> Arc<Vec<String>> {
    let path = audit_path();
    let mtime = file_mtime(&path);
    let mut cache = cache().lock().expect("completion cache");
    let path_changed = cache
        .audit_path
        .as_ref()
        .map(|p| p != &path)
        .unwrap_or(true);
    let mtime_changed = cache.audit_mtime != mtime;
    if path_changed || mtime_changed {
        let content = fs::read_to_string(&path).unwrap_or_default();
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for line in content.lines().rev().take(200) {
            if let Some(tx) = extract_tx_from_line(line) {
                if seen.insert(tx.clone()) {
                    out.push(tx);
                }
            }
        }
        cache.audit_txs = Arc::new(out);
        cache.audit_path = Some(path);
        cache.audit_mtime = mtime;
    }
    cache.audit_txs.clone()
}

fn collect_keys(prefix: &str, value: &serde_json::Value, out: &mut Vec<String>) {
    if let serde_json::Value::Object(map) = value {
        for (k, v) in map {
            let key = if prefix.is_empty() {
                k.to_string()
            } else {
                format!("{prefix}.{k}")
            };
            out.push(key.clone());
            collect_keys(&key, v, out);
        }
    }
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn visit_log_path(db_path: &Path) -> PathBuf {
    db_path.with_extension("visits.jsonl")
}

#[cfg(feature = "redirect")]
fn audit_path() -> PathBuf {
    let db = db_path();
    let dir = db.parent().unwrap_or_else(|| Path::new("."));
    dir.join("audit.jsonl")
}

#[cfg(feature = "redirect")]
fn extract_tx_from_line(line: &str) -> Option<String> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some(params) = v.get("params_json").or_else(|| v.get("params")) {
            if let Some(tx) = extract_tx_from_params(params) {
                return Some(tx);
            }
        }
    }
    if let Some(idx) = line.find("\"tx\"") {
        let rest = &line[idx..];
        if let Some(start) = rest
            .find('"')
            .and_then(|i| rest[i + 1..].find('"').map(|j| i + 1 + j))
        {
            let rest = &rest[start + 1..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    if let Some(idx) = line.find("tx=") {
        let rest = &line[idx + 3..];
        let end = rest.find(' ').unwrap_or(rest.len());
        let tx = rest[..end].trim();
        if !tx.is_empty() {
            return Some(tx.to_string());
        }
    }
    None
}

#[cfg(feature = "redirect")]
fn extract_tx_from_params(params: &serde_json::Value) -> Option<String> {
    match params {
        serde_json::Value::Object(map) => map
            .get("tx")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        serde_json::Value::String(s) => extract_tx_from_text(s),
        _ => None,
    }
}

#[cfg(feature = "redirect")]
fn extract_tx_from_text(text: &str) -> Option<String> {
    let idx = text.find("tx=")?;
    let rest = &text[idx + 3..];
    let end = rest.find(' ').unwrap_or(rest.len());
    let tx = rest[..end].trim();
    if tx.is_empty() {
        None
    } else {
        Some(tx.to_string())
    }
}
