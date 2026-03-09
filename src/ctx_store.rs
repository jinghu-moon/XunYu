use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub(crate) const CTX_STATE_ENV: &str = "XUN_CTX_STATE";
pub(crate) const CTX_FILE_ENV: &str = "XUN_CTX_FILE";

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct CtxStore {
    #[serde(default)]
    pub(crate) profiles: BTreeMap<String, CtxProfile>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct CtxProfile {
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) proxy: CtxProxy,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) env: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub(crate) struct CtxProxy {
    #[serde(default)]
    pub(crate) mode: CtxProxyMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) noproxy: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub(crate) enum CtxProxyMode {
    #[default]
    Keep,
    Set,
    Off,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct CtxProxyState {
    pub(crate) url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) noproxy: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct CtxSession {
    pub(crate) active: String,
    pub(crate) previous_dir: String,
    #[serde(default)]
    pub(crate) previous_env: BTreeMap<String, Option<String>>,
    #[serde(default)]
    pub(crate) previous_proxy: Option<CtxProxyState>,
    #[serde(default)]
    pub(crate) proxy_changed: bool,
}

pub(crate) fn ctx_store_path() -> PathBuf {
    if let Ok(path) = env::var(CTX_FILE_ENV)
        && !path.trim().is_empty()
    {
        return PathBuf::from(path);
    }
    let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".xun.ctx.json")
}

pub(crate) fn session_path_from_env() -> Option<PathBuf> {
    env::var(CTX_STATE_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
}

pub(crate) fn load_store(path: &Path) -> CtxStore {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(crate) fn save_store(path: &Path, store: &CtxStore) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let json = serde_json::to_string_pretty(store).unwrap_or_else(|_| "{}".to_string());
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)
}

pub(crate) fn load_session(path: &Path) -> Option<CtxSession> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

pub(crate) fn save_session(path: &Path, session: &CtxSession) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let json = serde_json::to_string_pretty(session).unwrap_or_else(|_| "{}".to_string());
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_store_missing_returns_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.json");
        let store = load_store(&path);
        assert!(store.profiles.is_empty());
    }

    #[test]
    fn save_store_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ctx.json");
        let mut store = CtxStore::default();
        store.profiles.insert(
            "work".to_string(),
            CtxProfile {
                path: "C:\\Repo".to_string(),
                proxy: CtxProxy {
                    mode: CtxProxyMode::Set,
                    url: Some("http://127.0.0.1:7890".to_string()),
                    noproxy: Some("localhost".to_string()),
                },
                tags: vec!["work".to_string()],
                env: BTreeMap::from([("RUST_LOG".to_string(), "info".to_string())]),
            },
        );
        save_store(&path, &store).unwrap();
        let loaded = load_store(&path);
        assert!(loaded.profiles.contains_key("work"));
        let p = loaded.profiles.get("work").unwrap();
        assert_eq!(p.path, "C:\\Repo");
        assert_eq!(p.tags, vec!["work"]);
        assert_eq!(p.proxy.mode, CtxProxyMode::Set);
        assert_eq!(p.proxy.url.as_deref(), Some("http://127.0.0.1:7890"));
        assert_eq!(p.env.get("RUST_LOG").map(String::as_str), Some("info"));
    }

    #[test]
    fn save_session_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("session.json");
        let session = CtxSession {
            active: "work".to_string(),
            previous_dir: "C:\\Prev".to_string(),
            previous_env: BTreeMap::from([("XUN_CTX".to_string(), None)]),
            previous_proxy: Some(CtxProxyState {
                url: "http://127.0.0.1:7890".to_string(),
                noproxy: None,
            }),
            proxy_changed: true,
        };
        save_session(&path, &session).unwrap();
        let loaded = load_session(&path).unwrap();
        assert_eq!(loaded.active, "work");
        assert_eq!(loaded.previous_dir, "C:\\Prev");
        assert!(loaded.previous_proxy.is_some());
        assert!(loaded.proxy_changed);
    }
}
