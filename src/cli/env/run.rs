use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "template")]
/// Expand one %VAR% template string.
pub struct EnvTemplateCmd {
    /// template text, e.g. "Path=%PATH%"
    #[argh(positional)]
    pub input: String,

    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// check references and cycles only
    #[argh(switch)]
    pub validate_only: bool,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "run")]
/// Run command with merged/expanded environment.
pub struct EnvRunCmd {
    /// optional env file(s), repeatable
    #[argh(option, long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[argh(option)]
    pub set: Vec<String>,

    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// print exports for shell: bash|powershell|cmd
    #[argh(option)]
    pub shell: Option<String>,

    /// validate schema before running command
    #[argh(switch)]
    pub schema_check: bool,

    /// send desktop notification on command finish
    #[argh(switch)]
    pub notify: bool,

    /// command + args (recommended after --)
    #[argh(positional)]
    pub command: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "tui")]
/// Launch the Env TUI panel.
pub struct EnvTuiCmd {}
