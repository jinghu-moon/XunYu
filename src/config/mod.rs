mod compat;
mod defaults;
mod load_save;
mod model;

use std::path::PathBuf;

#[allow(unused_imports)]
pub(crate) use model::{AclConfig, GlobalConfig, ProxyConfig, TreeConfig};
#[cfg(feature = "redirect")]
#[allow(unused_imports)]
pub(crate) use model::{
    MatchCondition, RedirectConfig, RedirectOnConflict, RedirectProfile, RedirectRule,
    RedirectUnmatched,
};
#[cfg(feature = "protect")]
pub(crate) use model::{ProtectConfig, ProtectRule};

pub(crate) fn load_config() -> GlobalConfig {
    load_save::load_config()
}

#[cfg(feature = "redirect")]
pub(crate) fn load_config_strict() -> Result<GlobalConfig, String> {
    load_save::load_config_strict()
}

pub(crate) fn save_config(cfg: &GlobalConfig) -> Result<(), std::io::Error> {
    load_save::save_config(cfg)
}

pub(crate) fn config_path() -> PathBuf {
    load_save::config_path()
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
            load_save::config_path_from_env(
                Some(p.to_string_lossy().as_ref()),
                Some("C:\\Users\\x")
            ),
            p
        );
    }

    #[test]
    fn config_path_falls_back_to_userprofile() {
        let dir = tempdir().unwrap();
        assert_eq!(
            load_save::config_path_from_env(None, Some(dir.path().to_string_lossy().as_ref())),
            dir.path().join(".xun.config.json")
        );
    }

    #[test]
    fn load_config_missing_returns_default() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("missing.config.json");
        let cfg = load_save::load_config_from_path(&p);
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

        let cfg = load_save::load_config_from_path(&p);
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

        load_save::save_config_to_path(&cfg, &p).expect("save");
        let loaded = load_save::load_config_from_path(&p);
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
