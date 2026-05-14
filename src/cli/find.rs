use clap::Args;

use super::defaults::default_output_format;

/// Find files and directories by pattern and metadata.
#[derive(Args, Debug, Clone)]
pub struct FindCmd {
    /// base directories (default: cwd)
    pub paths: Vec<String>,

    /// include glob pattern (repeatable or comma separated)
    #[arg(short = 'i', long)]
    pub include: Vec<String>,

    /// exclude glob pattern (repeatable or comma separated)
    #[arg(short = 'e', long)]
    pub exclude: Vec<String>,

    /// include regex pattern (repeatable)
    #[arg(long)]
    pub regex_include: Vec<String>,

    /// exclude regex pattern (repeatable)
    #[arg(long)]
    pub regex_exclude: Vec<String>,

    /// include extensions (comma separated, repeatable)
    #[arg(long)]
    pub extension: Vec<String>,

    /// exclude extensions (comma separated, repeatable)
    #[arg(long)]
    pub not_extension: Vec<String>,

    /// include names (comma separated, repeatable)
    #[arg(long)]
    pub name: Vec<String>,

    /// load rules from file (glob, default exclude)
    #[arg(short = 'F', long)]
    pub filter_file: Option<String>,

    /// size filter (repeatable)
    #[arg(short = 's', long)]
    pub size: Vec<String>,

    /// fuzzy size filter
    #[arg(long)]
    pub fuzzy_size: Option<String>,

    /// mtime filter (repeatable)
    #[arg(long)]
    pub mtime: Vec<String>,

    /// ctime filter (repeatable)
    #[arg(long)]
    pub ctime: Vec<String>,

    /// atime filter (repeatable)
    #[arg(long)]
    pub atime: Vec<String>,

    /// depth filter
    #[arg(short = 'd', long)]
    pub depth: Option<String>,

    /// attribute filter (e.g. +h,-r)
    #[arg(long)]
    pub attribute: Option<String>,

    /// only empty files
    #[arg(long)]
    pub empty_files: bool,

    /// exclude empty files
    #[arg(long)]
    pub not_empty_files: bool,

    /// only empty directories
    #[arg(long)]
    pub empty_dirs: bool,

    /// exclude empty directories
    #[arg(long)]
    pub not_empty_dirs: bool,

    /// case sensitive matching
    #[arg(long)]
    pub case: bool,

    /// count only
    #[arg(short = 'c', long)]
    pub count: bool,

    /// dry run (no filesystem scan)
    #[arg(long)]
    pub dry_run: bool,

    /// test path for dry run
    #[arg(long)]
    pub test_path: Option<String>,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}
