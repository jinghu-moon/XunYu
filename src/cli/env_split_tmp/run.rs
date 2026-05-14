use clap::Args;

#[derive(Args, Debug, Clone)]
/// Expand one %VAR% template string.
pub struct EnvTemplateCmd {
    /// template text, e.g. "Path=%PATH%"
    pub input: String,

    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// check references and cycles only
    #[arg(long)]
    pub validate_only: bool,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Run command with merged/expanded environment.
pub struct EnvRunCmd {
    /// optional env file(s), repeatable
    #[arg(long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[arg(long)]
    pub set: Vec<String>,

    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// print exports for shell: bash|powershell|cmd
    #[arg(long)]
    pub shell: Option<String>,

    /// validate schema before running command
    #[arg(long)]
    pub schema_check: bool,

    /// send desktop notification on command finish
    #[arg(long)]
    pub notify: bool,

    /// command + args (recommended after --)
    pub command: Vec<String>,
}

#[derive(Args, Debug, Clone)]
/// Launch the Env TUI panel.
pub struct EnvTuiCmd {}
