use argh::FromArgs;

/// Generate directory tree.
#[derive(FromArgs)]
#[argh(subcommand, name = "tree")]
pub struct TreeCmd {
    /// target path (default: cwd)
    #[argh(positional)]
    pub path: Option<String>,
    /// max depth, 0=unlimited
    #[argh(option, short = 'd')]
    pub depth: Option<usize>,
    /// output file
    #[argh(option, short = 'o')]
    pub output: Option<String>,
    /// include hidden files
    #[argh(switch)]
    pub hidden: bool,
    /// skip clipboard copy
    #[argh(switch)]
    pub no_clip: bool,

    /// plain output (no box drawing)
    #[argh(switch)]
    pub plain: bool,

    /// stats only (no output lines)
    #[argh(switch)]
    pub stats_only: bool,

    /// fast mode (skip sorting and metadata)
    #[argh(switch)]
    pub fast: bool,

    /// sort by: name | mtime | size
    #[argh(option, default = "String::from(\"name\")")]
    pub sort: String,

    /// show size for each item (directories show total size)
    #[argh(switch)]
    pub size: bool,

    /// max output items
    #[argh(option)]
    pub max_items: Option<usize>,

    /// include pattern (repeatable or comma separated)
    #[argh(option)]
    pub include: Vec<String>,

    /// exclude pattern (repeatable or comma separated)
    #[argh(option)]
    pub exclude: Vec<String>,
}
