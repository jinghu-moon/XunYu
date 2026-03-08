use argh::FromArgs;

/// Manage ~/.xun.config.json.
#[derive(FromArgs)]
#[argh(subcommand, name = "config")]
pub struct ConfigCmd {
    #[argh(subcommand)]
    pub cmd: ConfigSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum ConfigSubCommand {
    Get(ConfigGetCmd),
    Set(ConfigSetCmd),
    Edit(ConfigEditCmd),
}

/// Get a config value by dot path (e.g. proxy.defaultUrl).
#[derive(FromArgs)]
#[argh(subcommand, name = "get")]
pub struct ConfigGetCmd {
    /// key path (dot separated)
    #[argh(positional)]
    pub key: String,
}

/// Set a config value by dot path (e.g. tree.defaultDepth 3).
#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
pub struct ConfigSetCmd {
    /// key path (dot separated)
    #[argh(positional)]
    pub key: String,

    /// value (JSON if possible, otherwise string)
    #[argh(positional)]
    pub value: String,
}

/// Open config file in an editor.
#[derive(FromArgs)]
#[argh(subcommand, name = "edit")]
pub struct ConfigEditCmd {}
