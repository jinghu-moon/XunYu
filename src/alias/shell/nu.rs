use std::path::PathBuf;

use anyhow::Result;

use crate::alias::config::Config;
use crate::alias::shell::{
    MARKER_END, MARKER_START, ShellBackend, UpdateResult, atomic_write_if_changed, inject_block,
    read_or_empty,
};

pub(crate) struct NuBackend {
    config_path: Option<PathBuf>,
}

impl NuBackend {
    pub(crate) fn new(path: Option<PathBuf>) -> Self {
        Self { config_path: path }
    }

    fn resolve_config(&self) -> Option<PathBuf> {
        if let Some(p) = &self.config_path {
            return Some(p.clone());
        }
        if let Ok(p) = std::env::var("XUN_ALIAS_NU_CONFIG") {
            return Some(PathBuf::from(p));
        }
        let appdata = std::env::var("APPDATA").ok()?;
        Some(PathBuf::from(appdata).join("nushell").join("config.nu"))
    }
}

impl ShellBackend for NuBackend {
    fn name(&self) -> &str {
        "nushell"
    }

    fn generate_block(&self, cfg: &Config) -> String {
        let mut lines = Vec::with_capacity(cfg.alias.len() + cfg.app.len() + 2);
        lines.push(MARKER_START.to_string());
        for (name, alias) in &cfg.alias {
            if !alias.applies_to_shell("nu") {
                continue;
            }
            lines.push(format!(
                "def --wrapped {name} [...rest] {{ {} ...$rest }}",
                alias.command
            ));
        }
        for (name, app) in &cfg.app {
            let exe = escape_double_quotes(&app.exe);
            if let Some(args) = app.args.as_deref() {
                let args = escape_double_quotes(args);
                lines.push(format!(
                    r#"def --wrapped {name} [...rest] {{ ^"{}" {} ...$rest }}"#,
                    exe, args
                ));
            } else {
                lines.push(format!(
                    r#"def --wrapped {name} [...rest] {{ ^"{}" ...$rest }}"#,
                    exe
                ));
            }
        }
        lines.push(MARKER_END.to_string());
        lines.join("\n")
    }

    fn update(&self, cfg: &Config) -> Result<UpdateResult> {
        let Some(path) = self.resolve_config() else {
            return Ok(UpdateResult::Skipped {
                reason: "nushell config not found".to_string(),
            });
        };
        let content = read_or_empty(&path)?;
        let block = self.generate_block(cfg);
        let updated = inject_block(&content, &block, MARKER_START, MARKER_END);
        atomic_write_if_changed(&path, &content, &updated)?;
        Ok(UpdateResult::Written { path })
    }

    fn is_available(&self) -> bool {
        self.resolve_config().is_some()
    }
}

fn escape_double_quotes(value: &str) -> String {
    value.replace('"', "\\\"")
}
