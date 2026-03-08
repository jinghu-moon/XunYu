use super::*;

pub struct EnvExportCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// format: json|env|reg|csv
    #[argh(option)]
    pub format: String,

    /// output path (omit to print stdout)
    #[argh(option)]
    pub out: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "export-all")]
/// Export environment bundle as zip (json/env/reg/csv).
pub struct EnvExportAllCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// output zip path (default: ./xun-env-<scope>.zip)
    #[argh(option)]
    pub out: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "export-live")]
/// Export merged and expanded live environment.
pub struct EnvExportLiveCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// format: dotenv|sh|json|reg
    #[argh(option, default = "String::from(\"dotenv\")")]
    pub format: String,

    /// optional env file(s), repeatable
    #[argh(option, long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[argh(option)]
    pub set: Vec<String>,

    /// output path (omit to print stdout)
    #[argh(option)]
    pub out: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "env")]
/// Print merged and expanded environment as KEY=VALUE list.
pub struct EnvMergedCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,

    /// optional env file(s), repeatable
    #[argh(option, long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[argh(option)]
    pub set: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "import")]
/// Import environment variables.
pub struct EnvImportCmd {
    /// input file path (omit when using --stdin)
    #[argh(positional)]
    pub file: Option<String>,

    /// read import content from stdin
    #[argh(switch)]
    pub stdin: bool,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// merge strategy: merge|overwrite
    #[argh(option, default = "String::from(\"merge\")")]
    pub mode: String,

    /// parse and validate only
    #[argh(switch)]
    pub dry_run: bool,

    /// skip confirmation for overwrite
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "diff-live")]
/// Diff live environment against snapshot baseline.

