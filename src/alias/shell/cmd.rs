use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::alias::config::Config;
use crate::alias::shell::{
    CMD_MARKER_END, CMD_MARKER_START, ShellBackend, UpdateResult, atomic_write, inject_block,
    read_or_empty,
};

const CMD_AUTORUN_KEY: &str = r"Software\Microsoft\Command Processor";
const CMD_AUTORUN_VALUE: &str = "AutoRun";
const AUTORUN_BEGIN: &str = "@REM XUN_ALIAS_AUTORUN_BEGIN";
const AUTORUN_END: &str = "@REM XUN_ALIAS_AUTORUN_END";

pub(crate) struct CmdBackend {
    pub(crate) macrofile_path: PathBuf,
}

impl CmdBackend {
    pub(crate) fn new(config_dir: &Path) -> Self {
        Self {
            macrofile_path: config_dir.join("cmd_aliases.doskey"),
        }
    }
}

impl ShellBackend for CmdBackend {
    fn name(&self) -> &str {
        "cmd"
    }

    fn generate_block(&self, cfg: &Config) -> String {
        generate_macrofile(cfg)
    }

    fn update(&self, cfg: &Config) -> Result<UpdateResult> {
        if !self.is_available() {
            return Ok(UpdateResult::Skipped {
                reason: "cmd backend only available on Windows".to_string(),
            });
        }

        let content = read_or_empty(&self.macrofile_path)?;
        let block = self.generate_block(cfg);
        let updated = inject_block(&content, &block, CMD_MARKER_START, CMD_MARKER_END);
        atomic_write(&self.macrofile_path, &updated)?;
        install_autorun_merged(&self.macrofile_path)?;
        Ok(UpdateResult::Written {
            path: self.macrofile_path.clone(),
        })
    }

    fn is_available(&self) -> bool {
        cfg!(windows)
    }
}

fn generate_macrofile(cfg: &Config) -> String {
    let mut lines = Vec::new();
    lines.push(CMD_MARKER_START.to_string());
    for (name, alias) in &cfg.alias {
        if !alias.applies_to_shell("cmd") {
            continue;
        }
        lines.push(format!(
            "doskey {}={} $*",
            name,
            sanitize_for_doskey(&alias.command)
        ));
    }
    for (name, app) in &cfg.app {
        let command = if let Some(args) = app.args.as_deref() {
            format!(r#"start "" "{}" {} $*"#, app.exe, args)
        } else {
            format!(r#"start "" "{}" $*"#, app.exe)
        };
        lines.push(format!("doskey {name}={command}"));
    }
    lines.push(CMD_MARKER_END.to_string());
    lines.join("\n")
}

fn sanitize_for_doskey(command: &str) -> String {
    command.replace('$', "$$")
}

fn install_autorun_merged(macrofile: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use winreg::{RegKey, enums::*};

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey(CMD_AUTORUN_KEY)
            .with_context(|| format!("Failed to open {CMD_AUTORUN_KEY}"))?;

        // 规则：
        // 1) 无输出（@ 前缀）；
        // 2) 不调用 cmd.exe，避免递归；
        // 3) 保留原值，仅替换自己的片段。
        let inject = format!(
            r#"{AUTORUN_BEGIN} & @doskey /macrofile="{}" & {AUTORUN_END}"#,
            macrofile.display()
        );
        let existing: String = key.get_value(CMD_AUTORUN_VALUE).unwrap_or_default();
        let merged = merge_autorun(&existing, &inject);
        if merged != existing {
            key.set_value(CMD_AUTORUN_VALUE, &merged)
                .context("Failed to write merged AutoRun value")?;
        }
    }
    #[cfg(not(windows))]
    {
        let _ = macrofile;
    }
    Ok(())
}

fn merge_autorun(existing: &str, inject: &str) -> String {
    if existing.trim().is_empty() {
        return inject.to_string();
    }
    if let (Some(s), Some(e)) = (existing.find(AUTORUN_BEGIN), existing.find(AUTORUN_END))
        && s < e
    {
        let head = existing[..s].trim().trim_end_matches('&').trim();
        let tail = existing[e + AUTORUN_END.len()..]
            .trim()
            .trim_start_matches('&')
            .trim();
        return match (head.is_empty(), tail.is_empty()) {
            (true, true) => inject.to_string(),
            (false, true) => format!("{head} & {inject}"),
            (true, false) => format!("{inject} & {tail}"),
            (false, false) => format!("{head} & {inject} & {tail}"),
        };
    }
    if existing.contains("@doskey /macrofile=") {
        // 兼容旧值：不做复杂替换，直接追加新片段（保持无破坏）。
        format!("{existing} & {inject}")
    } else {
        format!("{existing} & {inject}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_autorun_appends_when_empty() {
        let inject =
            "@REM XUN_ALIAS_AUTORUN_BEGIN & @doskey /macrofile=\"x\" & @REM XUN_ALIAS_AUTORUN_END";
        assert_eq!(merge_autorun("", inject), inject);
    }

    #[test]
    fn merge_autorun_replaces_old_block() {
        let old = "foo & @REM XUN_ALIAS_AUTORUN_BEGIN & old & @REM XUN_ALIAS_AUTORUN_END & bar";
        let inject = "@REM XUN_ALIAS_AUTORUN_BEGIN & new & @REM XUN_ALIAS_AUTORUN_END";
        let merged = merge_autorun(old, inject);
        assert!(merged.contains("foo"));
        assert!(merged.contains("bar"));
        assert!(merged.contains("new"));
        assert!(!merged.contains(" old "));
    }
}
