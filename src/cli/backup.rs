use argh::FromArgs;

/// Incremental project backup. Alias: `bak`.
#[derive(FromArgs)]
#[argh(subcommand, name = "backup")]
pub struct BackupCmd {
    #[argh(subcommand)]
    pub cmd: Option<BackupSubCommand>,

    /// backup description
    #[argh(option, short = 'm')]
    pub msg: Option<String>,
    /// working directory (default: cwd)
    #[argh(option, short = 'C')]
    pub dir: Option<String>,

    /// dry run (no copy/zip/cleanup)
    #[argh(switch)]
    pub dry_run: bool,

    /// skip compression for this run
    #[argh(switch)]
    pub no_compress: bool,

    /// override max backups
    #[argh(option)]
    pub retain: Option<usize>,

    /// add include path (repeatable or comma separated)
    #[argh(option)]
    pub include: Vec<String>,

    /// add exclude path (repeatable or comma separated)
    #[argh(option)]
    pub exclude: Vec<String>,

    /// incremental backup: only copy new/modified files
    #[argh(switch)]
    pub incremental: bool,

    /// skip creating a new backup when no changes are detected
    #[argh(switch)]
    pub skip_if_unchanged: bool,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum BackupSubCommand {
    List(BackupListCmd),
    Verify(BackupVerifyCmd),
    Find(BackupFindCmd),
}

/// List available backups.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct BackupListCmd {
    /// output machine-readable JSON
    #[argh(switch)]
    pub json: bool,
}

/// Verify integrity of a directory backup.
#[derive(FromArgs)]
#[argh(subcommand, name = "verify")]
pub struct BackupVerifyCmd {
    /// backup name
    #[argh(positional)]
    pub name: String,

    /// output machine-readable JSON
    #[argh(switch)]
    pub json: bool,
}

/// Find backups by tag or other metadata filters.
#[derive(FromArgs)]
#[argh(subcommand, name = "find")]
pub struct BackupFindCmd {
    /// tag filter
    #[argh(positional)]
    pub tag: Option<String>,

    /// output machine-readable JSON
    #[argh(switch)]
    pub json: bool,
}
