use std::path::PathBuf;

use anyhow::Result;

use crate::alias::config::Config;
use crate::alias::shell::{
    MARKER_END, MARKER_START, ShellBackend, UpdateResult, atomic_write, inject_block, read_or_empty,
};

pub(crate) struct PsBackend {
    pub(crate) profile_path: Option<PathBuf>,
}

impl PsBackend {
    pub(crate) fn new(profile_path: Option<PathBuf>) -> Self {
        Self { profile_path }
    }

    fn resolve_profile(&self) -> Option<PathBuf> {
        if let Some(p) = &self.profile_path {
            return Some(p.clone());
        }
        detect_ps_profile()
    }
}

impl ShellBackend for PsBackend {
    fn name(&self) -> &str {
        "powershell"
    }

    fn generate_block(&self, cfg: &Config) -> String {
        generate_ps_block(cfg)
    }

    fn update(&self, cfg: &Config) -> Result<UpdateResult> {
        let Some(path) = self.resolve_profile() else {
            return Ok(UpdateResult::Skipped {
                reason: "PowerShell profile not found".to_string(),
            });
        };
        let content = read_or_empty(&path)?;
        let block = self.generate_block(cfg);
        let updated = inject_block(&content, &block, MARKER_START, MARKER_END);
        atomic_write(&path, &updated)?;
        Ok(UpdateResult::Written { path })
    }

    fn is_available(&self) -> bool {
        self.resolve_profile().is_some()
    }
}

pub(crate) fn detect_ps_profile() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("XUN_ALIAS_PS_PROFILE") {
        return Some(PathBuf::from(p));
    }
    let home = std::env::var("USERPROFILE").ok()?;
    let ps7 = PathBuf::from(&home)
        .join("Documents")
        .join("PowerShell")
        .join("Microsoft.PowerShell_profile.ps1");
    if ps7.exists() {
        return Some(ps7);
    }
    Some(
        PathBuf::from(home)
            .join("Documents")
            .join("WindowsPowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
    )
}

fn generate_ps_block(cfg: &Config) -> String {
    let mut lines = Vec::new();
    lines.push(MARKER_START.to_string());
    for (name, alias) in &cfg.alias {
        if !alias.applies_to_shell("ps") {
            continue;
        }
        lines.push(alias_to_ps(name, &alias.command));
    }
    for (name, app) in &cfg.app {
        lines.push(app_to_ps(name, &app.exe, app.args.as_deref()));
    }
    lines.push(MARKER_END.to_string());
    lines.join("\n")
}

fn alias_to_ps(name: &str, command: &str) -> String {
    let is_simple = !command.contains(|c: char| {
        c.is_whitespace() || matches!(c, '|' | '&' | ';' | '<' | '>' | '$' | '`')
    });
    if is_simple {
        format!("Set-Alias {name} {command}")
    } else {
        format!("function {name} {{ {command} @args }}")
    }
}

fn app_to_ps(name: &str, exe: &str, fixed_args: Option<&str>) -> String {
    let exe = exe.replace('\'', "''");
    if let Some(args) = fixed_args {
        let args = args.replace('\'', "''");
        format!("function {name} {{ Start-Process '{exe}' -ArgumentList (@('{args}') + $args) }}")
    } else {
        format!("function {name} {{ Start-Process '{exe}' -ArgumentList $args }}")
    }
}
