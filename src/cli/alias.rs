use argh::FromArgs;

/// Alias management for commands and applications.
#[derive(FromArgs)]
#[argh(subcommand, name = "alias")]
pub struct AliasCmd {
    /// alias config file path (default: %APPDATA%/xun/aliases.toml)
    #[argh(option)]
    pub config: Option<String>,

    #[argh(subcommand)]
    pub cmd: AliasSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum AliasSubCommand {
    Setup(AliasSetupCmd),
    Add(AliasAddCmd),
    Rm(AliasRmCmd),
    Ls(AliasLsCmd),
    Find(AliasFindCmd),
    Which(AliasWhichCmd),
    Sync(AliasSyncCmd),
    Export(AliasExportCmd),
    Import(AliasImportCmd),
    App(AliasAppCmd),
}

/// Setup alias runtime (shim template + shells).
#[derive(FromArgs)]
#[argh(subcommand, name = "setup")]
pub struct AliasSetupCmd {
    /// skip cmd backend
    #[argh(switch)]
    pub no_cmd: bool,

    /// skip powershell backend
    #[argh(switch)]
    pub no_ps: bool,

    /// skip bash backend
    #[argh(switch)]
    pub no_bash: bool,

    /// skip nushell backend
    #[argh(switch)]
    pub no_nu: bool,

    /// only setup core shells (cmd + powershell)
    #[argh(switch)]
    pub core_only: bool,
}

/// Add shell alias.
#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
pub struct AliasAddCmd {
    /// alias name
    #[argh(positional)]
    pub name: String,

    /// command string
    #[argh(positional)]
    pub command: String,

    /// alias mode: auto|exe|cmd
    #[argh(option, default = "String::from(\"auto\")")]
    pub mode: String,

    /// description
    #[argh(option)]
    pub desc: Option<String>,

    /// tags (comma-separated, repeatable)
    #[argh(option)]
    pub tag: Vec<String>,

    /// limit to shells: cmd|ps|bash|nu (comma-separated, repeatable)
    #[argh(option)]
    pub shell: Vec<String>,

    /// overwrite existing alias
    #[argh(switch)]
    pub force: bool,
}

/// Remove aliases.
#[derive(FromArgs)]
#[argh(subcommand, name = "rm")]
pub struct AliasRmCmd {
    /// alias names
    #[argh(positional)]
    pub names: Vec<String>,
}

/// List aliases.
#[derive(FromArgs)]
#[argh(subcommand, name = "ls")]
pub struct AliasLsCmd {
    /// filter: cmd|app
    #[argh(option)]
    pub r#type: Option<String>,

    /// filter by tag
    #[argh(option)]
    pub tag: Option<String>,

    /// json output
    #[argh(switch)]
    pub json: bool,
}

/// Find aliases with fuzzy match.
#[derive(FromArgs)]
#[argh(subcommand, name = "find")]
pub struct AliasFindCmd {
    /// keyword
    #[argh(positional)]
    pub keyword: String,
}

/// Show alias target and shim info.
#[derive(FromArgs)]
#[argh(subcommand, name = "which")]
pub struct AliasWhichCmd {
    /// alias name
    #[argh(positional)]
    pub name: String,
}

/// Sync shim + app paths + shells.
#[derive(FromArgs)]
#[argh(subcommand, name = "sync")]
pub struct AliasSyncCmd {}

/// Export aliases config.
#[derive(FromArgs)]
#[argh(subcommand, name = "export")]
pub struct AliasExportCmd {
    /// output file path (stdout when omitted)
    #[argh(option, short = 'o')]
    pub output: Option<String>,
}

/// Import aliases config.
#[derive(FromArgs)]
#[argh(subcommand, name = "import")]
pub struct AliasImportCmd {
    /// source toml file
    #[argh(positional)]
    pub file: String,

    /// overwrite conflicts
    #[argh(switch)]
    pub force: bool,
}

/// App alias operations.
#[derive(FromArgs)]
#[argh(subcommand, name = "app")]
pub struct AliasAppCmd {
    #[argh(subcommand)]
    pub cmd: AliasAppSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum AliasAppSubCommand {
    Add(AliasAppAddCmd),
    Rm(AliasAppRmCmd),
    Ls(AliasAppLsCmd),
    Scan(AliasAppScanCmd),
    Which(AliasAppWhichCmd),
    Sync(AliasAppSyncCmd),
}

/// Add application alias.
#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
pub struct AliasAppAddCmd {
    /// alias name
    #[argh(positional)]
    pub name: String,

    /// executable path
    #[argh(positional)]
    pub exe: String,

    /// fixed args
    #[argh(option)]
    pub args: Option<String>,

    /// description
    #[argh(option)]
    pub desc: Option<String>,

    /// tags
    #[argh(option)]
    pub tag: Vec<String>,

    /// disable app paths registration
    #[argh(switch)]
    pub no_apppaths: bool,

    /// overwrite conflicts
    #[argh(switch)]
    pub force: bool,
}

/// Remove app aliases.
#[derive(FromArgs)]
#[argh(subcommand, name = "rm")]
pub struct AliasAppRmCmd {
    /// app alias names
    #[argh(positional)]
    pub names: Vec<String>,
}

/// List app aliases.
#[derive(FromArgs)]
#[argh(subcommand, name = "ls")]
pub struct AliasAppLsCmd {
    /// json output
    #[argh(switch)]
    pub json: bool,
}

/// Scan installed applications.
#[derive(FromArgs)]
#[argh(subcommand, name = "scan")]
pub struct AliasAppScanCmd {
    /// source: reg|startmenu|path|all
    #[argh(option, default = "String::from(\"all\")")]
    pub source: String,

    /// keyword filter
    #[argh(option)]
    pub filter: Option<String>,

    /// output json only
    #[argh(switch)]
    pub json: bool,

    /// add all scanned entries
    #[argh(switch)]
    pub all: bool,

    /// bypass cache
    #[argh(switch)]
    pub no_cache: bool,
}

/// Show application alias target.
#[derive(FromArgs)]
#[argh(subcommand, name = "which")]
pub struct AliasAppWhichCmd {
    /// app alias name
    #[argh(positional)]
    pub name: String,
}

/// Sync app aliases only.
#[derive(FromArgs)]
#[argh(subcommand, name = "sync")]
pub struct AliasAppSyncCmd {}
