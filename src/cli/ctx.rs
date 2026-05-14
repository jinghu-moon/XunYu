use clap::{Args, Parser, Subcommand};

use super::defaults::default_output_format;

/// Context switch profiles.
#[derive(Parser, Debug, Clone)]
pub struct CtxCmd {
    #[command(subcommand)]
    pub cmd: CtxSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CtxSubCommand {
    Set(CtxSetCmd),
    Use(CtxUseCmd),
    Off(CtxOffCmd),
    List(CtxListCmd),
    Show(CtxShowCmd),
    Del(CtxDelCmd),
    Rename(CtxRenameCmd),
}

/// Define or update a context profile.
#[derive(Args, Debug, Clone)]
pub struct CtxSetCmd {
    /// profile name
    pub name: String,

    /// working directory
    #[arg(long)]
    pub path: Option<String>,

    /// proxy: <url> | off | keep
    #[arg(long)]
    pub proxy: Option<String>,

    /// NO_PROXY (when proxy is set)
    #[arg(long)]
    pub noproxy: Option<String>,

    /// default tags (comma separated), or "-" to clear
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// environment variable (KEY=VALUE), repeatable
    #[arg(long)]
    pub env: Vec<String>,

    /// import env from file (dotenv format)
    #[arg(long)]
    pub env_file: Option<String>,
}

/// Activate a context profile.
#[derive(Args, Debug, Clone)]
pub struct CtxUseCmd {
    /// profile name
    pub name: String,
}

/// Deactivate current profile.
#[derive(Args, Debug, Clone)]
pub struct CtxOffCmd {}

/// List profiles.
#[derive(Args, Debug, Clone)]
pub struct CtxListCmd {
    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

/// Show profile details (default: active profile).
#[derive(Args, Debug, Clone)]
pub struct CtxShowCmd {
    /// profile name (optional, defaults to active)
    pub name: Option<String>,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

/// Delete a profile.
#[derive(Args, Debug, Clone)]
pub struct CtxDelCmd {
    /// profile name
    pub name: String,
}

/// Rename a profile.
#[derive(Args, Debug, Clone)]
pub struct CtxRenameCmd {
    /// old name
    pub old: String,

    /// new name
    pub new: String,
}
