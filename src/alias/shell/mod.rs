#[cfg(feature = "alias-shell-extra")]
pub(crate) mod bash;
pub(crate) mod cmd;
#[cfg(feature = "alias-shell-extra")]
pub(crate) mod nu;
pub(crate) mod ps;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::alias::config::Config;

pub(crate) const MARKER_START: &str = "# === XUN_ALIAS BEGIN ===";
pub(crate) const MARKER_END: &str = "# === XUN_ALIAS END ===";
pub(crate) const CMD_MARKER_START: &str = "REM === XUN_ALIAS BEGIN ===";
pub(crate) const CMD_MARKER_END: &str = "REM === XUN_ALIAS END ===";

pub(crate) trait ShellBackend {
    fn name(&self) -> &str;
    fn generate_block(&self, config: &Config) -> String;
    fn update(&self, config: &Config) -> Result<UpdateResult>;
    fn is_available(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UpdateResult {
    Written { path: PathBuf },
    Skipped { reason: String },
}

pub(crate) fn inject_block(content: &str, new_block: &str, start: &str, end: &str) -> String {
    if let (Some(s), Some(e)) = (content.find(start), content.find(end)) {
        if s < e {
            let before = content[..s].trim_end_matches('\n');
            let after = content[e + end.len()..].trim_start_matches('\n');
            if after.is_empty() {
                format!("{before}\n{new_block}\n")
            } else {
                format!("{before}\n{new_block}\n{after}")
            }
        } else {
            append_block(content, new_block)
        }
    } else {
        append_block(content, new_block)
    }
}

fn append_block(content: &str, block: &str) -> String {
    if content.is_empty() {
        format!("{block}\n")
    } else if content.ends_with('\n') {
        format!("{content}\n{block}\n")
    } else {
        format!("{content}\n\n{block}\n")
    }
}

pub(crate) fn read_or_empty(path: &Path) -> Result<String> {
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path.display()))
}

pub(crate) fn atomic_write_if_changed(
    path: &Path,
    current_content: &str,
    new_content: &str,
) -> Result<bool> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dir: {}", parent.display()))?;
    }
    if current_content == new_content {
        return Ok(false);
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, new_content.as_bytes())
        .with_context(|| format!("Failed to write temp file: {}", tmp.display()))?;
    fs::rename(&tmp, path)
        .with_context(|| format!("Failed to replace file: {}", path.display()))?;
    Ok(true)
}

#[cfg(feature = "alias-shell-extra")]
pub(crate) fn win_path_to_bash(path: &str) -> String {
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        let drive = path.chars().next().unwrap_or('c').to_ascii_lowercase();
        let tail = path[2..].replace('\\', "/");
        format!("/{drive}{tail}")
    } else {
        path.replace('\\', "/")
    }
}
