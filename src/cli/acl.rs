use clap::{Args, Parser, Subcommand};

/// Windows ACL management.
#[derive(Parser, Debug, Clone)]
pub struct AclCmd {
    #[command(subcommand)]
    pub cmd: AclSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum AclSubCommand {
    #[command(name = "show", alias = "view")]
    Show(AclViewCmd),
    Add(AclAddCmd),
    #[command(name = "rm", alias = "remove")]
    Rm(AclRemoveCmd),
    Purge(AclPurgeCmd),
    Diff(AclDiffCmd),
    Batch(AclBatchCmd),
    Effective(AclEffectiveCmd),
    Copy(AclCopyCmd),
    Backup(AclBackupCmd),
    Restore(AclRestoreCmd),
    Inherit(AclInheritCmd),
    Owner(AclOwnerCmd),
    Orphans(AclOrphansCmd),
    Repair(AclRepairCmd),
    Audit(AclAuditCmd),
    Config(AclConfigCmd),
}

/// View ACL summary or detailed entries for a path.
#[derive(Args, Debug, Clone)]
pub struct AclViewCmd {
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

/// Add a permission entry (interactive wizard; use flags for scripted mode).
#[derive(Args, Debug, Clone)]
pub struct AclAddCmd {
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

/// Remove explicit ACE entries (interactive multi-select).
#[derive(Args, Debug, Clone)]
pub struct AclRemoveCmd {
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
#[derive(Args, Debug, Clone)]
pub struct AclPurgeCmd {
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
#[derive(Args, Debug, Clone)]
pub struct AclDiffCmd {
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
#[derive(Args, Debug, Clone)]
pub struct AclBatchCmd {
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
    #[arg(long)]
    pub output: Option<String>,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Show the effective access a user has on a path.
#[derive(Args, Debug, Clone)]
pub struct AclEffectiveCmd {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// user to check (default: current user)
    #[arg(short = 'u', long)]
    pub user: Option<String>,
}

/// Copy the entire ACL from a reference path onto the target.
#[derive(Args, Debug, Clone)]
pub struct AclCopyCmd {
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
#[derive(Args, Debug, Clone)]
pub struct AclBackupCmd {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// output JSON file (auto-named if omitted)
    #[arg(short = 'o', long)]
    pub output: Option<String>,
}

/// Restore an ACL from a previously created JSON backup.
#[derive(Args, Debug, Clone)]
pub struct AclRestoreCmd {
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
#[derive(Args, Debug, Clone)]
pub struct AclInheritCmd {
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
#[derive(Args, Debug, Clone)]
pub struct AclOwnerCmd {
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
#[derive(Args, Debug, Clone)]
pub struct AclOrphansCmd {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// scan recursively (default: true)
    #[arg(long, default_value = "true")]
    pub recursive: String,

    /// action: none | export | delete | both
    #[arg(long, default_value = "none")]
    pub action: String,

    /// output CSV path
    #[arg(long)]
    pub output: Option<String>,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Forced ACL repair: take ownership + grant FullControl (parallel).
#[derive(Args, Debug, Clone)]
pub struct AclRepairCmd {
    /// target path
    #[arg(short = 'p', long)]
    pub path: String,

    /// export error CSV when failures occur
    #[arg(long)]
    pub export_errors: bool,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// clean reset: break inheritance on root, wipe all ACEs, write only
    /// Administrators+SYSTEM FullControl; child objects re-enable inheritance
    /// with no explicit ACEs. Use for user-data directories only.
    #[arg(long)]
    pub reset_clean: bool,

    /// additional principals to grant FullControl after clean reset
    /// (comma-separated, e.g. "DOMAIN\\User,BUILTIN\\Users").
    /// Only used with --reset-clean.
    #[arg(long)]
    pub grant: Option<String>,
}

/// View or export the audit log.
#[derive(Args, Debug, Clone)]
pub struct AclAuditCmd {
    /// show last N entries
    #[arg(long, default_value_t = 30)]
    pub tail: usize,

    /// export CSV
    #[arg(long)]
    pub export: Option<String>,
}

/// View or edit ACL configuration.
#[derive(Args, Debug, Clone)]
pub struct AclConfigCmd {
    /// set a key-value pair: --set KEY VALUE
    #[arg(long)]
    pub set: Vec<String>,
    /// value for `--set KEY VALUE` when KEY consumes the option value
    pub set_value: Vec<String>,
}
