use argh::FromArgs;

use super::defaults::default_output_format;

/// Context switch profiles.
#[derive(FromArgs)]
#[argh(subcommand, name = "ctx")]
pub struct CtxCmd {
    #[argh(subcommand)]
    pub cmd: CtxSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
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
#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
pub struct CtxSetCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,

    /// working directory
    #[argh(option)]
    pub path: Option<String>,

    /// proxy: <url> | off | keep
    #[argh(option)]
    pub proxy: Option<String>,

    /// NO_PROXY (when proxy is set)
    #[argh(option)]
    pub noproxy: Option<String>,

    /// default tags (comma separated), or "-" to clear
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// environment variable (KEY=VALUE), repeatable
    #[argh(option)]
    pub env: Vec<String>,

    /// import env from file (dotenv format)
    #[argh(option, long = "env-file")]
    pub env_file: Option<String>,
}

/// Activate a context profile.
#[derive(FromArgs)]
#[argh(subcommand, name = "use")]
pub struct CtxUseCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,
}

/// Deactivate current profile.
#[derive(FromArgs)]
#[argh(subcommand, name = "off")]
pub struct CtxOffCmd {}

/// List profiles.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct CtxListCmd {
    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

/// Show profile details (default: active profile).
#[derive(FromArgs)]
#[argh(subcommand, name = "show")]
pub struct CtxShowCmd {
    /// profile name (optional, defaults to active)
    #[argh(positional)]
    pub name: Option<String>,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

/// Delete a profile.
#[derive(FromArgs)]
#[argh(subcommand, name = "del")]
pub struct CtxDelCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,
}

/// Rename a profile.
#[derive(FromArgs)]
#[argh(subcommand, name = "rename")]
pub struct CtxRenameCmd {
    /// old name
    #[argh(positional)]
    pub old: String,

    /// new name
    #[argh(positional)]
    pub new: String,
}
