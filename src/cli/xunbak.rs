use argh::FromArgs;

/// Xunbak container and 7-Zip plugin tooling.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "xunbak")]
pub struct XunbakCmd {
    #[argh(subcommand)]
    pub cmd: XunbakSubCommand,
}

#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand)]
pub enum XunbakSubCommand {
    Plugin(XunbakPluginCmd),
}

/// Manage the xunbak 7-Zip plugin.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "plugin")]
pub struct XunbakPluginCmd {
    #[argh(subcommand)]
    pub cmd: XunbakPluginSubCommand,
}

#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand)]
pub enum XunbakPluginSubCommand {
    Install(XunbakPluginInstallCmd),
    Uninstall(XunbakPluginUninstallCmd),
    Doctor(XunbakPluginDoctorCmd),
}

/// Install xunbak.dll into an existing 7-Zip installation.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "install")]
pub struct XunbakPluginInstallCmd {
    /// explicit 7-Zip home, e.g. C:/Program Files/7-Zip
    #[argh(option)]
    pub sevenzip_home: Option<String>,

    /// plugin build config: debug | release
    #[argh(option)]
    pub config: Option<String>,

    /// refuse to replace an existing xunbak.dll
    #[argh(switch)]
    pub no_overwrite: bool,

    /// also associate .xunbak with 7zFM.exe under the current user
    #[argh(switch)]
    pub associate: bool,
}

/// Remove xunbak.dll from an existing 7-Zip installation.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "uninstall")]
pub struct XunbakPluginUninstallCmd {
    /// explicit 7-Zip home, e.g. C:/Program Files/7-Zip
    #[argh(option)]
    pub sevenzip_home: Option<String>,

    /// also remove the current-user .xunbak association when it points to 7-Zip
    #[argh(switch)]
    pub remove_association: bool,
}

/// Diagnose xunbak 7-Zip plugin readiness.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "doctor")]
pub struct XunbakPluginDoctorCmd {
    /// explicit 7-Zip home, e.g. C:/Program Files/7-Zip
    #[argh(option)]
    pub sevenzip_home: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> XunbakCmd {
        <XunbakCmd as argh::FromArgs>::from_args(&["xunbak"], args).expect("parse xunbak cmd")
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
