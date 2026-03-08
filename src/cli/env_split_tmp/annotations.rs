use super::*;

pub struct EnvAnnotateCmd {
    #[argh(subcommand)]
    pub cmd: EnvAnnotateSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvAnnotateSubCommand {
    Set(EnvAnnotateSetCmd),
    List(EnvAnnotateListCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
/// Set annotation for one variable.
pub struct EnvAnnotateSetCmd {
    /// variable name
    #[argh(positional)]
    pub name: String,

    /// annotation text
    #[argh(positional)]
    pub note: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// List all annotations.
pub struct EnvAnnotateListCmd {
    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "config")]
/// Manage env core config.

