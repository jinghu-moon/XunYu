use super::*;

pub struct EnvStatusCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// List environment variables.

