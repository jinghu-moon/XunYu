//! ACL CLI 定义（clap derive）
//!
//! 新架构的 acl 命令定义，替代 argh 版本。
//! 共 16 个子命令。

use clap::{Parser, Subcommand};

use super::table_row::TableRow;
use super::value::{ColumnDef, Value, ValueKind};

// ── ACL 主命令 ──────────────────────────────────────────────────

/// Windows ACL management.
#[derive(Parser, Debug, Clone)]
#[command(name = "acl", about = "Windows ACL management")]
pub struct AclCmd {
    #[command(subcommand)]
    pub sub: AclSubCommand,
}

/// ACL 子命令枚举（16 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum AclSubCommand {
    /// View ACL summary or detailed entries for a path
    #[command(name = "show", alias = "view")]
    Show(AclViewArgs),
    /// Add a permission entry
    Add(AclAddArgs),
    /// Remove explicit ACE entries
    #[command(name = "rm", alias = "remove")]
    Rm(AclRemoveArgs),
    /// Remove ALL explicit rules for a specific principal
    Purge(AclPurgeArgs),
    /// Compare the ACLs of two paths
    Diff(AclDiffArgs),
    /// Process multiple paths from a file or comma-separated list
    Batch(AclBatchArgs),
    /// Show the effective access a user has on a path
    Effective(AclEffectiveArgs),
    /// Copy the entire ACL from a reference path onto the target
    Copy(AclCopyArgs),
    /// Backup the ACL of a path to a JSON file
    Backup(AclBackupArgs),
    /// Restore an ACL from a previously created JSON backup
    Restore(AclRestoreArgs),
    /// Enable or disable DACL inheritance on a path
    Inherit(AclInheritArgs),
    /// Change the owner of a path
    Owner(AclOwnerArgs),
    /// Scan for (and optionally clean up) orphaned SIDs in ACLs
    Orphans(AclOrphansArgs),
    /// Forced ACL repair: take ownership + grant FullControl
    Repair(AclRepairArgs),
    /// View or export the audit log
    Audit(AclAuditArgs),
    /// View or edit ACL configuration
    Config(AclConfigArgs),
}

// ── 子命令参数 ──────────────────────────────────────────────────

/// View ACL summary or detailed entries for a path.
#[derive(Parser, Debug, Clone)]
pub struct AclViewArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// show full detail for each ACE
    #[arg(long)]
    pub detail: bool,

    /// export ACL entries to CSV
    #[arg(long)]
    pub export: Option<String>,
}

/// Add a permission entry.
#[derive(Parser, Debug, Clone)]
pub struct AclAddArgs {
    /// target path (single)
    #[arg(short = 'p', long)]
    pub path: Option<String>,

    /// TXT file with one path per line
    #[arg(long)]
    pub file: Option<String>,

    /// comma-separated path list
    #[arg(long)]
    pub paths: Option<String>,

    /// principal name, e.g. "BUILTIN\\Users"
    #[arg(long)]
    pub principal: Option<String>,

    /// rights level: FullControl | Modify | ReadAndExecute | Read | Write
    #[arg(long)]
    pub rights: Option<String>,

    /// access type: Allow | Deny
    #[arg(long)]
    pub ace_type: Option<String>,

    /// inheritance: BothInherit | ContainerOnly | ObjectOnly | None
    #[arg(long)]
    pub inherit: Option<String>,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Remove explicit ACE entries.
#[derive(Parser, Debug, Clone)]
pub struct AclRemoveArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// principal name to match (non-interactive)
    #[arg(long)]
    pub principal: Option<String>,

    /// raw SID to match (non-interactive)
    #[arg(long)]
    pub raw_sid: Option<String>,

    /// rights level: FullControl | Modify | ReadAndExecute | Read | Write
    #[arg(long)]
    pub rights: Option<String>,

    /// access type: Allow | Deny
    #[arg(long)]
    pub ace_type: Option<String>,

    /// skip confirmation (non-interactive)
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Remove ALL explicit rules for a specific principal.
#[derive(Parser, Debug, Clone)]
pub struct AclPurgeArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// principal to purge (interactive if omitted)
    #[arg(long)]
    pub principal: Option<String>,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Compare the ACLs of two paths.
#[derive(Parser, Debug, Clone)]
pub struct AclDiffArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// reference path
    #[arg(short = 'r', long)]
    pub reference: String,

    /// write diff result to CSV
    #[arg(short = 'o', long)]
    pub output: Option<String>,
}

/// Process multiple paths from a file or comma-separated list.
#[derive(Parser, Debug, Clone)]
pub struct AclBatchArgs {
    /// TXT file with one path per line
    #[arg(long)]
    pub file: Option<String>,

    /// comma-separated path list
    #[arg(long)]
    pub paths: Option<String>,

    /// action: repair | backup | orphans | inherit-reset
    #[arg(long)]
    pub action: String,

    /// output directory for exports
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Show the effective access a user has on a path.
#[derive(Parser, Debug, Clone)]
pub struct AclEffectiveArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// user to check (default: current user)
    #[arg(short = 'u', long)]
    pub user: Option<String>,
}

/// Copy the entire ACL from a reference path onto the target.
#[derive(Parser, Debug, Clone)]
pub struct AclCopyArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// reference path
    #[arg(short = 'r', long)]
    pub reference: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Backup the ACL of a path to a JSON file.
#[derive(Parser, Debug, Clone)]
pub struct AclBackupArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// output JSON file (auto-named if omitted)
    #[arg(short = 'o', long)]
    pub output: Option<String>,
}

/// Restore an ACL from a previously created JSON backup.
#[derive(Parser, Debug, Clone)]
pub struct AclRestoreArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// backup JSON file to read from
    #[arg(long)]
    pub from: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Enable or disable DACL inheritance on a path.
///
/// `preserve` 使用 String 类型，因为 argh 的 `default = "true"` 布尔参数
/// 在 clap 中需要显式传值（`--preserve false`），不能用 bool SetTrue。
#[derive(Parser, Debug, Clone)]
pub struct AclInheritArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// break inheritance
    #[arg(long)]
    pub disable: bool,

    /// restore inheritance
    #[arg(long)]
    pub enable: bool,

    /// when breaking: keep inherited ACEs as explicit copies (default: true)
    #[arg(long, default_value = "true")]
    pub preserve: String,
}

/// Change the owner of a path.
#[derive(Parser, Debug, Clone)]
pub struct AclOwnerArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// new owner principal
    #[arg(long)]
    pub set: Option<String>,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Scan for (and optionally clean up) orphaned SIDs in ACLs.
///
/// `recursive` 使用 String 类型（同 `preserve`）。
#[derive(Parser, Debug, Clone)]
pub struct AclOrphansArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// scan recursively
    #[arg(long, default_value = "true")]
    pub recursive: String,

    /// action: none | export | delete | both
    #[arg(long, default_value = "none")]
    pub action: String,

    /// output CSV path
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Forced ACL repair: take ownership + grant FullControl.
#[derive(Parser, Debug, Clone)]
pub struct AclRepairArgs {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// export error CSV when failures occur
    #[arg(long)]
    pub export_errors: bool,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// clean reset: break inheritance on root, wipe all ACEs
    #[arg(long)]
    pub reset_clean: bool,

    /// additional principals to grant FullControl after clean reset
    #[arg(long)]
    pub grant: Option<String>,
}

/// View or export the audit log.
#[derive(Parser, Debug, Clone)]
pub struct AclAuditArgs {
    /// show last N entries
    #[arg(long, default_value_t = 30)]
    pub tail: usize,

    /// export CSV
    #[arg(long)]
    pub export: Option<String>,
}

/// View or edit ACL configuration.
#[derive(Parser, Debug, Clone)]
pub struct AclConfigArgs {
    /// set a key-value pair: --set KEY VALUE
    #[arg(long, num_args = 2)]
    pub set: Vec<String>,
}

// ── 输出类型：AclEntry ──────────────────────────────────────────

/// ACL 条目。
#[derive(Debug, Clone)]
pub struct AclEntry {
    pub path: String,
    pub principal: String,
    pub rights: String,
    pub ace_type: String,
    pub inherited: bool,
}

impl AclEntry {
    pub fn new(
        path: impl Into<String>,
        principal: impl Into<String>,
        rights: impl Into<String>,
        ace_type: impl Into<String>,
        inherited: bool,
    ) -> Self {
        Self {
            path: path.into(),
            principal: principal.into(),
            rights: rights.into(),
            ace_type: ace_type.into(),
            inherited,
        }
    }
}

impl TableRow for AclEntry {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("path", ValueKind::Path),
            ColumnDef::new("principal", ValueKind::String),
            ColumnDef::new("rights", ValueKind::String),
            ColumnDef::new("ace_type", ValueKind::String),
            ColumnDef::new("inherited", ValueKind::Bool),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.path.clone()),
            Value::String(self.principal.clone()),
            Value::String(self.rights.clone()),
            Value::String(self.ace_type.clone()),
            Value::Bool(self.inherited),
        ]
    }
}
