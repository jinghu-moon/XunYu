use super::*;

pub struct EnvDiffLiveCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// baseline snapshot id (default: latest)
    #[argh(option)]
    pub snapshot: Option<String>,

    /// baseline time, format: RFC3339 or YYYY-MM-DD or YYYY-MM-DD HH:MM:SS
    #[argh(option)]
    pub since: Option<String>,

    /// enable ANSI colors
    #[argh(switch)]
    pub color: bool,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "graph")]
/// Show variable dependency graph (%VAR% references).
pub struct EnvGraphCmd {
    /// root variable name
    #[argh(positional)]
    pub name: String,

    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// max traversal depth (1-64)
    #[argh(option, default = "8")]
    pub max_depth: usize,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "validate")]
/// Validate environment with schema rules.

