use argh::FromArgs;

/// Restore files from a backup created by `xun bak`.
#[derive(FromArgs)]
#[argh(subcommand, name = "restore")]
pub struct RestoreCmd {
    /// backup name (looked up in backups dir) or direct path to backup dir/zip
    #[argh(positional)]
    pub name_or_path: String,

    /// restore a single file (relative path, e.g. src/main.rs)
    #[argh(option)]
    pub file: Option<String>,

    /// restore files matching glob pattern (e.g. '**/*.ts')
    #[argh(option)]
    pub glob: Option<String>,

    /// restore to this directory instead of the project root
    #[argh(option)]
    pub to: Option<String>,

    /// snapshot current state before restoring (creates a pre_restore backup)
    #[argh(switch)]
    pub snapshot: bool,

    /// project root (default: cwd)
    #[argh(option, short = 'C')]
    pub dir: Option<String>,

    /// dry run: show what would be restored without writing files
    #[argh(switch)]
    pub dry_run: bool,

    /// skip confirmation prompt
    #[argh(switch, short = 'y')]
    pub yes: bool,
}
