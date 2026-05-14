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
    pub sub: XunbakSubCommand,
}

/// Xunbak 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum XunbakSubCommand {
    /// Manage the xunbak 7-Zip plugin
    Plugin(XunbakPluginArgs),
}

// ── Plugin 嵌套子命令 ────────────────────────────────────────────

/// Manage the xunbak 7-Zip plugin.
#[derive(Parser, Debug, Clone)]
pub struct XunbakPluginArgs {
    #[command(subcommand)]
    pub sub: XunbakPluginSubCommand,
}

/// Plugin 子命令枚举（3 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum XunbakPluginSubCommand {
    /// Install xunbak.dll into an existing 7-Zip installation
    Install(XunbakPluginInstallArgs),
    /// Remove xunbak.dll from an existing 7-Zip installation
    Uninstall(XunbakPluginUninstallArgs),
    /// Diagnose xunbak 7-Zip plugin readiness
    Doctor(XunbakPluginDoctorArgs),
}

/// Install xunbak.dll into an existing 7-Zip installation.
#[derive(Parser, Debug, Clone)]
pub struct XunbakPluginInstallArgs {
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
pub struct XunbakPluginUninstallArgs {
    /// explicit 7-Zip home, e.g. C:/Program Files/7-Zip
    #[arg(long)]
    pub sevenzip_home: Option<String>,

    /// also remove the current-user .xunbak association when it points to 7-Zip
    #[arg(long)]
    pub remove_association: bool,
}

/// Diagnose xunbak 7-Zip plugin readiness.
#[derive(Parser, Debug, Clone)]
pub struct XunbakPluginDoctorArgs {
    /// explicit 7-Zip home, e.g. C:/Program Files/7-Zip
    #[arg(long)]
    pub sevenzip_home: Option<String>,
}
