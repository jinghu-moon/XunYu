use serde::{Deserialize, Serialize};
#[cfg(feature = "redirect")]
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default, Deserialize, Serialize, Clone)]
pub(crate) struct GlobalConfig {
    #[serde(default)]
    pub(crate) tree: TreeConfig,
    #[serde(default)]
    pub(crate) proxy: ProxyConfig,
    #[serde(default)]
    pub(crate) acl: AclConfig,
    #[cfg(feature = "protect")]
    #[serde(default)]
    pub(crate) protect: ProtectConfig,
    #[cfg(feature = "redirect")]
    #[serde(default)]
    pub(crate) redirect: RedirectConfig,
}

#[derive(Default, Deserialize, Serialize, Clone)]
pub(crate) struct TreeConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "defaultDepth")]
    pub(crate) default_depth: Option<usize>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "excludeNames"
    )]
    pub(crate) exclude_names: Vec<String>,
}

#[derive(Default, Deserialize, Serialize, Clone)]
pub(crate) struct ProxyConfig {
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "defaultUrl",
        alias = "default_url"
    )]
    pub(crate) default_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) noproxy: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub(crate) struct AclConfig {
    pub(crate) throttle_limit: usize,
    pub(crate) chunk_size: usize,
    pub(crate) audit_log_path: String,
    pub(crate) export_path: String,
    pub(crate) default_owner: String,
    pub(crate) max_audit_lines: usize,
}

impl Default for AclConfig {
    fn default() -> Self {
        Self {
            throttle_limit: 16,
            chunk_size: 200,
            audit_log_path: default_acl_audit_log_path(),
            export_path: default_acl_export_path(),
            default_owner: "BUILTIN\\Administrators".to_string(),
            max_audit_lines: 5000,
        }
    }
}

#[cfg(feature = "protect")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub(crate) struct ProtectConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) rules: Vec<ProtectRule>,
}

#[cfg(feature = "protect")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub(crate) struct ProtectRule {
    pub(crate) path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) deny: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) require: Vec<String>,
}

#[cfg(feature = "redirect")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub(crate) struct RedirectConfig {
    #[serde(default)]
    pub(crate) profiles: BTreeMap<String, RedirectProfile>,
}

#[cfg(feature = "redirect")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct RedirectProfile {
    #[serde(default)]
    pub(crate) rules: Vec<RedirectRule>,
    #[serde(default)]
    pub(crate) unmatched: RedirectUnmatched,
    #[serde(default, rename = "on_conflict")]
    pub(crate) on_conflict: RedirectOnConflict,
    #[serde(default)]
    pub(crate) recursive: bool,
    #[serde(default = "default_redirect_max_depth")]
    pub(crate) max_depth: u32,
}

#[cfg(feature = "redirect")]
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(try_from = "String", into = "String")]
pub(crate) enum RedirectOnConflict {
    RenameNew,
    RenameDate,
    RenameExisting,
    HashDedup,
    Skip,
    Overwrite,
    Trash,
    Ask,
}

#[cfg(feature = "redirect")]
impl Default for RedirectOnConflict {
    fn default() -> Self {
        Self::RenameNew
    }
}

#[cfg(feature = "redirect")]
impl RedirectOnConflict {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::RenameNew => "rename_new",
            Self::RenameDate => "rename_date",
            Self::RenameExisting => "rename_existing",
            Self::HashDedup => "hash_dedup",
            Self::Skip => "skip",
            Self::Overwrite => "overwrite",
            Self::Trash => "trash",
            Self::Ask => "ask",
        }
    }
}

#[cfg(feature = "redirect")]
impl TryFrom<String> for RedirectOnConflict {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let raw = value.trim();
        if raw.is_empty() {
            return Ok(Self::default());
        }
        let v = raw.to_ascii_lowercase();
        match v.as_str() {
            "rename_new" => Ok(Self::RenameNew),
            "rename_date" => Ok(Self::RenameDate),
            "rename_existing" => Ok(Self::RenameExisting),
            "hash_dedup" => Ok(Self::HashDedup),
            "skip" => Ok(Self::Skip),
            "overwrite" => Ok(Self::Overwrite),
            "trash" => Ok(Self::Trash),
            "ask" => Ok(Self::Ask),
            _ => {
                let opts = [
                    "rename_new",
                    "rename_date",
                    "rename_existing",
                    "hash_dedup",
                    "skip",
                    "overwrite",
                    "trash",
                    "ask",
                ];
                let mut msg = format!("Unsupported on_conflict value: {raw}.");
                if let Some(s) = crate::suggest::did_you_mean(raw, &opts) {
                    msg.push_str(&format!(" Did you mean: \"{s}\"?"));
                }
                msg.push_str(" Valid options: rename_new | rename_date | rename_existing | hash_dedup | skip | overwrite | trash | ask");
                Err(msg)
            }
        }
    }
}

#[cfg(feature = "redirect")]
impl From<RedirectOnConflict> for String {
    fn from(value: RedirectOnConflict) -> Self {
        value.as_str().to_string()
    }
}

#[cfg(feature = "redirect")]
impl std::fmt::Display for RedirectOnConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(feature = "redirect")]
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(try_from = "String", into = "String")]
pub(crate) enum RedirectUnmatched {
    Skip,
    Archive { age_expr: String, dest: String },
}

#[cfg(feature = "redirect")]
impl Default for RedirectUnmatched {
    fn default() -> Self {
        Self::Skip
    }
}

#[cfg(feature = "redirect")]
impl TryFrom<String> for RedirectUnmatched {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let raw = value.trim();
        if raw.is_empty() {
            return Ok(Self::Skip);
        }
        if raw.eq_ignore_ascii_case("skip") {
            return Ok(Self::Skip);
        }
        let rest = raw
            .strip_prefix("archive:")
            .or_else(|| raw.strip_prefix("ARCHIVE:"))
            .ok_or_else(|| format!("Unsupported unmatched action: {raw}"))?;
        let mut parts = rest.splitn(2, ':');
        let age_expr = parts
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("Invalid unmatched archive action: {raw}"))?
            .to_string();
        let dest = parts
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("Invalid unmatched archive action: {raw}"))?
            .to_string();
        Ok(Self::Archive { age_expr, dest })
    }
}

#[cfg(feature = "redirect")]
impl From<RedirectUnmatched> for String {
    fn from(value: RedirectUnmatched) -> Self {
        match value {
            RedirectUnmatched::Skip => "skip".to_string(),
            RedirectUnmatched::Archive { age_expr, dest } => format!("archive:{age_expr}:{dest}"),
        }
    }
}

#[cfg(feature = "redirect")]
impl RedirectUnmatched {
    pub(crate) fn to_raw_string(&self) -> String {
        match self {
            Self::Skip => "skip".to_string(),
            Self::Archive { age_expr, dest } => format!("archive:{age_expr}:{dest}"),
        }
    }
}

#[cfg(feature = "redirect")]
impl std::fmt::Display for RedirectUnmatched {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_raw_string())
    }
}

#[cfg(feature = "redirect")]
fn default_redirect_max_depth() -> u32 {
    1
}

#[cfg(feature = "redirect")]
impl Default for RedirectProfile {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            unmatched: RedirectUnmatched::default(),
            on_conflict: RedirectOnConflict::default(),
            recursive: false,
            max_depth: default_redirect_max_depth(),
        }
    }
}

#[cfg(feature = "redirect")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub(crate) struct RedirectRule {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default, rename = "match")]
    pub(crate) match_cond: MatchCondition,
    #[serde(default)]
    pub(crate) dest: String,
}

#[cfg(feature = "redirect")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub(crate) struct MatchCondition {
    #[serde(default)]
    pub(crate) ext: Vec<String>,
    #[serde(default)]
    pub(crate) glob: Option<String>,
    #[serde(default)]
    pub(crate) regex: Option<String>,
    #[serde(default)]
    pub(crate) size: Option<String>,
    #[serde(default)]
    pub(crate) age: Option<String>,
}

pub(crate) fn load_config() -> GlobalConfig {
    load_config_from_path(&config_path())
}

#[cfg(feature = "redirect")]
pub(crate) fn load_config_strict() -> Result<GlobalConfig, String> {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        Err(_) => Ok(GlobalConfig::default()),
    }
}

fn load_config_from_path(path: &Path) -> GlobalConfig {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => GlobalConfig::default(),
    }
}

pub(crate) fn save_config(cfg: &GlobalConfig) -> Result<(), std::io::Error> {
    save_config_to_path(cfg, &config_path())
}

fn save_config_to_path(cfg: &GlobalConfig, path: &Path) -> Result<(), std::io::Error> {
    let tmp = path.with_extension("tmp");
    let s = serde_json::to_string_pretty(cfg)?;
    fs::write(&tmp, s)?;
    fs::rename(&tmp, path)
}

pub(crate) fn config_path() -> PathBuf {
    let xun_config = env::var("XUN_CONFIG").ok();
    let userprofile = env::var("USERPROFILE").ok();
    config_path_from_env(xun_config.as_deref(), userprofile.as_deref())
}

fn config_path_from_env(xun_config: Option<&str>, userprofile: Option<&str>) -> PathBuf {
    if let Some(p) = xun_config {
        return PathBuf::from(p);
    }
    let home = userprofile.unwrap_or(".");
    PathBuf::from(home).join(".xun.config.json")
}

fn default_acl_audit_log_path() -> String {
    let base = env::var("LOCALAPPDATA")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base)
        .join("xun")
        .join("acl_audit.jsonl")
        .to_string_lossy()
        .into_owned()
}

fn default_acl_export_path() -> String {
    env::var("USERPROFILE")
        .map(|p| {
            PathBuf::from(p)
                .join("Desktop")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|_| ".".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn config_path_prefers_xun_config_env() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("custom.config.json");
        assert_eq!(
            config_path_from_env(Some(p.to_string_lossy().as_ref()), Some("C:\\Users\\x")),
            p
        );
    }

    #[test]
    fn config_path_falls_back_to_userprofile() {
        let dir = tempdir().unwrap();
        assert_eq!(
            config_path_from_env(None, Some(dir.path().to_string_lossy().as_ref())),
            dir.path().join(".xun.config.json")
        );
    }

    #[test]
    fn load_config_missing_returns_default() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("missing.config.json");
        let cfg = load_config_from_path(&p);
        assert_eq!(cfg.tree.default_depth, None);
        assert!(cfg.tree.exclude_names.is_empty());
        assert_eq!(cfg.proxy.default_url, None);
        assert_eq!(cfg.acl.throttle_limit, 16);
    }

    #[test]
    fn load_config_valid_json_is_parsed() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("cfg.json");
        fs::write(
            &p,
            r#"{
  "tree": { "defaultDepth": 3, "excludeNames": ["node_modules"] },
  "proxy": { "defaultUrl": "http://127.0.0.1:7890", "noproxy": "localhost" }
}"#,
        )
        .unwrap();

        let cfg = load_config_from_path(&p);
        assert_eq!(cfg.tree.default_depth, Some(3));
        assert_eq!(cfg.tree.exclude_names, vec!["node_modules"]);
        assert_eq!(
            cfg.proxy.default_url.as_deref(),
            Some("http://127.0.0.1:7890")
        );
        assert_eq!(cfg.proxy.noproxy.as_deref(), Some("localhost"));
    }

    #[test]
    fn tree_config_and_proxy_config_defaults() {
        let t = TreeConfig::default();
        assert_eq!(t.default_depth, None);
        assert!(t.exclude_names.is_empty());

        let p = ProxyConfig::default();
        assert_eq!(p.default_url, None);

        let a = AclConfig::default();
        assert_eq!(a.throttle_limit, 16);
        assert_eq!(a.chunk_size, 200);
    }

    #[cfg(feature = "protect")]
    #[test]
    fn save_config_roundtrip() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("cfg.json");

        let mut cfg = GlobalConfig::default();
        cfg.protect.rules.push(ProtectRule {
            path: "C:\\tmp".to_string(),
            deny: vec!["rm".to_string()],
            require: vec!["reason".to_string()],
        });

        save_config_to_path(&cfg, &p).expect("save");
        let loaded = load_config_from_path(&p);
        assert_eq!(loaded.protect.rules.len(), 1);
        assert_eq!(loaded.protect.rules[0].path, "C:\\tmp");
        assert_eq!(loaded.protect.rules[0].deny, vec!["rm"]);
        assert_eq!(loaded.protect.rules[0].require, vec!["reason"]);
    }

    #[cfg(feature = "redirect")]
    #[test]
    fn redirect_profile_defaults_are_stable() {
        let p = RedirectProfile::default();
        assert_eq!(p.unmatched, RedirectUnmatched::Skip);
        assert_eq!(p.on_conflict, RedirectOnConflict::RenameNew);
        assert!(p.rules.is_empty());
        assert!(!p.recursive);
        assert_eq!(p.max_depth, 1);
    }
}
