use clap::{Args, Parser, Subcommand};

/// Xunbak container and 7-Zip plugin tooling.
#[derive(Parser, Clone, Debug, PartialEq, Eq)]
pub struct XunbakCmd {
    #[command(subcommand)]
    pub cmd: XunbakSubCommand,
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum XunbakSubCommand {
    Plugin(XunbakPluginCmd),
}

/// Manage the xunbak 7-Zip plugin.
#[derive(Parser, Clone, Debug, PartialEq, Eq)]
pub struct XunbakPluginCmd {
    #[command(subcommand)]
    pub cmd: XunbakPluginSubCommand,
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum XunbakPluginSubCommand {
    Install(XunbakPluginInstallCmd),
    Uninstall(XunbakPluginUninstallCmd),
    Doctor(XunbakPluginDoctorCmd),
}

/// Install xunbak.dll into an existing 7-Zip installation.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
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
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct XunbakPluginUninstallCmd {
    /// explicit 7-Zip home, e.g. C:/Program Files/7-Zip
    #[arg(long)]
    pub sevenzip_home: Option<String>,

    /// also remove the current-user .xunbak association when it points to 7-Zip
    #[arg(long)]
    pub remove_association: bool,
}

/// Diagnose xunbak 7-Zip plugin readiness.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct XunbakPluginDoctorCmd {
    /// explicit 7-Zip home, e.g. C:/Program Files/7-Zip
    #[arg(long)]
    pub sevenzip_home: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> XunbakCmd {
        let mut raw = vec!["xunbak"];
        raw.extend_from_slice(args);
        XunbakCmd::try_parse_from(&raw).expect("parse xunbak cmd")
    }

    #[test]
    fn parse_xunbak_plugin_install() {
        let cmd = parse(&[
            "plugin",
            "install",
            "--sevenzip-home",
            "C:/Program Files/7-Zip",
            "--config",
            "release",
            "--associate",
        ]);
        assert!(matches!(
            cmd.cmd,
            XunbakSubCommand::Plugin(XunbakPluginCmd {
                cmd: XunbakPluginSubCommand::Install(_)
            })
        ));
    }

    #[test]
    fn parse_xunbak_plugin_uninstall() {
        let cmd = parse(&["plugin", "uninstall", "--remove-association"]);
        assert!(matches!(
            cmd.cmd,
            XunbakSubCommand::Plugin(XunbakPluginCmd {
                cmd: XunbakPluginSubCommand::Uninstall(_)
            })
        ));
    }

    #[test]
    fn parse_xunbak_plugin_doctor() {
        let cmd = parse(&["plugin", "doctor"]);
        assert!(matches!(
            cmd.cmd,
            XunbakSubCommand::Plugin(XunbakPluginCmd {
                cmd: XunbakPluginSubCommand::Doctor(_)
            })
        ));
    }
}
