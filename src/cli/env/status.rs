use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "status")]
/// Show env subsystem status overview.
pub struct EnvStatusCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}
