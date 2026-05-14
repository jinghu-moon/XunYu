use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
/// Manage variable annotations.
pub struct EnvAnnotateCmd {
    #[command(subcommand)]
    pub cmd: EnvAnnotateSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum EnvAnnotateSubCommand {
    Set(EnvAnnotateSetCmd),
    List(EnvAnnotateListCmd),
}

#[derive(Args, Debug, Clone)]
/// Set annotation for one variable.
pub struct EnvAnnotateSetCmd {
    /// variable name
    pub name: String,

    /// annotation text
    pub note: String,
}

#[derive(Args, Debug, Clone)]
/// List all annotations.
pub struct EnvAnnotateListCmd {
    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}
