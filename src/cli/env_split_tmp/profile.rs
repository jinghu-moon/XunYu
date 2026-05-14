use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
/// Profile operations.
pub struct EnvProfileCmd {
    #[command(subcommand)]
    pub cmd: EnvProfileSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum EnvProfileSubCommand {
    List(EnvProfileListCmd),
    Capture(EnvProfileCaptureCmd),
    Apply(EnvProfileApplyCmd),
    Diff(EnvProfileDiffCmd),
    Delete(EnvProfileDeleteCmd),
}

#[derive(Args, Debug, Clone)]
/// List profiles.
pub struct EnvProfileListCmd {
    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Capture current scope vars into a profile.
pub struct EnvProfileCaptureCmd {
    /// profile name
    pub name: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,
}

#[derive(Args, Debug, Clone)]
/// Apply one profile.
pub struct EnvProfileApplyCmd {
    /// profile name
    pub name: String,

    /// optional target scope override: user|system
    #[arg(long)]
    pub scope: Option<String>,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug, Clone)]
/// Diff profile against live scope.
pub struct EnvProfileDiffCmd {
    /// profile name
    pub name: String,

    /// optional target scope override: user|system
    #[arg(long)]
    pub scope: Option<String>,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Delete one profile.
pub struct EnvProfileDeleteCmd {
    /// profile name
    pub name: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}
