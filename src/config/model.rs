use serde::{Deserialize, Serialize};
#[cfg(feature = "redirect")]
use std::collections::BTreeMap;

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
    #[serde(default = "super::defaults::default_redirect_max_depth")]
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
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(try_from = "String", into = "String")]
pub(crate) enum RedirectUnmatched {
    Skip,
    Archive { age_expr: String, dest: String },
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
