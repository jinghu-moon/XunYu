use std::path::PathBuf;

use anyhow::Result;

use crate::alias::config::Config;
use crate::alias::shell::{
    MARKER_END, MARKER_START, ShellBackend, UpdateResult, atomic_write, inject_block,
    read_or_empty, win_path_to_bash,
};

pub(crate) struct BashBackend {
    bashrc_path: Option<PathBuf>,
}

impl BashBackend {
    pub(crate) fn new(path: Option<PathBuf>) -> Self {
        Self { bashrc_path: path }
    }

    fn resolve_bashrc(&self) -> Option<PathBuf> {
        if let Some(p) = &self.bashrc_path {
            return Some(p.clone());
        }
        if let Ok(p) = std::env::var("XUN_ALIAS_BASHRC") {
            return Some(PathBuf::from(p));
        }
        let home = std::env::var("USERPROFILE")
            .ok()
            .or_else(|| std::env::var("HOME").ok())?;
        Some(PathBuf::from(home).join(".bashrc"))
    }
}

impl ShellBackend for BashBackend {
    fn name(&self) -> &str {
        "bash"
    }

    fn generate_block(&self, cfg: &Config) -> String {
        let mut lines = Vec::new();
        lines.push(MARKER_START.to_string());
        for (name, alias) in &cfg.alias {
            if !alias.applies_to_shell("bash") {
                continue;
            }
            lines.push(format!(
                "alias {}='{}'",
                name,
                escape_single_quotes(&alias.command)
            ));
        }
        for (name, app) in &cfg.app {
            let exe = win_path_to_bash(&app.exe);
            if let Some(args) = app.args.as_deref() {
                lines.push(format!(r#"function {name} {{ "{exe}" {args} "$@"; }}"#));
            } else {
                lines.push(format!(r#"function {name} {{ "{exe}" "$@"; }}"#));
            }
        }
        lines.push(MARKER_END.to_string());
        lines.join("\n")
    }

    fn update(&self, cfg: &Config) -> Result<UpdateResult> {
        let Some(path) = self.resolve_bashrc() else {
            return Ok(UpdateResult::Skipped {
                reason: "bash profile not found".to_string(),
            });
        };
        let content = read_or_empty(&path)?;
        let block = self.generate_block(cfg);
        let updated = inject_block(&content, &block, MARKER_START, MARKER_END);
        atomic_write(&path, &updated)?;
        Ok(UpdateResult::Written { path })
    }

    fn is_available(&self) -> bool {
        self.resolve_bashrc().is_some()
    }
}

fn escape_single_quotes(s: &str) -> String {
    s.replace('\'', r"'\''")
}
