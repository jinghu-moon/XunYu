use argh::FromArgs;

/// Windows ACL management.
#[derive(FromArgs)]
#[argh(subcommand, name = "acl")]
pub struct AclCmd {
    #[argh(subcommand)]
    pub cmd: AclSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum AclSubCommand {
    View(AclViewCmd),
    Add(AclAddCmd),
    Remove(AclRemoveCmd),
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
#[derive(FromArgs)]
#[argh(subcommand, name = "view")]
pub struct AclViewCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// show full detail for each ACE
    #[argh(switch)]
    pub detail: bool,

    /// export ACL entries to CSV
    #[argh(option)]
    pub export: Option<String>,
}

/// Add a permission entry (interactive wizard; use flags for scripted mode).
#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
pub struct AclAddCmd {
    /// target path (single)
    #[argh(option, short = 'p')]
    pub path: Option<String>,

    /// TXT file with one path per line
    #[argh(option)]
    pub file: Option<String>,

    /// comma-separated path list
    #[argh(option)]
    pub paths: Option<String>,

    /// principal name, e.g. "BUILTIN\\Users"
    #[argh(option)]
    pub principal: Option<String>,

    /// rights level: FullControl | Modify | ReadAndExecute | Read | Write
    #[argh(option)]
    pub rights: Option<String>,

    /// access type: Allow | Deny
    #[argh(option)]
    pub ace_type: Option<String>,

    /// inheritance: BothInherit | ContainerOnly | ObjectOnly | None
    #[argh(option)]
    pub inherit: Option<String>,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// Remove explicit ACE entries (interactive multi-select).
#[derive(FromArgs)]
#[argh(subcommand, name = "remove")]
pub struct AclRemoveCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// principal name to match (non-interactive)
    #[argh(option)]
    pub principal: Option<String>,

    /// raw SID to match (non-interactive)
    #[argh(option)]
    pub raw_sid: Option<String>,

    /// rights level: FullControl | Modify | ReadAndExecute | Read | Write
    #[argh(option)]
    pub rights: Option<String>,

    /// access type: Allow | Deny
    #[argh(option)]
    pub ace_type: Option<String>,

    /// skip confirmation (non-interactive)
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// Remove ALL explicit rules for a specific principal.
#[derive(FromArgs)]
#[argh(subcommand, name = "purge")]
pub struct AclPurgeCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// principal to purge (interactive if omitted)
    #[argh(option)]
    pub principal: Option<String>,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// Compare the ACLs of two paths.
#[derive(FromArgs)]
#[argh(subcommand, name = "diff")]
pub struct AclDiffCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// reference path
    #[argh(option, short = 'r')]
    pub reference: String,

    /// write diff result to CSV
    #[argh(option, short = 'o')]
    pub output: Option<String>,
}

/// Process multiple paths from a file or comma-separated list.
#[derive(FromArgs)]
#[argh(subcommand, name = "batch")]
pub struct AclBatchCmd {
    /// TXT file with one path per line
    #[argh(option)]
    pub file: Option<String>,

    /// comma-separated path list
    #[argh(option)]
    pub paths: Option<String>,

    /// action: repair | backup | orphans | inherit-reset
    #[argh(option)]
    pub action: String,

    /// output directory for exports
    #[argh(option)]
    pub output: Option<String>,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// Show the effective access a user has on a path.
#[derive(FromArgs)]
#[argh(subcommand, name = "effective")]
pub struct AclEffectiveCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// user to check (default: current user)
    #[argh(option, short = 'u')]
    pub user: Option<String>,
}

/// Copy the entire ACL from a reference path onto the target.
#[derive(FromArgs)]
#[argh(subcommand, name = "copy")]
pub struct AclCopyCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// reference path
    #[argh(option, short = 'r')]
    pub reference: String,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// Backup the ACL of a path to a JSON file.
#[derive(FromArgs)]
#[argh(subcommand, name = "backup")]
pub struct AclBackupCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// output JSON file (auto-named if omitted)
    #[argh(option, short = 'o')]
    pub output: Option<String>,
}

/// Restore an ACL from a previously created JSON backup.
#[derive(FromArgs)]
#[argh(subcommand, name = "restore")]
pub struct AclRestoreCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// backup JSON file to read from
    #[argh(option)]
    pub from: String,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// Enable or disable DACL inheritance on a path.
#[derive(FromArgs)]
#[argh(subcommand, name = "inherit")]
pub struct AclInheritCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// break inheritance
    #[argh(switch)]
    pub disable: bool,

    /// restore inheritance
    #[argh(switch)]
    pub enable: bool,

    /// when breaking: keep inherited ACEs as explicit copies (default: true)
    #[argh(option, default = "true")]
    pub preserve: bool,
}

/// Change the owner of a path.
#[derive(FromArgs)]
#[argh(subcommand, name = "owner")]
pub struct AclOwnerCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// new owner principal
    #[argh(option)]
    pub set: Option<String>,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// Scan for (and optionally clean up) orphaned SIDs in ACLs.
#[derive(FromArgs)]
#[argh(subcommand, name = "orphans")]
pub struct AclOrphansCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// scan recursively
    #[argh(option, default = "true")]
    pub recursive: bool,

    /// action: none | export | delete | both
    #[argh(option, default = "String::from(\"none\")")]
    pub action: String,

    /// output CSV path
    #[argh(option)]
    pub output: Option<String>,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// Forced ACL repair: take ownership + grant FullControl (parallel).
#[derive(FromArgs)]
#[argh(subcommand, name = "repair")]
pub struct AclRepairCmd {
    /// target path
    #[argh(option, short = 'p')]
    pub path: String,

    /// export error CSV when failures occur
    #[argh(switch)]
    pub export_errors: bool,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,

    /// clean reset: break inheritance on root, wipe all ACEs, write only
    /// Administrators+SYSTEM FullControl; child objects re-enable inheritance
    /// with no explicit ACEs. Use for user-data directories only.
    #[argh(switch)]
    pub reset_clean: bool,

    /// additional principals to grant FullControl after clean reset
    /// (comma-separated, e.g. "DOMAIN\\User,BUILTIN\\Users").
    /// Only used with --reset-clean.
    #[argh(option)]
    pub grant: Option<String>,
}

/// View or export the audit log.
#[derive(FromArgs)]
#[argh(subcommand, name = "audit")]
pub struct AclAuditCmd {
    /// show last N entries
    #[argh(option, default = "30")]
    pub tail: usize,

    /// export CSV
    #[argh(option)]
    pub export: Option<String>,
}

/// View or edit ACL configuration.
#[derive(FromArgs)]
#[argh(subcommand, name = "config")]
pub struct AclConfigCmd {
    /// set a key-value pair: --set KEY VALUE
    #[argh(option)]
    pub set: Vec<String>,
    /// value for `--set KEY VALUE` when KEY consumes the option value
    #[argh(positional)]
    pub set_value: Vec<String>,
}
