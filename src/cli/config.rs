use clap::{Args, Parser, Subcommand};

/// Manage ~/.xun.config.json.
#[derive(Parser, Debug, Clone)]
pub struct ConfigCmd {
    #[command(subcommand)]
    pub cmd: ConfigSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigSubCommand {
    Get(ConfigGetCmd),
    Set(ConfigSetCmd),
    Edit(ConfigEditCmd),
}

/// Get a config value by dot path (e.g. proxy.defaultUrl).
#[derive(Args, Debug, Clone)]
pub struct ConfigGetCmd {
    /// key path (dot separated)
    pub key: String,
}

/// Set a config value by dot path (e.g. tree.defaultDepth 3).
#[derive(Args, Debug, Clone)]
pub struct ConfigSetCmd {
    /// key path (dot separated)
    pub key: String,

    /// value (JSON if possible, otherwise string)
    pub value: String,
}

/// Open config file in an editor.
#[derive(Args, Debug, Clone)]
pub struct ConfigEditCmd {}
