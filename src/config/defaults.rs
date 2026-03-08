use std::env;
use std::path::PathBuf;

use super::model::AclConfig;
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

#[cfg(feature = "redirect")]
pub(super) fn default_redirect_max_depth() -> u32 {
    1
}

#[cfg(feature = "redirect")]
impl Default for RedirectOnConflict {
    fn default() -> Self {
        Self::RenameNew
    }
}

#[cfg(feature = "redirect")]
impl Default for RedirectUnmatched {
    fn default() -> Self {
        Self::Skip
    }
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
