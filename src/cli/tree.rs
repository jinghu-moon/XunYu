use clap::Args;

/// Generate directory tree.
#[derive(Args, Debug, Clone)]
pub struct TreeCmd {
    /// target path (default: cwd)
    pub path: Option<String>,

    /// max depth, 0=unlimited
    #[arg(short = 'd', long)]
    pub depth: Option<usize>,

    /// output file
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// include hidden files
    #[arg(long)]
    pub hidden: bool,

    /// skip clipboard copy
    #[arg(long)]
    pub no_clip: bool,

    /// plain output (no box drawing)
    #[arg(long)]
    pub plain: bool,

    /// stats only (no output lines)
    #[arg(long)]
    pub stats_only: bool,

    /// fast mode (skip sorting and metadata)
    #[arg(long)]
    pub fast: bool,

    /// sort by: name | mtime | size
    #[arg(long, default_value = "name")]
    pub sort: String,

    /// show size for each item (directories show total size)
    #[arg(long)]
    pub size: bool,

    /// max output items
    #[arg(long)]
    pub max_items: Option<usize>,

    /// include pattern (repeatable or comma separated)
    #[arg(long)]
    pub include: Vec<String>,

    /// exclude pattern (repeatable or comma separated)
    #[arg(long)]
    pub exclude: Vec<String>,
}
