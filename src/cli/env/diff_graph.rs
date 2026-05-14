use clap::Args;

#[derive(Args, Debug, Clone)]
/// Diff live environment against snapshot baseline.
pub struct EnvDiffLiveCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// baseline snapshot id (default: latest)
    #[arg(long)]
    pub snapshot: Option<String>,

    /// baseline time, format: RFC3339 or YYYY-MM-DD or YYYY-MM-DD HH:MM:SS
    #[arg(long)]
    pub since: Option<String>,

    /// enable ANSI colors
    #[arg(long)]
    pub color: bool,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Show variable dependency graph (%VAR% references).
pub struct EnvGraphCmd {
    /// root variable name
    pub name: String,

    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// max traversal depth (1-64)
    #[arg(long, default_value_t = 8)]
    pub max_depth: usize,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Validate environment with schema rules.
pub struct EnvValidateCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,

    /// treat warnings as errors
    #[arg(long)]
    pub strict: bool,
}
