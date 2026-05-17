//! Env CLI 定义（clap derive）
//!
//! 新架构的 env 命令定义，替代 argh 版本。
//! 共 27 个顶层子命令，其中 7 个包含嵌套子命令组。

use clap::{Parser, Subcommand};

use super::table_row::TableRow;
use super::value::{ColumnDef, Value, ValueKind};

// ── Env 主命令 ──────────────────────────────────────────────────

/// Environment variable management.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "env",
    about = "Environment variable management",
    after_help = "EXAMPLES:\n    \
        xun env list                   # list user env vars\n    \
        xun env show PATH              # show PATH value\n    \
        xun env set MY_VAR hello       # set a variable\n    \
        xun env rm MY_VAR              # delete a variable\n    \
        xun env search python          # search by keyword\n    \
        xun env path add C:\\tools      # add to PATH\n    \
        xun env snapshot create        # create snapshot\n    \
        xun env doctor --fix           # diagnose and fix issues\n    \
        xun env export -f json -o env.json  # export to JSON"
)]
pub struct EnvCmd {
    #[command(subcommand)]
    pub sub: EnvSubCommand,
}

/// Env 子命令枚举（27 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum EnvSubCommand {
    /// Show env subsystem status overview
    Status(EnvStatusArgs),
    /// List environment variables
    List(EnvListArgs),
    /// Search environment variables by name/value
    Search(EnvSearchArgs),
    /// Get one environment variable
    #[command(name = "show", alias = "get")]
    Show(EnvGetArgs),
    /// Set one environment variable
    Set(EnvSetArgs),
    /// Delete one environment variable
    #[command(name = "rm", alias = "del")]
    Rm(EnvDelArgs),
    /// Run environment checks (alias of doctor)
    Check(EnvCheckArgs),
    /// PATH operations
    Path(EnvPathCmd),
    /// Deduplicate PATH entries
    PathDedup(EnvPathDedupArgs),
    /// Snapshot operations
    Snapshot(EnvSnapshotCmd),
    /// Run environment health checks
    Doctor(EnvDoctorArgs),
    /// Profile operations
    Profile(EnvProfileCmd),
    /// Batch operations
    Batch(EnvBatchCmd),
    /// Apply one profile directly
    Apply(EnvApplyArgs),
    /// Export environment variables
    Export(EnvExportArgs),
    /// Export environment bundle as zip
    ExportAll(EnvExportAllArgs),
    /// Export merged and expanded live environment
    ExportLive(EnvExportLiveArgs),
    /// Print merged and expanded environment as KEY=VALUE list
    Env(EnvMergedArgs),
    /// Import environment variables
    Import(EnvImportArgs),
    /// Diff live environment against snapshot baseline
    DiffLive(EnvDiffLiveArgs),
    /// Show variable dependency graph
    Graph(EnvGraphArgs),
    /// Validate environment with schema rules
    Validate(EnvValidateArgs),
    /// Manage env schema rules
    Schema(EnvSchemaCmd),
    /// Manage variable annotations
    Annotate(EnvAnnotateCmd),
    /// Manage env core config
    Config(EnvConfigCmd),
    /// Show env audit log entries
    Audit(EnvAuditArgs),
    /// Watch env variable changes by polling
    Watch(EnvWatchArgs),
    /// Expand one %VAR% template string
    Template(EnvTemplateArgs),
    /// Run command with merged/expanded environment
    Run(EnvRunArgs),
    /// Launch the Env TUI panel
    Tui(EnvTuiArgs),
}

// ── 独立子命令（无嵌套） ────────────────────────────────────────

/// Show env subsystem status overview.
#[derive(Parser, Debug, Clone)]
pub struct EnvStatusArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

/// List environment variables.
#[derive(Parser, Debug, Clone)]
pub struct EnvListArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// Search environment variables by name/value.
#[derive(Parser, Debug, Clone)]
pub struct EnvSearchArgs {
    /// keyword query
    pub query: String,

    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// Get one environment variable.
#[derive(Parser, Debug, Clone)]
pub struct EnvGetArgs {
    /// variable name
    pub name: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// Set one environment variable.
#[derive(Parser, Debug, Clone)]
pub struct EnvSetArgs {
    /// variable name
    pub name: String,

    /// variable value
    pub value: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// disable automatic pre-write snapshot
    #[arg(long)]
    pub no_snapshot: bool,
}

/// Delete one environment variable.
#[derive(Parser, Debug, Clone)]
pub struct EnvDelArgs {
    /// variable name
    pub name: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Run environment checks (alias of doctor).
#[derive(Parser, Debug, Clone)]
pub struct EnvCheckArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// apply fixes
    #[arg(long)]
    pub fix: bool,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

/// Deduplicate PATH entries.
#[derive(Parser, Debug, Clone)]
pub struct EnvPathDedupArgs {
    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// remove missing directories while deduping
    #[arg(long)]
    pub remove_missing: bool,

    /// preview only, do not write
    #[arg(long)]
    pub dry_run: bool,
}

/// Run environment health checks.
#[derive(Parser, Debug, Clone)]
pub struct EnvDoctorArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// apply fixes
    #[arg(long)]
    pub fix: bool,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

/// Apply one profile directly.
#[derive(Parser, Debug, Clone)]
pub struct EnvApplyArgs {
    /// profile name
    pub name: String,

    /// optional target scope override: user|system
    #[arg(long)]
    pub scope: Option<String>,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Export environment variables.
#[derive(Parser, Debug, Clone)]
pub struct EnvExportArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// format: json|env|reg|csv
    #[arg(short = 'f', long)]
    pub format: String,

    /// output path (omit to print stdout)
    #[arg(short = 'o', long)]
    pub out: Option<String>,
}

/// Export environment bundle as zip.
#[derive(Parser, Debug, Clone)]
pub struct EnvExportAllArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// output zip path
    #[arg(short = 'o', long)]
    pub out: Option<String>,
}

/// Export merged and expanded live environment.
#[derive(Parser, Debug, Clone)]
pub struct EnvExportLiveArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// format: dotenv|sh|json|reg
    #[arg(short = 'f', long, default_value = "dotenv")]
    pub format: String,

    /// optional env file(s), repeatable
    #[arg(long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[arg(long)]
    pub set: Vec<String>,

    /// output path (omit to print stdout)
    #[arg(short = 'o', long)]
    pub out: Option<String>,
}

/// Print merged and expanded environment as KEY=VALUE list.
#[derive(Parser, Debug, Clone)]
pub struct EnvMergedArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,

    /// optional env file(s), repeatable
    #[arg(long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[arg(long)]
    pub set: Vec<String>,
}

/// Import environment variables.
#[derive(Parser, Debug, Clone)]
pub struct EnvImportArgs {
    /// input file path (omit when using --stdin)
    pub file: Option<String>,

    /// read import content from stdin
    #[arg(long)]
    pub stdin: bool,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// merge strategy: merge|overwrite
    #[arg(short = 'm', long, default_value = "merge")]
    pub mode: String,

    /// parse and validate only
    #[arg(long)]
    pub dry_run: bool,

    /// skip confirmation for overwrite
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Diff live environment against snapshot baseline.
#[derive(Parser, Debug, Clone)]
pub struct EnvDiffLiveArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// baseline snapshot id (default: latest)
    #[arg(long)]
    pub snapshot: Option<String>,

    /// baseline time (RFC3339 or YYYY-MM-DD)
    #[arg(long)]
    pub since: Option<String>,

    /// enable ANSI colors
    #[arg(long)]
    pub color: bool,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

/// Show variable dependency graph (%VAR% references).
#[derive(Parser, Debug, Clone)]
pub struct EnvGraphArgs {
    /// root variable name
    pub name: String,

    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// max traversal depth (1..=64)
    #[arg(long, default_value_t = 8)]
    pub max_depth: usize,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

/// Validate environment with schema rules.
#[derive(Parser, Debug, Clone)]
pub struct EnvValidateArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,

    /// treat warnings as errors
    #[arg(long)]
    pub strict: bool,
}

/// Show env audit log entries.
#[derive(Parser, Debug, Clone)]
pub struct EnvAuditArgs {
    /// max rows, 0 for all
    #[arg(long, default_value_t = 50)]
    pub limit: usize,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

/// Watch env variable changes by polling.
#[derive(Parser, Debug, Clone)]
pub struct EnvWatchArgs {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// poll interval in milliseconds (100..60000)
    #[arg(long, default_value_t = 2000, value_parser = clap::value_parser!(u64).range(100..60000))]
    pub interval_ms: u64,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,

    /// run one poll cycle and exit
    #[arg(long)]
    pub once: bool,
}

/// Expand one %VAR% template string.
#[derive(Parser, Debug, Clone)]
pub struct EnvTemplateArgs {
    /// template text, e.g. "Path=%PATH%"
    pub input: String,

    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// check references and cycles only
    #[arg(long)]
    pub validate_only: bool,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

/// Run command with merged/expanded environment.
#[derive(Parser, Debug, Clone)]
pub struct EnvRunArgs {
    /// optional env file(s), repeatable
    #[arg(long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[arg(long)]
    pub set: Vec<String>,

    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// print exports for shell: bash|powershell|cmd
    #[arg(long)]
    pub shell: Option<String>,

    /// validate schema before running command
    #[arg(long)]
    pub schema_check: bool,

    /// send desktop notification on command finish
    #[arg(long)]
    pub notify: bool,

    /// command + args (recommended after --)
    pub command: Vec<String>,
}

/// Launch the Env TUI panel.
#[derive(Parser, Debug, Clone)]
pub struct EnvTuiArgs {}

// ── 嵌套子命令组：path ──────────────────────────────────────────

/// PATH operations.
#[derive(Parser, Debug, Clone)]
pub struct EnvPathCmd {
    #[command(subcommand)]
    pub sub: EnvPathSubCommand,
}

/// PATH 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum EnvPathSubCommand {
    /// Add one PATH entry
    Add(EnvPathAddArgs),
    /// Remove one PATH entry
    Rm(EnvPathRmArgs),
}

/// Add one PATH entry.
#[derive(Parser, Debug, Clone)]
pub struct EnvPathAddArgs {
    /// path entry
    pub entry: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// insert at the front
    #[arg(long, conflicts_with = "tail")]
    pub head: bool,

    /// insert at the end
    #[arg(long, conflicts_with = "head")]
    pub tail: bool,
}

/// Remove one PATH entry.
#[derive(Parser, Debug, Clone)]
pub struct EnvPathRmArgs {
    /// path entry
    pub entry: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,
}

// ── 嵌套子命令组：snapshot ──────────────────────────────────────

/// Snapshot operations.
#[derive(Parser, Debug, Clone)]
pub struct EnvSnapshotCmd {
    #[command(subcommand)]
    pub sub: EnvSnapshotSubCommand,
}

/// Snapshot 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum EnvSnapshotSubCommand {
    /// Create a snapshot
    Create(EnvSnapshotCreateArgs),
    /// List snapshots
    List(EnvSnapshotListArgs),
    /// Restore a snapshot
    Restore(EnvSnapshotRestoreArgs),
    /// Prune old snapshots, keep latest N
    Prune(EnvSnapshotPruneArgs),
}

/// Create a snapshot.
#[derive(Parser, Debug, Clone)]
pub struct EnvSnapshotCreateArgs {
    /// snapshot description
    #[arg(long)]
    pub desc: Option<String>,
}

/// List snapshots.
#[derive(Parser, Debug, Clone)]
pub struct EnvSnapshotListArgs {
    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// Restore a snapshot.
#[derive(Parser, Debug, Clone)]
pub struct EnvSnapshotRestoreArgs {
    /// snapshot id
    #[arg(long)]
    pub id: Option<String>,

    /// restore latest snapshot
    #[arg(long)]
    pub latest: bool,

    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Prune old snapshots, keep latest N.
#[derive(Parser, Debug, Clone)]
pub struct EnvSnapshotPruneArgs {
    /// how many latest snapshots to keep (1..10000)
    #[arg(long, default_value_t = 50)]
    pub keep: usize,
}

// ── 嵌套子命令组：profile ───────────────────────────────────────

/// Profile operations.
#[derive(Parser, Debug, Clone)]
pub struct EnvProfileCmd {
    #[command(subcommand)]
    pub sub: EnvProfileSubCommand,
}

/// Profile 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum EnvProfileSubCommand {
    /// List profiles
    List(EnvProfileListArgs),
    /// Capture current scope vars into a profile
    Capture(EnvProfileCaptureArgs),
    /// Apply one profile
    Apply(EnvProfileApplyArgs),
    /// Diff profile against live scope
    Diff(EnvProfileDiffArgs),
    /// Delete one profile
    #[command(name = "rm", alias = "delete")]
    Rm(EnvProfileDeleteArgs),
}

/// List profiles.
#[derive(Parser, Debug, Clone)]
pub struct EnvProfileListArgs {
    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// Capture current scope vars into a profile.
#[derive(Parser, Debug, Clone)]
pub struct EnvProfileCaptureArgs {
    /// profile name
    pub name: String,

    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,
}

/// Apply one profile.
#[derive(Parser, Debug, Clone)]
pub struct EnvProfileApplyArgs {
    /// profile name
    pub name: String,

    /// optional target scope override: user|system
    #[arg(long)]
    pub scope: Option<String>,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Diff profile against live scope.
#[derive(Parser, Debug, Clone)]
pub struct EnvProfileDiffArgs {
    /// profile name
    pub name: String,

    /// optional target scope override: user|system
    #[arg(long)]
    pub scope: Option<String>,

    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

/// Delete one profile.
#[derive(Parser, Debug, Clone)]
pub struct EnvProfileDeleteArgs {
    /// profile name
    pub name: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

// ── 嵌套子命令组：batch ─────────────────────────────────────────

/// Batch operations.
#[derive(Parser, Debug, Clone)]
pub struct EnvBatchCmd {
    #[command(subcommand)]
    pub sub: EnvBatchSubCommand,
}

/// Batch 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum EnvBatchSubCommand {
    /// Batch set KEY=VALUE pairs
    Set(EnvBatchSetArgs),
    /// Batch delete names
    #[command(name = "rm", alias = "delete")]
    Rm(EnvBatchDeleteArgs),
    /// Rename one variable
    Rename(EnvBatchRenameArgs),
}

/// Batch set KEY=VALUE pairs.
#[derive(Parser, Debug, Clone)]
pub struct EnvBatchSetArgs {
    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// preview only, do not write
    #[arg(long)]
    pub dry_run: bool,

    /// items like KEY=VALUE
    pub items: Vec<String>,
}

/// Batch delete names.
#[derive(Parser, Debug, Clone)]
pub struct EnvBatchDeleteArgs {
    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// preview only, do not write
    #[arg(long)]
    pub dry_run: bool,

    /// variable names
    pub names: Vec<String>,
}

/// Rename one variable.
#[derive(Parser, Debug, Clone)]
pub struct EnvBatchRenameArgs {
    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// preview only, do not write
    #[arg(long)]
    pub dry_run: bool,

    /// old variable name
    pub old: String,

    /// new variable name
    pub new: String,
}

// ── 嵌套子命令组：schema ────────────────────────────────────────

/// Manage env schema rules.
#[derive(Parser, Debug, Clone)]
pub struct EnvSchemaCmd {
    #[command(subcommand)]
    pub sub: EnvSchemaSubCommand,
}

/// Schema 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum EnvSchemaSubCommand {
    /// Show current schema
    Show(EnvSchemaShowArgs),
    /// Add or replace required rule
    AddRequired(EnvSchemaAddRequiredArgs),
    /// Add or replace regex rule
    AddRegex(EnvSchemaAddRegexArgs),
    /// Add or replace enum rule
    AddEnum(EnvSchemaAddEnumArgs),
    /// Remove one rule by pattern
    Remove(EnvSchemaRemoveArgs),
    /// Reset schema to empty
    Reset(EnvSchemaResetArgs),
}

/// Show current schema.
#[derive(Parser, Debug, Clone)]
pub struct EnvSchemaShowArgs {
    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

/// Add or replace required rule.
#[derive(Parser, Debug, Clone)]
pub struct EnvSchemaAddRequiredArgs {
    /// variable pattern, supports * and ?
    pub pattern: String,

    /// mark as warning when not strict
    #[arg(long)]
    pub warn_only: bool,
}

/// Add or replace regex rule.
#[derive(Parser, Debug, Clone)]
pub struct EnvSchemaAddRegexArgs {
    /// variable pattern, supports * and ?
    pub pattern: String,

    /// regex expression
    pub regex: String,

    /// mark as warning when not strict
    #[arg(long)]
    pub warn_only: bool,
}

/// Add or replace enum rule.
#[derive(Parser, Debug, Clone)]
pub struct EnvSchemaAddEnumArgs {
    /// variable pattern, supports * and ?
    pub pattern: String,

    /// allowed values, one or more
    pub values: Vec<String>,

    /// mark as warning when not strict
    #[arg(long)]
    pub warn_only: bool,
}

/// Remove one rule by pattern.
#[derive(Parser, Debug, Clone)]
pub struct EnvSchemaRemoveArgs {
    /// rule pattern
    pub pattern: String,
}

/// Reset schema to empty.
#[derive(Parser, Debug, Clone)]
pub struct EnvSchemaResetArgs {
    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

// ── 嵌套子命令组：annotate ──────────────────────────────────────

/// Manage variable annotations.
#[derive(Parser, Debug, Clone)]
pub struct EnvAnnotateCmd {
    #[command(subcommand)]
    pub sub: EnvAnnotateSubCommand,
}

/// Annotate 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum EnvAnnotateSubCommand {
    /// Set annotation for one variable
    Set(EnvAnnotateSetArgs),
    /// List all annotations
    List(EnvAnnotateListArgs),
}

/// Set annotation for one variable.
#[derive(Parser, Debug, Clone)]
pub struct EnvAnnotateSetArgs {
    /// variable name
    pub name: String,

    /// annotation text
    pub note: String,
}

/// List all annotations.
#[derive(Parser, Debug, Clone)]
pub struct EnvAnnotateListArgs {
    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

// ── 嵌套子命令组：config ────────────────────────────────────────

/// Manage env core config.
#[derive(Parser, Debug, Clone)]
pub struct EnvConfigCmd {
    #[command(subcommand)]
    pub sub: EnvConfigSubCommand,
}

/// Config 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum EnvConfigSubCommand {
    /// Show current env config
    Show(EnvConfigShowArgs),
    /// Print env config file path
    Path(EnvConfigPathArgs),
    /// Reset env config to defaults
    Reset(EnvConfigResetArgs),
    /// Get one env config value
    Get(EnvConfigGetArgs),
    /// Set one env config value
    Set(EnvConfigSetArgs),
}

/// Show current env config.
#[derive(Parser, Debug, Clone)]
pub struct EnvConfigShowArgs {
    /// output format: text|json
    #[arg(short = 'f', long, default_value = "text")]
    pub format: String,
}

/// Print env config file path.
#[derive(Parser, Debug, Clone)]
pub struct EnvConfigPathArgs {}

/// Reset env config to defaults.
#[derive(Parser, Debug, Clone)]
pub struct EnvConfigResetArgs {
    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Get one env config value.
#[derive(Parser, Debug, Clone)]
pub struct EnvConfigGetArgs {
    /// key
    pub key: String,
}

/// Set one env config value.
#[derive(Parser, Debug, Clone)]
pub struct EnvConfigSetArgs {
    /// key
    pub key: String,

    /// value
    pub value: String,
}

// ── 输出类型：EnvVar ────────────────────────────────────────────

/// 环境变量条目。
#[derive(Debug, Clone)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
    pub scope: String,
}

impl EnvVar {
    pub fn new(
        name: impl Into<String>,
        value: impl Into<String>,
        scope: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            scope: scope.into(),
        }
    }
}

impl TableRow for EnvVar {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("name", ValueKind::String),
            ColumnDef::new("value", ValueKind::String),
            ColumnDef::new("scope", ValueKind::String),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.name.clone()),
            Value::String(self.value.clone()),
            Value::String(self.scope.clone()),
        ]
    }
}

// ── 输出类型：EnvSnapshotEntry ──────────────────────────────────

/// 快照条目。
#[derive(Debug, Clone)]
pub struct EnvSnapshotEntry {
    pub id: String,
    pub created: String,
    pub desc: String,
    pub var_count: usize,
}

impl EnvSnapshotEntry {
    pub fn new(
        id: impl Into<String>,
        created: impl Into<String>,
        desc: impl Into<String>,
        var_count: usize,
    ) -> Self {
        Self {
            id: id.into(),
            created: created.into(),
            desc: desc.into(),
            var_count,
        }
    }
}

impl TableRow for EnvSnapshotEntry {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("id", ValueKind::String),
            ColumnDef::new("created", ValueKind::Date),
            ColumnDef::new("desc", ValueKind::String),
            ColumnDef::new("var_count", ValueKind::Int),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.id.clone()),
            Value::String(self.created.clone()),
            Value::String(self.desc.clone()),
            Value::Int(self.var_count as i64),
        ]
    }
}

// ── 输出类型：EnvProfileEntry ───────────────────────────────────

/// Profile 条目。
#[derive(Debug, Clone)]
pub struct EnvProfileEntry {
    pub name: String,
    pub var_count: usize,
    pub created: String,
}

impl EnvProfileEntry {
    pub fn new(
        name: impl Into<String>,
        var_count: usize,
        created: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            var_count,
            created: created.into(),
        }
    }
}

impl TableRow for EnvProfileEntry {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("name", ValueKind::String),
            ColumnDef::new("var_count", ValueKind::Int),
            ColumnDef::new("created", ValueKind::Date),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.name.clone()),
            Value::Int(self.var_count as i64),
            Value::String(self.created.clone()),
        ]
    }
}

// ============================================================
// CommandSpec 实现
// ============================================================

use super::command::CommandSpec;
use super::context::CmdContext;
use super::error::XunError;
use super::services::env as env_svc;
use crate::env_core::types::EnvScope;

fn parse_scope(s: &str) -> Result<EnvScope, XunError> {
    match s {
        "user" => Ok(EnvScope::User),
        "system" => Ok(EnvScope::System),
        "all" => Ok(EnvScope::All),
        _ => Err(XunError::user(format!("invalid scope: {s}, expected user|system|all"))),
    }
}

// ── 独立子命令 ───────────────────────────────────────────────

/// env status — 状态概览。
pub struct EnvStatusCmd {
    pub args: EnvStatusArgs,
}

impl CommandSpec for EnvStatusCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::status_env(scope)
    }
}

/// env list — 列出环境变量。
pub struct EnvListCmd {
    pub args: EnvListArgs,
}

impl CommandSpec for EnvListCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::list_env_vars(scope)
    }
}

/// env search — 搜索环境变量。
pub struct EnvSearchCmd {
    pub args: EnvSearchArgs,
}

impl CommandSpec for EnvSearchCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.query.is_empty() {
            return Err(XunError::user("search query is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::search_env_vars(scope, &self.args.query)
    }
}

/// env show — 获取环境变量。
pub struct EnvShowCmd {
    pub args: EnvGetArgs,
}

impl CommandSpec for EnvShowCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("variable name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::get_env_var(&self.args.name, scope)
    }
}

/// env set — 设置环境变量。
pub struct EnvSetCmd {
    pub args: EnvSetArgs,
}

impl CommandSpec for EnvSetCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("variable name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        let op = env_svc::EnvSetOp::new(&self.args.name, &self.args.value, scope);
        use super::operation::Operation;
        op.execute(_ctx)?;
        Ok(Value::Null)
    }
}

/// env rm — 删除环境变量。
pub struct EnvRmCmd {
    pub args: EnvDelArgs,
}

impl CommandSpec for EnvRmCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("variable name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        let op = env_svc::EnvDelOp::new(&self.args.name, scope);
        use super::operation::Operation;
        op.execute(_ctx)?;
        Ok(Value::Null)
    }
}

/// env check — 运行检查。
pub struct EnvCheckCmd {
    pub args: EnvCheckArgs,
}

impl CommandSpec for EnvCheckCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::doctor_env(scope, self.args.fix)
    }
}

/// env path-dedup — PATH 去重。
pub struct EnvPathDedupCmd {
    pub args: EnvPathDedupArgs,
}

impl CommandSpec for EnvPathDedupCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::path_dedup_env(scope, self.args.remove_missing, self.args.dry_run)
    }
}

/// env doctor — 健康检查。
pub struct EnvDoctorCmd {
    pub args: EnvDoctorArgs,
}

impl CommandSpec for EnvDoctorCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::doctor_env(scope, self.args.fix)
    }
}

/// env apply — 应用 profile。
pub struct EnvApplyCmd {
    pub args: EnvApplyArgs,
}

impl CommandSpec for EnvApplyCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("profile name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = self.args.scope.as_deref().map(parse_scope).transpose()?;
        env_svc::apply_env(&self.args.name, scope)
    }
}

/// env export — 导出环境变量。
pub struct EnvExportCmd {
    pub args: EnvExportArgs,
}

impl CommandSpec for EnvExportCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::export_env(scope, &self.args.format, self.args.out.as_deref())
    }
}

/// env export-all — 导出 bundle。
pub struct EnvExportAllCmd {
    pub args: EnvExportAllArgs,
}

impl CommandSpec for EnvExportAllCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::export_all_env(scope, self.args.out.as_deref())
    }
}

/// env export-live — 导出 live 环境。
pub struct EnvExportLiveCmd {
    pub args: EnvExportLiveArgs,
}

impl CommandSpec for EnvExportLiveCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::export_live_env(
            scope,
            &self.args.format,
            &self.args.env_files,
            &self.args.set,
            self.args.out.as_deref(),
        )
    }
}

/// env env — 合并环境变量列表。
pub struct EnvMergedCmd {
    pub args: EnvMergedArgs,
}

impl CommandSpec for EnvMergedCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::merged_env(scope, &self.args.env_files, &self.args.set)
    }
}

/// env import — 导入环境变量。
pub struct EnvImportCmd {
    pub args: EnvImportArgs,
}

impl CommandSpec for EnvImportCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::import_env(
            self.args.file.as_deref(),
            self.args.stdin,
            scope,
            &self.args.mode,
            self.args.dry_run,
        )
    }
}

/// env diff-live — Diff live 环境。
pub struct EnvDiffLiveCmd {
    pub args: EnvDiffLiveArgs,
}

impl CommandSpec for EnvDiffLiveCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::diff_live_env(
            scope,
            self.args.snapshot.as_deref(),
        )
    }
}

/// env graph — 依赖图。
pub struct EnvGraphCmd {
    pub args: EnvGraphArgs,
}

impl CommandSpec for EnvGraphCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("variable name is required"));
        }
        if self.args.max_depth == 0 || self.args.max_depth > 64 {
            return Err(XunError::user("max_depth must be between 1 and 64"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::graph_env(&self.args.name, scope, self.args.max_depth)
    }
}

/// env validate — 验证 schema。
pub struct EnvValidateCmd {
    pub args: EnvValidateArgs,
}

impl CommandSpec for EnvValidateCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::validate_env(scope, self.args.strict)
    }
}

/// env audit — 审计日志。
pub struct EnvAuditCmd {
    pub args: EnvAuditArgs,
}

impl CommandSpec for EnvAuditCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::audit_env(self.args.limit)
    }
}

/// env watch — 监听变化。
pub struct EnvWatchCmd {
    pub args: EnvWatchArgs,
}

impl CommandSpec for EnvWatchCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        // env watch 是长轮询操作，暂不通过 service 层封装
        Err(XunError::user("env watch is not yet supported in service layer; use `env diff-live` for snapshot comparison"))
    }
}

/// env template — 模板展开。
pub struct EnvTemplateCmd {
    pub args: EnvTemplateArgs,
}

impl CommandSpec for EnvTemplateCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.input.is_empty() {
            return Err(XunError::user("template input is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::template_env(&self.args.input, scope, self.args.validate_only)
    }
}

/// env run — 运行命令。
pub struct EnvRunCmd {
    pub args: EnvRunArgs,
}

impl CommandSpec for EnvRunCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.command.is_empty() {
            return Err(XunError::user("command is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::run_env(
            &self.args.env_files,
            &self.args.set,
            scope,
            self.args.schema_check,
            self.args.notify,
            &self.args.command,
        )
    }
}

/// env tui — TUI 面板。
pub struct EnvTuiCmd {
    pub args: EnvTuiArgs,
}

impl CommandSpec for EnvTuiCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        Err(XunError::user("TUI mode is not yet implemented"))
    }
}

// ── PATH 子命令 ──────────────────────────────────────────────

/// env path add — 添加 PATH 条目。
pub struct EnvPathAddCmd {
    pub args: EnvPathAddArgs,
}

impl CommandSpec for EnvPathAddCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.entry.is_empty() {
            return Err(XunError::user("path entry is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::path_add_env(scope, &self.args.entry, self.args.head)
    }
}

/// env path rm — 删除 PATH 条目。
pub struct EnvPathRmCmd {
    pub args: EnvPathRmArgs,
}

impl CommandSpec for EnvPathRmCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.entry.is_empty() {
            return Err(XunError::user("path entry is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::path_rm_env(scope, &self.args.entry)
    }
}

// ── Snapshot 子命令 ──────────────────────────────────────────

/// env snapshot create — 创建快照。
pub struct EnvSnapshotCreateCmd {
    pub args: EnvSnapshotCreateArgs,
}

impl CommandSpec for EnvSnapshotCreateCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::snapshot_create_env(self.args.desc.as_deref())
    }
}

/// env snapshot list — 列出快照。
pub struct EnvSnapshotListCmd {
    pub args: EnvSnapshotListArgs,
}

impl CommandSpec for EnvSnapshotListCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::snapshot_list_env()
    }
}

/// env snapshot restore — 恢复快照。
pub struct EnvSnapshotRestoreCmd {
    pub args: EnvSnapshotRestoreArgs,
}

impl CommandSpec for EnvSnapshotRestoreCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.id.is_none() && !self.args.latest {
            return Err(XunError::user("either --id or --latest is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::snapshot_restore_env(
            self.args.id.as_deref(),
            self.args.latest,
            scope,
        )
    }
}

/// env snapshot prune — 清理旧快照。
pub struct EnvSnapshotPruneCmd {
    pub args: EnvSnapshotPruneArgs,
}

impl CommandSpec for EnvSnapshotPruneCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::snapshot_prune_env(self.args.keep)
    }
}

// ── Profile 子命令 ───────────────────────────────────────────

/// env profile list — 列出 profiles。
pub struct EnvProfileListCmd {
    pub args: EnvProfileListArgs,
}

impl CommandSpec for EnvProfileListCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::profile_list_env()
    }
}

/// env profile capture — 捕获 profile。
pub struct EnvProfileCaptureCmd {
    pub args: EnvProfileCaptureArgs,
}

impl CommandSpec for EnvProfileCaptureCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("profile name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::profile_capture_env(&self.args.name, scope)
    }
}

/// env profile apply — 应用 profile。
pub struct EnvProfileApplyCmd {
    pub args: EnvProfileApplyArgs,
}

impl CommandSpec for EnvProfileApplyCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("profile name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = self.args.scope.as_deref().map(parse_scope).transpose()?;
        env_svc::profile_apply_env(&self.args.name, scope)
    }
}

/// env profile diff — Profile diff。
pub struct EnvProfileDiffCmd {
    pub args: EnvProfileDiffArgs,
}

impl CommandSpec for EnvProfileDiffCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("profile name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = self.args.scope.as_deref().map(parse_scope).transpose()?;
        env_svc::profile_diff_env(&self.args.name, scope)
    }
}

/// env profile rm — 删除 profile。
pub struct EnvProfileRmCmd {
    pub args: EnvProfileDeleteArgs,
}

impl CommandSpec for EnvProfileRmCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("profile name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::profile_delete_env(&self.args.name)
    }
}

// ── Batch 子命令 ─────────────────────────────────────────────

/// env batch set — 批量设置。
pub struct EnvBatchSetCmd {
    pub args: EnvBatchSetArgs,
}

impl CommandSpec for EnvBatchSetCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.items.is_empty() {
            return Err(XunError::user("at least one KEY=VALUE item is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::batch_set_env(scope, &self.args.items, self.args.dry_run)
    }
}

/// env batch rm — 批量删除。
pub struct EnvBatchRmCmd {
    pub args: EnvBatchDeleteArgs,
}

impl CommandSpec for EnvBatchRmCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.names.is_empty() {
            return Err(XunError::user("at least one variable name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::batch_delete_env(scope, &self.args.names, self.args.dry_run)
    }
}

/// env batch rename — 批量重命名。
pub struct EnvBatchRenameCmd {
    pub args: EnvBatchRenameArgs,
}

impl CommandSpec for EnvBatchRenameCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.old.is_empty() || self.args.new.is_empty() {
            return Err(XunError::user("both old and new names are required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let scope = parse_scope(&self.args.scope)?;
        env_svc::batch_rename_env(scope, &self.args.old, &self.args.new, self.args.dry_run)
    }
}

// ── Schema 子命令 ────────────────────────────────────────────

/// env schema show — 显示 schema。
pub struct EnvSchemaShowCmd {
    pub args: EnvSchemaShowArgs,
}

impl CommandSpec for EnvSchemaShowCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::schema_show_env()
    }
}

/// env schema add-required — 添加 required 规则。
pub struct EnvSchemaAddRequiredCmd {
    pub args: EnvSchemaAddRequiredArgs,
}

impl CommandSpec for EnvSchemaAddRequiredCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.pattern.is_empty() {
            return Err(XunError::user("pattern is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::schema_add_required_env(&self.args.pattern, self.args.warn_only)
    }
}

/// env schema add-regex — 添加 regex 规则。
pub struct EnvSchemaAddRegexCmd {
    pub args: EnvSchemaAddRegexArgs,
}

impl CommandSpec for EnvSchemaAddRegexCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.pattern.is_empty() || self.args.regex.is_empty() {
            return Err(XunError::user("both pattern and regex are required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::schema_add_regex_env(&self.args.pattern, &self.args.regex, self.args.warn_only)
    }
}

/// env schema add-enum — 添加 enum 规则。
pub struct EnvSchemaAddEnumCmd {
    pub args: EnvSchemaAddEnumArgs,
}

impl CommandSpec for EnvSchemaAddEnumCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.pattern.is_empty() {
            return Err(XunError::user("pattern is required"));
        }
        if self.args.values.is_empty() {
            return Err(XunError::user("at least one value is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::schema_add_enum_env(&self.args.pattern, &self.args.values, self.args.warn_only)
    }
}

/// env schema remove — 删除规则。
pub struct EnvSchemaRemoveCmd {
    pub args: EnvSchemaRemoveArgs,
}

impl CommandSpec for EnvSchemaRemoveCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.pattern.is_empty() {
            return Err(XunError::user("pattern is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::schema_remove_env(&self.args.pattern)
    }
}

/// env schema reset — 重置 schema。
pub struct EnvSchemaResetCmd {
    pub args: EnvSchemaResetArgs,
}

impl CommandSpec for EnvSchemaResetCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::schema_reset_env()
    }
}

// ── Annotate 子命令 ──────────────────────────────────────────

/// env annotate set — 设置注解。
pub struct EnvAnnotateSetCmd {
    pub args: EnvAnnotateSetArgs,
}

impl CommandSpec for EnvAnnotateSetCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("variable name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::annotate_set_env(&self.args.name, &self.args.note)
    }
}

/// env annotate list — 列出注解。
pub struct EnvAnnotateListCmd {
    pub args: EnvAnnotateListArgs,
}

impl CommandSpec for EnvAnnotateListCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::annotate_list_env()
    }
}

// ── Config 子命令 ────────────────────────────────────────────

/// env config show — 显示 config。
pub struct EnvConfigShowCmd {
    pub args: EnvConfigShowArgs,
}

impl CommandSpec for EnvConfigShowCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::config_show_env()
    }
}

/// env config path — Config path。
pub struct EnvConfigPathCmd {
    pub args: EnvConfigPathArgs,
}

impl CommandSpec for EnvConfigPathCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::config_path_env()
    }
}

/// env config reset — 重置 config。
pub struct EnvConfigResetCmd {
    pub args: EnvConfigResetArgs,
}

impl CommandSpec for EnvConfigResetCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::config_reset_env()
    }
}

/// env config get — 获取 config 值。
pub struct EnvConfigGetCmd {
    pub args: EnvConfigGetArgs,
}

impl CommandSpec for EnvConfigGetCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.key.is_empty() {
            return Err(XunError::user("config key is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::config_get_env(&self.args.key)
    }
}

/// env config set — 设置 config 值。
pub struct EnvConfigSetCmd {
    pub args: EnvConfigSetArgs,
}

impl CommandSpec for EnvConfigSetCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.key.is_empty() {
            return Err(XunError::user("config key is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        env_svc::config_set_env(&self.args.key, &self.args.value)
    }
}
