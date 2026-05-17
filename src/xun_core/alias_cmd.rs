//! Alias CLI 定义（clap derive）
//!
//! 新架构的 alias 命令定义，替代 argh 版本。
//! 10 个顶层子命令 + 6 个嵌套 app 子命令。

use clap::{Parser, Subcommand};

use super::table_row::TableRow;
use super::value::{ColumnDef, Value, ValueKind};

// ── Alias 主命令 ──────────────────────────────────────────────────

/// Alias management for commands and applications.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "alias",
    about = "Alias management",
    after_help = "EXAMPLES:\n    \
        xun alias setup                   # initialize alias runtime\n    \
        xun alias add gs \"git status\"    # add shell alias\n    \
        xun alias ls                      # list all aliases\n    \
        xun alias rm gs                   # remove an alias\n    \
        xun alias app add code \"C:\\VS Code\\Code.exe\"  # app alias\n    \
        xun alias find git                # search aliases"
)]
pub struct AliasCmd {
    /// alias config file path (default: %APPDATA%/xun/aliases.toml)
    #[arg(long)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub sub: AliasSubCommand,
}

/// Alias 子命令枚举（10 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum AliasSubCommand {
    /// Setup alias runtime (shim template + shells)
    Setup(AliasSetupArgs),
    /// Add shell alias
    Add(AliasAddArgs),
    /// Remove aliases
    Rm(AliasRmArgs),
    /// List aliases
    #[command(name = "list", alias = "ls")]
    List(AliasLsArgs),
    /// Find aliases with fuzzy match
    Find(AliasFindArgs),
    /// Show alias target and shim info
    Which(AliasWhichArgs),
    /// Sync shim + app paths + shells
    Sync(AliasSyncArgs),
    /// Export aliases config
    Export(AliasExportArgs),
    /// Import aliases config
    Import(AliasImportArgs),
    /// App alias operations
    App(AliasAppArgs),
}

// ── 顶层子命令参数 ────────────────────────────────────────────────

/// Setup alias runtime (shim template + shells).
#[derive(Parser, Debug, Clone)]
pub struct AliasSetupArgs {
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
#[derive(Parser, Debug, Clone)]
pub struct AliasAddArgs {
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
#[derive(Parser, Debug, Clone)]
pub struct AliasRmArgs {
    /// alias names
    pub names: Vec<String>,
}

/// List aliases.
#[derive(Parser, Debug, Clone)]
pub struct AliasLsArgs {
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
#[derive(Parser, Debug, Clone)]
pub struct AliasFindArgs {
    /// keyword
    pub keyword: String,
}

/// Show alias target and shim info.
#[derive(Parser, Debug, Clone)]
pub struct AliasWhichArgs {
    /// alias name
    pub name: String,
}

/// Sync shim + app paths + shells.
#[derive(Parser, Debug, Clone)]
pub struct AliasSyncArgs {}

/// Export aliases config.
#[derive(Parser, Debug, Clone)]
pub struct AliasExportArgs {
    /// output file path (stdout when omitted)
    #[arg(short = 'o', long)]
    pub output: Option<String>,
}

/// Import aliases config.
#[derive(Parser, Debug, Clone)]
pub struct AliasImportArgs {
    /// source toml file
    pub file: String,

    /// overwrite conflicts
    #[arg(long)]
    pub force: bool,
}

// ── App 嵌套子命令 ────────────────────────────────────────────────

/// App alias operations.
#[derive(Parser, Debug, Clone)]
pub struct AliasAppArgs {
    #[command(subcommand)]
    pub sub: AliasAppSubCommand,
}

/// App 子命令枚举（6 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum AliasAppSubCommand {
    /// Add application alias
    Add(AliasAppAddArgs),
    /// Remove app aliases
    Rm(AliasAppRmArgs),
    /// List app aliases
    #[command(name = "list", alias = "ls")]
    List(AliasAppLsArgs),
    /// Scan installed applications
    Scan(AliasAppScanArgs),
    /// Show application alias target
    Which(AliasAppWhichArgs),
    /// Sync app aliases only
    Sync(AliasAppSyncArgs),
}

/// Add application alias.
#[derive(Parser, Debug, Clone)]
pub struct AliasAppAddArgs {
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
#[derive(Parser, Debug, Clone)]
pub struct AliasAppRmArgs {
    /// app alias names
    pub names: Vec<String>,
}

/// List app aliases.
#[derive(Parser, Debug, Clone)]
pub struct AliasAppLsArgs {
    /// json output
    #[arg(long)]
    pub json: bool,
}

/// Scan installed applications.
#[derive(Parser, Debug, Clone)]
pub struct AliasAppScanArgs {
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
#[derive(Parser, Debug, Clone)]
pub struct AliasAppWhichArgs {
    /// app alias name
    pub name: String,
}

/// Sync app aliases only.
#[derive(Parser, Debug, Clone)]
pub struct AliasAppSyncArgs {}

// ── 输出类型：AliasEntry ──────────────────────────────────────────

/// Alias 条目。
#[derive(Debug, Clone)]
pub struct AliasEntry {
    pub name: String,
    pub command: String,
    pub mode: String,
    pub desc: String,
    pub tags: String,
}

impl AliasEntry {
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        mode: impl Into<String>,
        desc: impl Into<String>,
        tags: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            mode: mode.into(),
            desc: desc.into(),
            tags: tags.into(),
        }
    }
}

impl TableRow for AliasEntry {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("name", ValueKind::String),
            ColumnDef::new("command", ValueKind::String),
            ColumnDef::new("mode", ValueKind::String),
            ColumnDef::new("desc", ValueKind::String),
            ColumnDef::new("tags", ValueKind::String),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.name.clone()),
            Value::String(self.command.clone()),
            Value::String(self.mode.clone()),
            Value::String(self.desc.clone()),
            Value::String(self.tags.clone()),
        ]
    }
}

// ── CommandSpec 实现 ──────────────────────────────────────────────

#[cfg(feature = "alias")]
mod cmd_spec {
    use super::*;
    use crate::xun_core::command::CommandSpec;
    use crate::xun_core::context::CmdContext;
    use crate::xun_core::error::XunError;
    use crate::xun_core::services::alias as alias_svc;
    use crate::xun_core::value::Value;

/// alias setup
pub struct AliasSetupCmd {
    pub args: AliasSetupArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasSetupCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::setup_alias(
            self.config.as_deref(),
            self.args.no_cmd,
            self.args.no_ps,
            self.args.no_bash,
            self.args.no_nu,
            self.args.core_only,
        )
    }
}

/// alias add
pub struct AliasAddCmd {
    pub args: AliasAddArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasAddCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("alias name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::add_alias(
            self.config.as_deref(),
            &self.args.name,
            &self.args.command,
            &self.args.mode,
            self.args.desc.as_deref(),
            &self.args.tag,
            &self.args.shell,
            self.args.force,
        )
    }
}

/// alias rm
pub struct AliasRmCmd {
    pub args: AliasRmArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasRmCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.names.is_empty() {
            return Err(XunError::user("at least one alias name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::rm_alias(self.config.as_deref(), &self.args.names)
    }
}

/// alias list
pub struct AliasListCmd {
    pub args: AliasLsArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasListCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::list_alias(
            self.config.as_deref(),
            self.args.r#type.as_deref(),
            self.args.tag.as_deref(),
            self.args.json,
        )
    }
}

/// alias find
pub struct AliasFindCmd {
    pub args: AliasFindArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasFindCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::find_alias(self.config.as_deref(), &self.args.keyword)
    }
}

/// alias which
pub struct AliasWhichCmd {
    pub args: AliasWhichArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasWhichCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::which_alias(self.config.as_deref(), &self.args.name)
    }
}

/// alias sync
pub struct AliasSyncCmd {
    pub config: Option<String>,
}

impl CommandSpec for AliasSyncCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::sync_alias(self.config.as_deref())
    }
}

/// alias export
pub struct AliasExportCmd {
    pub args: AliasExportArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasExportCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::export_alias(self.config.as_deref(), self.args.output.as_deref())
    }
}

/// alias import
pub struct AliasImportCmd {
    pub args: AliasImportArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasImportCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.file.is_empty() {
            return Err(XunError::user("source file is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::import_alias(self.config.as_deref(), &self.args.file, self.args.force)
    }
}

// ── App 嵌套子命令 CommandSpec ────────────────────────────────────

/// alias app add
pub struct AliasAppAddCmd {
    pub args: AliasAppAddArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasAppAddCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("app alias name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::app_add(
            self.config.as_deref(),
            &self.args.name,
            &self.args.exe,
            self.args.args.as_deref(),
            self.args.desc.as_deref(),
            &self.args.tag,
            self.args.no_apppaths,
            self.args.force,
        )
    }
}

/// alias app rm
pub struct AliasAppRmCmd {
    pub args: AliasAppRmArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasAppRmCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.names.is_empty() {
            return Err(XunError::user("at least one app alias name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::app_rm(self.config.as_deref(), &self.args.names)
    }
}

/// alias app list
pub struct AliasAppLsCmd {
    pub args: AliasAppLsArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasAppLsCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::app_ls(self.config.as_deref(), self.args.json)
    }
}

/// alias app scan
pub struct AliasAppScanCmd {
    pub args: AliasAppScanArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasAppScanCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::app_scan(
            self.config.as_deref(),
            &self.args.source,
            self.args.filter.as_deref(),
            self.args.json,
            self.args.all,
            self.args.no_cache,
        )
    }
}

/// alias app which
pub struct AliasAppWhichCmd {
    pub args: AliasAppWhichArgs,
    pub config: Option<String>,
}

impl CommandSpec for AliasAppWhichCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::app_which(self.config.as_deref(), &self.args.name)
    }
}

/// alias app sync
pub struct AliasAppSyncCmd {
    pub config: Option<String>,
}

impl CommandSpec for AliasAppSyncCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        alias_svc::app_sync(self.config.as_deref())
    }
}

} // end mod cmd_spec

#[cfg(feature = "alias")]
pub use cmd_spec::*;
