use clap::{Args, Parser, Subcommand};

/// Alias management for commands and applications.
#[derive(Parser, Debug, Clone)]
pub struct AliasCmd {
    /// alias config file path (default: %APPDATA%/xun/aliases.toml)
    #[arg(long)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub cmd: AliasSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum AliasSubCommand {
    Setup(AliasSetupCmd),
    Add(AliasAddCmd),
    Rm(AliasRmCmd),
    #[command(name = "list", alias = "ls")]
    List(AliasLsCmd),
    Find(AliasFindCmd),
    Which(AliasWhichCmd),
    Sync(AliasSyncCmd),
    Export(AliasExportCmd),
    Import(AliasImportCmd),
    App(AliasAppCmd),
}

/// Setup alias runtime (shim template + shells).
#[derive(Args, Debug, Clone)]
pub struct AliasSetupCmd {
    /// skip cmd backend
    #[arg(long)]
    pub no_cmd: bool,

    /// skip powershell backend
    #[arg(long)]
    pub no_ps: bool,

    /// skip bash backend
    #[arg(long)]
    pub no_bash: bool,

    /// skip nushell backend
    #[arg(long)]
    pub no_nu: bool,

    /// only setup core shells (cmd + powershell)
    #[arg(long)]
    pub core_only: bool,
}

/// Add shell alias.
#[derive(Args, Debug, Clone)]
pub struct AliasAddCmd {
    /// alias name
    pub name: String,

    /// command string
    pub command: String,

    /// alias mode: auto|exe|cmd
    #[arg(long, default_value = "auto")]
    pub mode: String,

    /// description
    #[arg(long)]
    pub desc: Option<String>,

    /// tags (comma-separated, repeatable)
    #[arg(long)]
    pub tag: Vec<String>,

    /// limit to shells: cmd|ps|bash|nu (comma-separated, repeatable)
    #[arg(long)]
    pub shell: Vec<String>,

    /// overwrite existing alias
    #[arg(long)]
    pub force: bool,
}

/// Remove aliases.
#[derive(Args, Debug, Clone)]
pub struct AliasRmCmd {
    /// alias names
    pub names: Vec<String>,
}

/// List aliases.
#[derive(Args, Debug, Clone)]
pub struct AliasLsCmd {
    /// filter: cmd|app
    #[arg(long)]
    pub r#type: Option<String>,

    /// filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// json output
    #[arg(long)]
    pub json: bool,
}

/// Find aliases with fuzzy match.
#[derive(Args, Debug, Clone)]
pub struct AliasFindCmd {
    /// keyword
    pub keyword: String,
}

/// Show alias target and shim info.
#[derive(Args, Debug, Clone)]
pub struct AliasWhichCmd {
    /// alias name
    pub name: String,
}

/// Sync shim + app paths + shells.
#[derive(Args, Debug, Clone)]
pub struct AliasSyncCmd {}

/// Export aliases config.
#[derive(Args, Debug, Clone)]
pub struct AliasExportCmd {
    /// output file path (stdout when omitted)
    #[arg(short = 'o', long)]
    pub output: Option<String>,
}

/// Import aliases config.
#[derive(Args, Debug, Clone)]
pub struct AliasImportCmd {
    /// source toml file
    pub file: String,

    /// overwrite conflicts
    #[arg(long)]
    pub force: bool,
}

/// App alias operations.
#[derive(Parser, Debug, Clone)]
pub struct AliasAppCmd {
    #[command(subcommand)]
    pub cmd: AliasAppSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum AliasAppSubCommand {
    Add(AliasAppAddCmd),
    Rm(AliasAppRmCmd),
    #[command(name = "list", alias = "ls")]
    List(AliasAppLsCmd),
    Scan(AliasAppScanCmd),
    Which(AliasAppWhichCmd),
    Sync(AliasAppSyncCmd),
}

/// Add application alias.
#[derive(Args, Debug, Clone)]
pub struct AliasAppAddCmd {
    /// alias name
    pub name: String,

    /// executable path
    pub exe: String,

    /// fixed args
    #[arg(long)]
    pub args: Option<String>,

    /// description
    #[arg(long)]
    pub desc: Option<String>,

    /// tags
    #[arg(long)]
    pub tag: Vec<String>,

    /// disable app paths registration
    #[arg(long)]
    pub no_apppaths: bool,

    /// overwrite conflicts
    #[arg(long)]
    pub force: bool,
}

/// Remove app aliases.
#[derive(Args, Debug, Clone)]
pub struct AliasAppRmCmd {
    /// app alias names
    pub names: Vec<String>,
}

/// List app aliases.
#[derive(Args, Debug, Clone)]
pub struct AliasAppLsCmd {
    /// json output
    #[arg(long)]
    pub json: bool,
}

/// Scan installed applications.
#[derive(Args, Debug, Clone)]
pub struct AliasAppScanCmd {
    /// source: reg|startmenu|path|all
    #[arg(long, default_value = "all")]
    pub source: String,

    /// keyword filter
    #[arg(long)]
    pub filter: Option<String>,

    /// output json only
    #[arg(long)]
    pub json: bool,

    /// add all scanned entries
    #[arg(long)]
    pub all: bool,

    /// bypass cache
    #[arg(long)]
    pub no_cache: bool,
}

/// Show application alias target.
#[derive(Args, Debug, Clone)]
pub struct AliasAppWhichCmd {
    /// app alias name
    pub name: String,
}

/// Sync app aliases only.
#[derive(Args, Debug, Clone)]
pub struct AliasAppSyncCmd {}
