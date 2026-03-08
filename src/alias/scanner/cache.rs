use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::AppEntry;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct SourceCache {
    generated_unix: u64,
    #[serde(default)]
    fingerprint: String,
    #[serde(default)]
    entries: Vec<AppEntry>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct CacheDoc {
    // legacy 字段：兼容旧缓存结构
    #[serde(default)]
    generated_unix: u64,
    #[serde(default)]
    entries: BTreeMap<String, Vec<AppEntry>>,
    #[serde(default)]
    sources: BTreeMap<String, SourceCache>,
}

pub(crate) fn load_source(
    source: &str,
    ttl_secs: u64,
    fingerprint: Option<&str>,
) -> Option<Vec<AppEntry>> {
    let path = cache_path();
    let text = fs::read_to_string(path).ok()?;
    let doc: CacheDoc = serde_json::from_str(&text).ok()?;

    if let Some(item) = doc.sources.get(source) {
        if cache_expired(item.generated_unix, ttl_secs) {
            return None;
        }
        if !fingerprint_match(&item.fingerprint, fingerprint) {
            return None;
        }
        return Some(item.entries.clone());
    }

    // 旧版本缓存回退：如果请求了 fingerprint，旧格式无法验证，强制失效。
    if fingerprint.unwrap_or_default().is_empty() {
        if cache_expired(doc.generated_unix, ttl_secs) {
            return None;
        }
        return doc.entries.get(source).cloned();
    }
    None
}

pub(crate) fn store_source(source: &str, entries: &[AppEntry], fingerprint: Option<&str>) {
    let path = cache_path();
    let mut doc = fs::read_to_string(&path)
        .ok()
        .and_then(|v| serde_json::from_str::<CacheDoc>(&v).ok())
        .unwrap_or_default();
    let now = now_unix();
    doc.generated_unix = now;
    doc.entries.remove(source);
    doc.sources.insert(
        source.to_string(),
        SourceCache {
            generated_unix: now,
            fingerprint: fingerprint.unwrap_or_default().to_string(),
            entries: entries.to_vec(),
        },
    );

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &path,
        serde_json::to_vec_pretty(&doc).unwrap_or_else(|_| b"{}".to_vec()),
    );
}

fn cache_expired(generated_unix: u64, ttl_secs: u64) -> bool {
    now_unix().saturating_sub(generated_unix) > ttl_secs
}

fn fingerprint_match(cached: &str, expected: Option<&str>) -> bool {
    match expected {
        Some(v) => cached == v,
        None => true,
    }
}

fn cache_path() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("xun")
        .join("scan-cache.json")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0)
}
