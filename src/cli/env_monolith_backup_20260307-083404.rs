use argh::FromArgs;

use super::defaults::default_output_format;

#[derive(FromArgs)]
#[argh(subcommand, name = "env")]
/// Environment variable management.
pub struct EnvCmd {
    #[argh(subcommand)]
    pub cmd: EnvSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvSubCommand {
    Status(EnvStatusCmd),
    List(EnvListCmd),
    Search(EnvSearchCmd),
    Get(EnvGetCmd),
    Set(EnvSetCmd),
    Del(EnvDelCmd),
    Check(EnvCheckCmd),
    Path(EnvPathCmd),
    PathDedup(EnvPathDedupCmd),
    Snapshot(EnvSnapshotCmd),
    Doctor(EnvDoctorCmd),
    Profile(EnvProfileCmd),
    Batch(EnvBatchCmd),
    Apply(EnvApplyCmd),
    Export(EnvExportCmd),
    ExportAll(EnvExportAllCmd),
    ExportLive(EnvExportLiveCmd),
    Env(EnvMergedCmd),
    Import(EnvImportCmd),
    DiffLive(EnvDiffLiveCmd),
    Graph(EnvGraphCmd),
    Validate(EnvValidateCmd),
    Schema(EnvSchemaCmd),
    Annotate(EnvAnnotateCmd),
    Config(EnvConfigCmd),
    Audit(EnvAuditCmd),
    Watch(EnvWatchCmd),
    Template(EnvTemplateCmd),
    Run(EnvRunCmd),
    Tui(EnvTuiCmd),
}

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

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// List environment variables.
pub struct EnvListCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "search")]
/// Search environment variables by name/value.
pub struct EnvSearchCmd {
    /// keyword query
    #[argh(positional)]
    pub query: String,

    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "get")]
/// Get one environment variable.
pub struct EnvGetCmd {
    /// variable name
    #[argh(positional)]
    pub name: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
/// Set one environment variable.
pub struct EnvSetCmd {
    /// variable name
    #[argh(positional)]
    pub name: String,

    /// variable value
    #[argh(positional)]
    pub value: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// disable automatic pre-write snapshot
    #[argh(switch)]
    pub no_snapshot: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "del")]
/// Delete one environment variable.
pub struct EnvDelCmd {
    /// variable name
    #[argh(positional)]
    pub name: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "check")]
/// Run environment checks (alias of doctor).
pub struct EnvCheckCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// apply fixes
    #[argh(switch)]
    pub fix: bool,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "path")]
/// PATH operations.
pub struct EnvPathCmd {
    #[argh(subcommand)]
    pub cmd: EnvPathSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "path-dedup")]
/// Deduplicate PATH entries.
pub struct EnvPathDedupCmd {
    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// remove missing directories while deduping
    #[argh(switch)]
    pub remove_missing: bool,

    /// preview only, do not write
    #[argh(switch)]
    pub dry_run: bool,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvPathSubCommand {
    Add(EnvPathAddCmd),
    Rm(EnvPathRmCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
/// Add one PATH entry.
pub struct EnvPathAddCmd {
    /// path entry
    #[argh(positional)]
    pub entry: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// insert at the front
    #[argh(switch)]
    pub head: bool,

    /// insert at the end
    #[argh(switch)]
    pub tail: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "rm")]
/// Remove one PATH entry.
pub struct EnvPathRmCmd {
    /// path entry
    #[argh(positional)]
    pub entry: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "snapshot")]
/// Snapshot operations.
pub struct EnvSnapshotCmd {
    #[argh(subcommand)]
    pub cmd: EnvSnapshotSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvSnapshotSubCommand {
    Create(EnvSnapshotCreateCmd),
    List(EnvSnapshotListCmd),
    Restore(EnvSnapshotRestoreCmd),
    Prune(EnvSnapshotPruneCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "create")]
/// Create a snapshot.
pub struct EnvSnapshotCreateCmd {
    /// snapshot description
    #[argh(option)]
    pub desc: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// List snapshots.
pub struct EnvSnapshotListCmd {
    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "restore")]
/// Restore a snapshot.
pub struct EnvSnapshotRestoreCmd {
    /// snapshot id
    #[argh(option)]
    pub id: Option<String>,

    /// restore latest snapshot
    #[argh(switch)]
    pub latest: bool,

    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "prune")]
/// Prune old snapshots, keep latest N.
pub struct EnvSnapshotPruneCmd {
    /// how many latest snapshots to keep
    #[argh(option, default = "50")]
    pub keep: usize,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "doctor")]
/// Run environment health checks.
pub struct EnvDoctorCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// apply fixes
    #[argh(switch)]
    pub fix: bool,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "profile")]
/// Profile operations.
pub struct EnvProfileCmd {
    #[argh(subcommand)]
    pub cmd: EnvProfileSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvProfileSubCommand {
    List(EnvProfileListCmd),
    Capture(EnvProfileCaptureCmd),
    Apply(EnvProfileApplyCmd),
    Diff(EnvProfileDiffCmd),
    Delete(EnvProfileDeleteCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// List profiles.
pub struct EnvProfileListCmd {
    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "capture")]
/// Capture current scope vars into a profile.
pub struct EnvProfileCaptureCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "apply")]
/// Apply one profile.
pub struct EnvProfileApplyCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,

    /// optional target scope override: user|system
    #[argh(option)]
    pub scope: Option<String>,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "diff")]
/// Diff profile against live scope.
pub struct EnvProfileDiffCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,

    /// optional target scope override: user|system
    #[argh(option)]
    pub scope: Option<String>,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "delete")]
/// Delete one profile.
pub struct EnvProfileDeleteCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "batch")]
/// Batch operations.
pub struct EnvBatchCmd {
    #[argh(subcommand)]
    pub cmd: EnvBatchSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvBatchSubCommand {
    Set(EnvBatchSetCmd),
    Delete(EnvBatchDeleteCmd),
    Rename(EnvBatchRenameCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
/// Batch set KEY=VALUE pairs.
pub struct EnvBatchSetCmd {
    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// preview only, do not write
    #[argh(switch)]
    pub dry_run: bool,

    /// items like KEY=VALUE
    #[argh(positional)]
    pub items: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "delete")]
/// Batch delete names.
pub struct EnvBatchDeleteCmd {
    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// preview only, do not write
    #[argh(switch)]
    pub dry_run: bool,

    /// variable names
    #[argh(positional)]
    pub names: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "rename")]
/// Rename one variable.
pub struct EnvBatchRenameCmd {
    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// preview only, do not write
    #[argh(switch)]
    pub dry_run: bool,

    /// old variable name
    #[argh(positional)]
    pub old: String,

    /// new variable name
    #[argh(positional)]
    pub new: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "apply")]
/// Apply one profile directly.
pub struct EnvApplyCmd {
    /// profile name
    #[argh(positional)]
    pub name: String,

    /// optional target scope override: user|system
    #[argh(option)]
    pub scope: Option<String>,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "export")]
/// Export environment variables.
pub struct EnvExportCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// format: json|env|reg|csv
    #[argh(option)]
    pub format: String,

    /// output path (omit to print stdout)
    #[argh(option)]
    pub out: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "export-all")]
/// Export environment bundle as zip (json/env/reg/csv).
pub struct EnvExportAllCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// output zip path (default: ./xun-env-<scope>.zip)
    #[argh(option)]
    pub out: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "export-live")]
/// Export merged and expanded live environment.
pub struct EnvExportLiveCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// format: dotenv|sh|json|reg
    #[argh(option, default = "String::from(\"dotenv\")")]
    pub format: String,

    /// optional env file(s), repeatable
    #[argh(option, long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[argh(option)]
    pub set: Vec<String>,

    /// output path (omit to print stdout)
    #[argh(option)]
    pub out: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "env")]
/// Print merged and expanded environment as KEY=VALUE list.
pub struct EnvMergedCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,

    /// optional env file(s), repeatable
    #[argh(option, long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[argh(option)]
    pub set: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "import")]
/// Import environment variables.
pub struct EnvImportCmd {
    /// input file path (omit when using --stdin)
    #[argh(positional)]
    pub file: Option<String>,

    /// read import content from stdin
    #[argh(switch)]
    pub stdin: bool,

    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// merge strategy: merge|overwrite
    #[argh(option, default = "String::from(\"merge\")")]
    pub mode: String,

    /// parse and validate only
    #[argh(switch)]
    pub dry_run: bool,

    /// skip confirmation for overwrite
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "diff-live")]
/// Diff live environment against snapshot baseline.
pub struct EnvDiffLiveCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// baseline snapshot id (default: latest)
    #[argh(option)]
    pub snapshot: Option<String>,

    /// baseline time, format: RFC3339 or YYYY-MM-DD or YYYY-MM-DD HH:MM:SS
    #[argh(option)]
    pub since: Option<String>,

    /// enable ANSI colors
    #[argh(switch)]
    pub color: bool,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "graph")]
/// Show variable dependency graph (%VAR% references).
pub struct EnvGraphCmd {
    /// root variable name
    #[argh(positional)]
    pub name: String,

    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// max traversal depth (1-64)
    #[argh(option, default = "8")]
    pub max_depth: usize,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "validate")]
/// Validate environment with schema rules.
pub struct EnvValidateCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,

    /// treat warnings as errors
    #[argh(switch)]
    pub strict: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "schema")]
/// Manage env schema rules.
pub struct EnvSchemaCmd {
    #[argh(subcommand)]
    pub cmd: EnvSchemaSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvSchemaSubCommand {
    Show(EnvSchemaShowCmd),
    AddRequired(EnvSchemaAddRequiredCmd),
    AddRegex(EnvSchemaAddRegexCmd),
    AddEnum(EnvSchemaAddEnumCmd),
    Remove(EnvSchemaRemoveCmd),
    Reset(EnvSchemaResetCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "show")]
/// Show current schema.
pub struct EnvSchemaShowCmd {
    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "add-required")]
/// Add or replace required rule.
pub struct EnvSchemaAddRequiredCmd {
    /// variable pattern, supports * and ?
    #[argh(positional)]
    pub pattern: String,

    /// mark as warning when not strict
    #[argh(switch)]
    pub warn_only: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "add-regex")]
/// Add or replace regex rule.
pub struct EnvSchemaAddRegexCmd {
    /// variable pattern, supports * and ?
    #[argh(positional)]
    pub pattern: String,

    /// regex expression
    #[argh(positional)]
    pub regex: String,

    /// mark as warning when not strict
    #[argh(switch)]
    pub warn_only: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "add-enum")]
/// Add or replace enum rule.
pub struct EnvSchemaAddEnumCmd {
    /// variable pattern, supports * and ?
    #[argh(positional)]
    pub pattern: String,

    /// allowed values, one or more
    #[argh(positional)]
    pub values: Vec<String>,

    /// mark as warning when not strict
    #[argh(switch)]
    pub warn_only: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "remove")]
/// Remove one rule by pattern.
pub struct EnvSchemaRemoveCmd {
    /// rule pattern
    #[argh(positional)]
    pub pattern: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "reset")]
/// Reset schema to empty.
pub struct EnvSchemaResetCmd {
    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "annotate")]
/// Manage variable annotations.
pub struct EnvAnnotateCmd {
    #[argh(subcommand)]
    pub cmd: EnvAnnotateSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvAnnotateSubCommand {
    Set(EnvAnnotateSetCmd),
    List(EnvAnnotateListCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
/// Set annotation for one variable.
pub struct EnvAnnotateSetCmd {
    /// variable name
    #[argh(positional)]
    pub name: String,

    /// annotation text
    #[argh(positional)]
    pub note: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// List all annotations.
pub struct EnvAnnotateListCmd {
    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

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

#[derive(FromArgs)]
#[argh(subcommand, name = "audit")]
/// Show env audit log entries.
pub struct EnvAuditCmd {
    /// max rows, 0 for all
    #[argh(option, default = "50")]
    pub limit: usize,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "watch")]
/// Watch env variable changes by polling.
pub struct EnvWatchCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// poll interval in milliseconds
    #[argh(option, default = "2000")]
    pub interval_ms: u64,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,

    /// run one poll cycle and exit
    #[argh(switch)]
    pub once: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "template")]
/// Expand one %VAR% template string.
pub struct EnvTemplateCmd {
    /// template text, e.g. "Path=%PATH%"
    #[argh(positional)]
    pub input: String,

    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// check references and cycles only
    #[argh(switch)]
    pub validate_only: bool,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "run")]
/// Run command with merged/expanded environment.
pub struct EnvRunCmd {
    /// optional env file(s), repeatable
    #[argh(option, long = "env")]
    pub env_files: Vec<String>,

    /// inline overrides, repeatable KEY=VALUE
    #[argh(option)]
    pub set: Vec<String>,

    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// print exports for shell: bash|powershell|cmd
    #[argh(option)]
    pub shell: Option<String>,

    /// validate schema before running command
    #[argh(switch)]
    pub schema_check: bool,

    /// send desktop notification on command finish
    #[argh(switch)]
    pub notify: bool,

    /// command + args (recommended after --)
    #[argh(positional)]
    pub command: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "tui")]
/// Launch the Env TUI panel.
pub struct EnvTuiCmd {}
