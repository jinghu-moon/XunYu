use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::path_guard::string_check;
use crate::path_guard::PathIssueKind;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub alias: BTreeMap<String, ShellAlias>,
    #[serde(default)]
    pub app: BTreeMap<String, AppAlias>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AliasMode {
    #[default]
    Auto,
    Exe,
    Cmd,
}

impl std::str::FromStr for AliasMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "exe" => Ok(Self::Exe),
            "cmd" => Ok(Self::Cmd),
            other => Err(format!(
                "Unsupported mode: {other} (expected auto|exe|cmd)."
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellAlias {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shells: Vec<String>,
    #[serde(default)]
    pub mode: AliasMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppAlias {
    pub exe: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default = "default_true")]
    pub register_apppaths: bool,
}

const fn default_true() -> bool {
    true
}

impl ShellAlias {
    pub(crate) fn applies_to_shell(&self, shell: &str) -> bool {
        self.shells.is_empty() || self.shells.iter().any(|v| v.eq_ignore_ascii_case(shell))
    }
}

/// Windows 文件名非法字符（文件名场景，比路径更严格）
/// 注：path_guard 的 check_chars() 只检查 < > " | * 和控制字符，
/// 因为 / \ : 在完整路径中是合法的。文件名场景需要额外检查。
const INVALID_FILENAME_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// 校验别名名称合法性：
/// - 不能为空
/// - 不能包含 Windows 文件名非法字符（/ \ : * ? " < > |）
/// - 不能包含空格（shim 文件名和 shell profile 中均有歧义）
/// - 不能以点开头或结尾（Windows 保留）
/// - 不能是 Windows 保留设备名（使用 path_guard 检查）
pub fn validate_alias_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Alias name cannot be empty.");
    }

    // Check for invalid filename characters
    if let Some(ch) = name.chars().find(|c| INVALID_FILENAME_CHARS.contains(c)) {
        bail!(
            "Alias name {:?} contains invalid character {:?}. \
             Characters / \\ : * ? \" < > | are not allowed.",
            name,
            ch
        );
    }

    // Alias-specific check: no spaces allowed (shim filename and shell profile ambiguity)
    if name.contains(' ') {
        bail!(
            "Alias name {:?} contains a space. Spaces are not allowed in alias names.",
            name
        );
    }

    // Use path_guard's check_component() for trailing dot/space and reserved name detection
    let wide: Vec<u16> = OsStr::new(name).encode_wide().collect();
    if let Some(kind) = string_check::check_component(&wide, false) {
        match kind {
            PathIssueKind::TrailingDotSpace => {
                bail!("Alias name {:?} cannot end with a dot or space.", name);
            }
            PathIssueKind::ReservedName => {
                bail!("Alias name {:?} is a reserved Windows device name.", name);
            }
            _ => {}
        }
    }

    // Alias-specific check: cannot start with a dot
    if name.starts_with('.') {
        bail!("Alias name {:?} cannot start with a dot.", name);
    }

    Ok(())
}

impl Config {
    pub(crate) fn name_exists(&self, name: &str) -> bool {
        self.alias.contains_key(name) || self.app.contains_key(name)
    }
}

pub(crate) fn config_path(override_path: Option<&Path>) -> PathBuf {
    if let Some(p) = override_path {
        return p.to_path_buf();
    }
    appdata_root().join("aliases.toml")
}

pub(crate) fn legacy_config_path() -> PathBuf {
    appdata_root().join("alias").join("aliases.toml")
}

pub(crate) fn shims_dir(config_file: &Path) -> PathBuf {
    config_file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("shims")
}

pub(crate) fn shim_template_path(config_file: &Path) -> PathBuf {
    config_file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("shim-template.exe")
}

pub(crate) fn shim_gui_template_path(config_file: &Path) -> PathBuf {
    config_file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("shim-template-gui.exe")
}

pub(crate) fn load(path: &Path) -> Result<Config> {
    migrate_legacy_if_needed(path)?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("Failed to read aliases config: {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("Invalid TOML: {}", path.display()))
}

pub(crate) fn save(path: &Path, cfg: &Config) -> Result<()> {
    let text = toml::to_string_pretty(cfg).context("Failed to serialize aliases config")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create aliases config dir: {}", parent.display())
        })?;
    }

    if path.exists()
        && fs::read_to_string(path)
            .map(|old| old == text)
            .unwrap_or(false)
    {
        return Ok(());
    }

    let tmp = path.with_extension("toml.tmp");
    let bak = path.with_extension("toml.bak");
    fs::write(&tmp, text.as_bytes())
        .with_context(|| format!("Failed to write temp file: {}", tmp.display()))?;

    if path.exists()
        && let Err(err) = fs::copy(path, &bak)
    {
        eprintln!(
            "Warning: failed to create aliases config backup {}: {err}",
            bak.display()
        );
    }

    replace_file(&tmp, path)
        .with_context(|| format!("Failed to replace aliases config: {}", path.display()))?;
    Ok(())
}

pub(crate) fn migrate_legacy_if_needed(path: &Path) -> Result<()> {
    let default_new = config_path(None);
    if path != default_new {
        return Ok(());
    }

    if path.exists() {
        return Ok(());
    }

    let legacy = legacy_config_path();
    if !legacy.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create migration dir: {}", parent.display()))?;
    }
    fs::copy(&legacy, path).with_context(|| {
        format!(
            "Failed to migrate legacy aliases config: {} -> {}",
            legacy.display(),
            path.display()
        )
    })?;
    let _ = fs::copy(path, path.with_extension("toml.bak"));
    Ok(())
}

fn appdata_root() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("xun")
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let mut from_w: Vec<u16> = from.as_os_str().encode_wide().collect();
    from_w.push(0);
    let mut to_w: Vec<u16> = to.as_os_str().encode_wide().collect();
    to_w.push(0);

    // MoveFileExW 的原子性为 best-effort：这里配合 .bak + 回读校验兜底。
    let ok = unsafe {
        MoveFileExW(
            from_w.as_ptr(),
            to_w.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_mode_parse() {
        assert_eq!("auto".parse::<AliasMode>().unwrap(), AliasMode::Auto);
        assert_eq!("exe".parse::<AliasMode>().unwrap(), AliasMode::Exe);
        assert_eq!("cmd".parse::<AliasMode>().unwrap(), AliasMode::Cmd);
        assert!("bad".parse::<AliasMode>().is_err());
    }

    #[test]
    fn default_apppaths_true() {
        let text = r#"
[app.code]
exe = "C:\\Code.exe"
"#;
        let cfg: Config = toml::from_str(text).unwrap();
        assert!(cfg.app["code"].register_apppaths);
    }
}
