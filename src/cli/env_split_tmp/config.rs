use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
/// Manage env core config.
pub struct EnvConfigCmd {
    #[command(subcommand)]
    pub cmd: EnvConfigSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum EnvConfigSubCommand {
    Show(EnvConfigShowCmd),
    Path(EnvConfigPathCmd),
    Reset(EnvConfigResetCmd),
    Get(EnvConfigGetCmd),
    Set(EnvConfigSetCmd),
}

#[derive(Args, Debug, Clone)]
/// Show current env config.
pub struct EnvConfigShowCmd {
    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Print env config file path.
pub struct EnvConfigPathCmd {}

#[derive(Args, Debug, Clone)]
/// Reset env config to defaults.
pub struct EnvConfigResetCmd {
    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug, Clone)]
/// Get one env config value.
pub struct EnvConfigGetCmd {
    /// key
    pub key: String,
}

#[derive(Args, Debug, Clone)]
/// Set one env config value.
pub struct EnvConfigSetCmd {
    /// key
    pub key: String,

    /// value
    pub value: String,
}
