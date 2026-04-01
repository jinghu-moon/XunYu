use serde::{Deserialize, Serialize};
#[cfg(feature = "redirect")]
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize, Clone)]
pub(crate) struct GlobalConfig {
    #[serde(default)]
    pub(crate) bookmark: BookmarkConfig,
    #[serde(default)]
    pub(crate) tree: TreeConfig,
    #[serde(default)]
    pub(crate) proxy: ProxyConfig,
    #[serde(default)]
    pub(crate) acl: AclConfig,
    #[cfg(feature = "desktop")]
    #[serde(default)]
    pub(crate) desktop: DesktopConfig,
    #[cfg(feature = "protect")]
    #[serde(default)]
    pub(crate) protect: ProtectConfig,
    #[cfg(feature = "redirect")]
    #[serde(default)]
    pub(crate) redirect: RedirectConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub(crate) struct BookmarkConfig {
    pub(crate) version: u32,
    #[serde(rename = "dataFile")]
    pub(crate) data_file: String,
    #[serde(rename = "visitLogFile")]
    pub(crate) visit_log_file: String,
    #[serde(rename = "defaultScope")]
    pub(crate) default_scope: String,
    #[serde(rename = "defaultListLimit")]
    pub(crate) default_list_limit: usize,
    #[serde(rename = "maxAge")]
    pub(crate) max_age: u64,
    #[serde(rename = "resolveSymlinks")]
    pub(crate) resolve_symlinks: bool,
    pub(crate) echo: bool,
    #[serde(rename = "excludeDirs")]
    pub(crate) exclude_dirs: Vec<String>,
    #[serde(rename = "autoLearn")]
    pub(crate) auto_learn: BookmarkAutoLearnConfig,
    pub(crate) fzf: BookmarkFzfConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub(crate) struct BookmarkAutoLearnConfig {
    pub(crate) enabled: bool,
    #[serde(rename = "importHistoryOnFirstInit")]
    pub(crate) import_history_on_first_init: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub(crate) struct BookmarkFzfConfig {
    #[serde(rename = "minVersion")]
    pub(crate) min_version: String,
    pub(crate) opts: String,
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
#[derive(Default)]
pub(crate) enum RedirectOnConflict {
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
#[derive(Default)]
pub(crate) enum RedirectUnmatched {
    #[default]
    Skip,
    Archive {
        age_expr: String,
        dest: String,
    },
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

#[cfg(feature = "desktop")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopConfig {
    #[serde(default)]
    pub(crate) daemon: DesktopDaemonConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) bindings: Vec<DesktopBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) remaps: Vec<DesktopRemap>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) snippets: Vec<DesktopSnippet>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) layouts: Vec<DesktopLayout>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) workspaces: Vec<DesktopWorkspace>,
    #[serde(default)]
    pub(crate) theme: DesktopThemeConfig,
    #[serde(default)]
    pub(crate) awake: DesktopAwakeConfig,
}

#[cfg(feature = "desktop")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopDaemonConfig {
    #[serde(default)]
    pub(crate) quiet: bool,
    #[serde(default, rename = "noTray")]
    pub(crate) no_tray: bool,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopBinding {
    pub(crate) hotkey: String,
    pub(crate) action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) app: Option<String>,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopRemap {
    pub(crate) from: String,
    pub(crate) to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) app: Option<String>,
    #[serde(default)]
    pub(crate) exact: bool,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopSnippet {
    pub(crate) trigger: String,
    pub(crate) expand: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) app: Option<String>,
    #[serde(default)]
    pub(crate) immediate: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) paste: Option<String>,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopLayout {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) template: DesktopLayoutTemplate,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub(crate) bindings: std::collections::BTreeMap<String, usize>,
}

#[cfg(feature = "desktop")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopLayoutTemplate {
    #[serde(default, rename = "type")]
    pub(crate) layout_type: String,
    #[serde(default)]
    pub(crate) rows: Option<u32>,
    #[serde(default)]
    pub(crate) cols: Option<u32>,
    #[serde(default)]
    pub(crate) gap: Option<u32>,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopWorkspace {
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) apps: Vec<DesktopWorkspaceApp>,
}

#[cfg(feature = "desktop")]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopWorkspaceApp {
    pub(crate) path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) args: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) rect: Option<[i32; 4]>,
}

#[cfg(feature = "desktop")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopThemeConfig {
    #[serde(default, rename = "followNightlight")]
    pub(crate) follow_nightlight: bool,
    #[serde(
        default,
        rename = "scheduleLightAt",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) schedule_light_at: Option<String>,
    #[serde(
        default,
        rename = "scheduleDarkAt",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) schedule_dark_at: Option<String>,
}

#[cfg(feature = "desktop")]
#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DesktopAwakeConfig {
    #[serde(default, rename = "defaultDisplayOn")]
    pub(crate) default_display_on: bool,
}
