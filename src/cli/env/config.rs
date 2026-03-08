use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "config")]
/// Manage env core config.
pub struct EnvConfigCmd {
    #[argh(subcommand)]
    pub cmd: EnvConfigSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvConfigSubCommand {
    Show(EnvConfigShowCmd),
    Path(EnvConfigPathCmd),
    Reset(EnvConfigResetCmd),
    Get(EnvConfigGetCmd),
    Set(EnvConfigSetCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "show")]
/// Show current env config.
pub struct EnvConfigShowCmd {
    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "path")]
/// Print env config file path.
pub struct EnvConfigPathCmd {}

#[derive(FromArgs)]
#[argh(subcommand, name = "reset")]
/// Reset env config to defaults.
pub struct EnvConfigResetCmd {
    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "get")]
/// Get one env config value.
pub struct EnvConfigGetCmd {
    /// key
    #[argh(positional)]
    pub key: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
/// Set one env config value.
pub struct EnvConfigSetCmd {
    /// key
    #[argh(positional)]
    pub key: String,

    /// value
    #[argh(positional)]
    pub value: String,
}
