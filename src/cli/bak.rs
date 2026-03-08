use argh::FromArgs;

/// Incremental project backup.
#[derive(FromArgs)]
#[argh(subcommand, name = "bak")]
pub struct BakCmd {
    /// operation and args: `list` | `restore <name>` (default: create backup)
    #[argh(positional)]
    pub op_args: Vec<String>,

    /// for restore: restore a single file (relative path)
    #[argh(option)]
    pub file: Option<String>,

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

    /// skip prompts
    #[argh(switch, short = 'y')]
    pub yes: bool,
}
