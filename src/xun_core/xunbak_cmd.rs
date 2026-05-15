//! Xunbak CLI 定义（clap derive）
//!
//! 新架构的 xunbak 命令定义，替代 argh 版本。
//! 嵌套结构：XunbakCmd → Plugin → Install/Uninstall/Doctor。

use clap::{Parser, Subcommand};

// ── Xunbak 主命令 ────────────────────────────────────────────────

/// Xunbak container and 7-Zip plugin tooling.
#[derive(Parser, Debug, Clone)]
#[command(name = "xunbak", about = "Xunbak container and 7-Zip plugin tooling")]
pub struct XunbakCmd {
    #[command(subcommand)]
    pub cmd: XunbakSubCommand,
}

/// Xunbak 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum XunbakSubCommand {
    /// Manage the xunbak 7-Zip plugin
    Plugin(XunbakPluginCmd),
}

// ── Plugin 嵌套子命令 ────────────────────────────────────────────

/// Manage the xunbak 7-Zip plugin.
#[derive(Parser, Debug, Clone)]
pub struct XunbakPluginCmd {
    #[command(subcommand)]
    pub cmd: XunbakPluginSubCommand,
}

/// Plugin 子命令枚举（3 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum XunbakPluginSubCommand {
    /// Install xunbak.dll into an existing 7-Zip installation
    Install(XunbakPluginInstallCmd),
    /// Remove xunbak.dll from an existing 7-Zip installation
    Uninstall(XunbakPluginUninstallCmd),
    /// Diagnose xunbak 7-Zip plugin readiness
    Doctor(XunbakPluginDoctorCmd),
}

/// Install xunbak.dll into an existing 7-Zip installation.
#[derive(Parser, Debug, Clone)]
pub struct XunbakPluginInstallCmd {
    /// explicit 7-Zip home, e.g. C:/Program Files/7-Zip
    #[arg(long)]
    pub sevenzip_home: Option<String>,

    /// plugin build config: debug | release
    #[arg(long)]
    pub config: Option<String>,

    /// refuse to replace an existing xunbak.dll
    #[arg(long)]
    pub no_overwrite: bool,

    /// also associate .xunbak with 7zFM.exe under the current user
    #[arg(long)]
    pub associate: bool,
}

/// Remove xunbak.dll from an existing 7-Zip installation.
#[derive(Parser, Debug, Clone)]
pub struct XunbakPluginUninstallCmd {
    /// explicit 7-Zip home, e.g. C:/Program Files/7-Zip
    #[arg(long)]
    pub sevenzip_home: Option<String>,

    /// also remove the current-user .xunbak association when it points to 7-Zip
    #[arg(long)]
    pub remove_association: bool,
}

/// Diagnose xunbak 7-Zip plugin readiness.
#[derive(Parser, Debug, Clone)]
pub struct XunbakPluginDoctorCmd {
    /// explicit 7-Zip home, e.g. C:/Program Files/7-Zip
    #[arg(long)]
    pub sevenzip_home: Option<String>,
}

// ============================================================
// CommandSpec 实现
// ============================================================

#[cfg(feature = "xunbak")]
use crate::xun_core::command::CommandSpec;
#[cfg(feature = "xunbak")]
use crate::xun_core::context::CmdContext;
#[cfg(feature = "xunbak")]
use crate::xun_core::error::XunError;
#[cfg(feature = "xunbak")]
use crate::xun_core::value::Value;

/// xunbak 命令。
#[cfg(feature = "xunbak")]
pub struct XunbakCmdSpec {
    pub args: XunbakCmd,
}

#[cfg(feature = "xunbak")]
impl CommandSpec for XunbakCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::xunbak::cmd_xunbak(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}
