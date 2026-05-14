use clap::Args;

#[derive(Args, Debug, Clone)]
/// Apply one profile directly.
pub struct EnvApplyCmd {
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
/// Export environment variables.
pub struct EnvExportCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// format: json|env|reg|csv
    #[arg(long)]
    pub format: String,

    /// output path (omit to print stdout)
    #[arg(long)]
    pub out: Option<String>,
}

#[derive(Args, Debug, Clone)]
/// Export environment bundle as zip (json/env/reg/csv).
pub struct EnvExportAllCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// output zip path (default: ./xun-env-<scope>.zip)
    #[arg(long)]
    pub out: Option<String>,
}

#[derive(Args, Debug, Clone)]
/// Export merged and expanded live environment.
pub struct EnvExportLiveCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// format: dotenv|sh|json|reg
    #[arg(long, default_value = "dotenv")]
    pub format: String,

    /// optional env file(s), repeatable
    #[arg(long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[arg(long)]
    pub set: Vec<String>,

    /// output path (omit to print stdout)
    #[arg(long)]
    pub out: Option<String>,
}

#[derive(Args, Debug, Clone)]
/// Print merged and expanded environment as KEY=VALUE list.
pub struct EnvMergedCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,

    /// optional env file(s), repeatable
    #[arg(long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[arg(long)]
    pub set: Vec<String>,
}

#[derive(Args, Debug, Clone)]
/// Import environment variables.
pub struct EnvImportCmd {
    /// input file path (omit when using --stdin)
    pub file: Option<String>,

    /// read import content from stdin
    #[arg(long)]
    pub stdin: bool,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// merge strategy: merge|overwrite
    #[arg(long, default_value = "merge")]
    pub mode: String,

    /// parse and validate only
    #[arg(long)]
    pub dry_run: bool,

    /// skip confirmation for overwrite
    #[arg(short = 'y', long)]
    pub yes: bool,
}
