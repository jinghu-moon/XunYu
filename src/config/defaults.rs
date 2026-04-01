use std::env;
use std::path::PathBuf;

use super::model::{AclConfig, BookmarkAutoLearnConfig, BookmarkConfig, BookmarkFzfConfig};
#[cfg(feature = "redirect")]
use super::model::{RedirectOnConflict, RedirectProfile, RedirectUnmatched};

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

impl Default for BookmarkConfig {
    fn default() -> Self {
        Self {
            version: 1,
            data_file: default_bookmark_data_file(),
            visit_log_file: default_bookmark_visit_log_file(),
            default_scope: "auto".to_string(),
            default_list_limit: 20,
            max_age: 10_000,
            resolve_symlinks: false,
            echo: false,
            exclude_dirs: vec![
                "node_modules".to_string(),
                "dist".to_string(),
                "build".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "tmp".to_string(),
                "temp".to_string(),
            ],
            auto_learn: BookmarkAutoLearnConfig::default(),
            fzf: BookmarkFzfConfig::default(),
        }
    }
}

impl Default for BookmarkAutoLearnConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            import_history_on_first_init: true,
        }
    }
}

impl Default for BookmarkFzfConfig {
    fn default() -> Self {
        Self {
            min_version: "0.51.0".to_string(),
            opts: String::new(),
        }
    }
}

#[cfg(feature = "redirect")]
pub(super) fn default_redirect_max_depth() -> u32 {
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

fn default_bookmark_data_file() -> String {
    let base = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base)
        .join(".xun.bookmark.json")
        .to_string_lossy()
        .into_owned()
}

fn default_bookmark_visit_log_file() -> String {
    let base = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base)
        .join(".xun.bookmark.visits.jsonl")
        .to_string_lossy()
        .into_owned()
}
