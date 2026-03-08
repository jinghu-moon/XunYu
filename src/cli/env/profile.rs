use super::super::defaults::default_output_format;
use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "profile")]
/// Profile operations.
pub struct EnvProfileCmd {
    #[argh(subcommand)]
    pub cmd: EnvProfileSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvProfileSubCommand {
    List(EnvProfileListCmd),
    Capture(EnvProfileCaptureCmd),
    Apply(EnvProfileApplyCmd),
    Diff(EnvProfileDiffCmd),
    Delete(EnvProfileDeleteCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// List profiles.
pub struct EnvProfileListCmd {
    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "capture")]
/// Capture current scope vars into a profile.
pub struct EnvProfileCaptureCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "apply")]
/// Apply one profile.
pub struct EnvProfileApplyCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,

    /// optional target scope override: user|system
    #[argh(option)]
    pub scope: Option<String>,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "diff")]
/// Diff profile against live scope.
pub struct EnvProfileDiffCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,

    /// optional target scope override: user|system
    #[argh(option)]
    pub scope: Option<String>,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "delete")]
/// Delete one profile.
pub struct EnvProfileDeleteCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}
