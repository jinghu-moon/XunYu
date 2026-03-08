use super::super::defaults::default_output_format;
use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// List environment variables.
pub struct EnvListCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "search")]
/// Search environment variables by name/value.
pub struct EnvSearchCmd {
    /// keyword query
    #[argh(positional)]
    pub query: String,

    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "get")]
/// Get one environment variable.
pub struct EnvGetCmd {
    /// variable name
    #[argh(positional)]
    pub name: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
/// Set one environment variable.
pub struct EnvSetCmd {
    /// variable name
    #[argh(positional)]
    pub name: String,

    /// variable value
    #[argh(positional)]
    pub value: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// disable automatic pre-write snapshot
    #[argh(switch)]
    pub no_snapshot: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "del")]
/// Delete one environment variable.
pub struct EnvDelCmd {
    /// variable name
    #[argh(positional)]
    pub name: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}
