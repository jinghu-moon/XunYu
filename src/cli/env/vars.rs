use super::super::defaults::default_output_format;
use clap::Args;

#[derive(Args, Debug, Clone)]
/// List environment variables.
pub struct EnvListCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Search environment variables by name/value.
pub struct EnvSearchCmd {
    /// keyword query
    pub query: String,

    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Get one environment variable.
pub struct EnvGetCmd {
    /// variable name
    pub name: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Set one environment variable.
pub struct EnvSetCmd {
    /// variable name
    pub name: String,

    /// variable value
    pub value: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// disable automatic pre-write snapshot
    #[arg(long)]
    pub no_snapshot: bool,
}

#[derive(Args, Debug, Clone)]
/// Delete one environment variable.
pub struct EnvDelCmd {
    /// variable name
    pub name: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}
