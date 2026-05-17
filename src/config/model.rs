use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(feature = "redirect")]
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize, Clone)]
pub struct GlobalConfig {
    #[serde(default)]
    pub bookmark: BookmarkConfig,
    #[serde(default)]
    pub tree: TreeConfig,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub acl: AclConfig,
    #[cfg(feature = "desktop")]
    #[serde(default)]
    pub desktop: DesktopConfig,
    #[cfg(feature = "protect")]
    #[serde(default)]
    pub protect: ProtectConfig,
    #[cfg(feature = "redirect")]
    #[serde(default)]
    pub redirect: RedirectConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct BookmarkConfig {
    pub version: u32,
    #[serde(rename = "dataFile")]
    pub data_file: String,
    #[serde(rename = "visitLogFile")]
    pub visit_log_file: String,
    #[serde(rename = "defaultScope")]
    pub default_scope: String,
    #[serde(rename = "defaultListLimit")]
    pub default_list_limit: usize,
    #[serde(rename = "maxAge")]
    pub max_age: u64,
    #[serde(rename = "resolveSymlinks")]
    pub resolve_symlinks: bool,
    pub echo: bool,
    #[serde(rename = "excludeDirs")]
    pub exclude_dirs: Vec<String>,
    #[serde(rename = "autoLearn")]
    pub auto_learn: BookmarkAutoLearnConfig,
    pub fzf: BookmarkFzfConfig,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub presets: HashMap<String, PresetConfig>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PresetConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct BookmarkAutoLearnConfig {
    pub enabled: bool,
    #[serde(rename = "importHistoryOnFirstInit")]
    pub import_history_on_first_init: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct BookmarkFzfConfig {
    #[serde(rename = "minVersion")]
    pub min_version: String,
    pub opts: String,
}

#[derive(Default, Deserialize, Serialize, Clone)]
pub struct TreeConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "defaultDepth")]
    pub default_depth: Option<usize>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "excludeNames"
    )]
    pub exclude_names: Vec<String>,
}

#[derive(Default, Deserialize, Serialize, Clone)]
pub struct ProxyConfig {
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "defaultUrl",
        alias = "default_url"
    )]
    pub default_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noproxy: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct AclConfig {
    pub throttle_limit: usize,
    pub chunk_size: usize,
    pub audit_log_path: String,
    pub export_path: String,
    pub default_owner: String,
    pub max_audit_lines: usize,
}

#[cfg(feature = "protect")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct ProtectConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<ProtectRule>,
}

#[cfg(feature = "protect")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct ProtectRule {
    pub path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub require: Vec<String>,
}

#[cfg(feature = "redirect")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct RedirectConfig {
    #[serde(default)]
    pub profiles: BTreeMap<String, RedirectProfile>,
}

#[cfg(feature = "redirect")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RedirectProfile {
    #[serde(default)]
    pub rules: Vec<RedirectRule>,
    #[serde(default)]
    pub unmatched: RedirectUnmatched,
    #[serde(default, rename = "on_conflict")]
    pub on_conflict: RedirectOnConflict,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default = "super::defaults::default_redirect_max_depth")]
    pub max_depth: u32,
}

#[cfg(feature = "redirect")]
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(try_from = "String", into = "String")]
#[derive(Default)]
pub enum RedirectOnConflict {
    #[default]
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
    pub fn as_str(&self) -> &'static str {
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
#[derive(Default)]
pub enum RedirectUnmatched {
    #[default]
    Skip,
    Archive {
        age_expr: String,
        dest: String,
    },
}

#[cfg(feature = "redirect")]
impl RedirectUnmatched {
    pub fn to_raw_string(&self) -> String {
        match self {
            Self::Skip => "skip".to_string(),
            Self::Archive { age_expr, dest } => format!("archive:{age_expr}:{dest}"),
        }
    }
}

#[cfg(feature = "redirect")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct RedirectRule {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "match")]
    pub match_cond: MatchCondition,
    #[serde(default)]
    pub dest: String,
}

#[cfg(feature = "redirect")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct MatchCondition {
    #[serde(default)]
    pub ext: Vec<String>,
    #[serde(default)]
    pub glob: Option<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub age: Option<String>,
}

#[cfg(feature = "desktop")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct DesktopConfig {
    #[serde(default)]
    pub daemon: DesktopDaemonConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<DesktopBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remaps: Vec<DesktopRemap>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub snippets: Vec<DesktopSnippet>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layouts: Vec<DesktopLayout>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workspaces: Vec<DesktopWorkspace>,
    #[serde(default)]
    pub theme: DesktopThemeConfig,
    #[serde(default)]
    pub awake: DesktopAwakeConfig,
}

#[cfg(feature = "desktop")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct DesktopDaemonConfig {
    #[serde(default)]
    pub quiet: bool,
    #[serde(default, rename = "noTray")]
    pub no_tray: bool,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DesktopBinding {
    pub hotkey: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DesktopRemap {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default)]
    pub exact: bool,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DesktopSnippet {
    pub trigger: String,
    pub expand: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default)]
    pub immediate: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paste: Option<String>,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DesktopLayout {
    pub name: String,
    #[serde(default)]
    pub template: DesktopLayoutTemplate,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub bindings: std::collections::BTreeMap<String, usize>,
}

#[cfg(feature = "desktop")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct DesktopLayoutTemplate {
    #[serde(default, rename = "type")]
    pub layout_type: String,
    #[serde(default)]
    pub rows: Option<u32>,
    #[serde(default)]
    pub cols: Option<u32>,
    #[serde(default)]
    pub gap: Option<u32>,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DesktopWorkspace {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apps: Vec<DesktopWorkspaceApp>,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DesktopWorkspaceApp {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<[i32; 4]>,
}

#[cfg(feature = "desktop")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct DesktopThemeConfig {
    #[serde(default, rename = "followNightlight")]
    pub follow_nightlight: bool,
    #[serde(
        default,
        rename = "scheduleLightAt",
        skip_serializing_if = "Option::is_none"
    )]
    pub schedule_light_at: Option<String>,
    #[serde(
        default,
        rename = "scheduleDarkAt",
        skip_serializing_if = "Option::is_none"
    )]
    pub schedule_dark_at: Option<String>,
}

#[cfg(feature = "desktop")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct DesktopAwakeConfig {
    #[serde(default, rename = "defaultDisplayOn")]
    pub default_display_on: bool,
}
