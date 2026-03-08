use std::collections::HashSet;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::alias::config::{AliasMode, AppAlias, Config, ShellAlias};

const EMBEDDED_SHIM_TEMPLATE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/alias_shim_template.bin"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShimKind {
    Exe {
        path: String,
        fixed_args: Option<String>,
    },
    Cmd {
        command: String,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct SyncEntry {
    pub(crate) name: String,
    pub(crate) shim_content: String,
    pub(crate) use_gui_template: bool,
}

#[derive(Debug, Default)]
pub(crate) struct SyncReport {
    pub(crate) created: Vec<String>,
    pub(crate) removed: Vec<String>,
    pub(crate) errors: Vec<(String, String)>,
}

mod classify;
mod io;
mod pe_patch;
mod render;
mod sync;
mod template;

pub(crate) use render::{app_alias_to_shim, shell_alias_to_shim};
pub(crate) use sync::{config_to_sync_entries, remove_shim, sync_all};
pub(crate) use template::deploy_shim_templates;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alias::config::{AliasMode, ShellAlias};

    #[test]
    fn auto_mode_detects_cmd_operators() {
        let kind = classify::classify_mode("git status | findstr M", AliasMode::Auto);
        assert!(matches!(kind, ShimKind::Cmd { .. }));
    }

    #[test]
    fn mode_cmd_forces_cmd() {
        let kind = classify::classify_mode("notepad.exe", AliasMode::Cmd);
        assert!(matches!(kind, ShimKind::Cmd { .. }));
    }

    #[test]
    fn shell_alias_shim_contains_mode() {
        let alias = ShellAlias {
            command: "echo hi".to_string(),
            desc: None,
            tags: vec![],
            shells: vec![],
            mode: AliasMode::Cmd,
        };
        let text = shell_alias_to_shim(&alias);
        assert!(text.contains("type = cmd"));
        assert!(text.contains("wait = true"));
    }
}
