use argh::FromArgs;

use super::defaults::default_output_format;

/// Find files and directories by pattern and metadata.
#[derive(FromArgs)]
#[argh(subcommand, name = "find")]
pub struct FindCmd {
    /// base directories (default: cwd)
    #[argh(positional)]
    pub paths: Vec<String>,

    /// include glob pattern (repeatable or comma separated)
    #[argh(option, short = 'i')]
    pub include: Vec<String>,

    /// exclude glob pattern (repeatable or comma separated)
    #[argh(option, short = 'e')]
    pub exclude: Vec<String>,

    /// include regex pattern (repeatable)
    #[argh(option)]
    pub regex_include: Vec<String>,

    /// exclude regex pattern (repeatable)
    #[argh(option)]
    pub regex_exclude: Vec<String>,

    /// include extensions (comma separated, repeatable)
    #[argh(option)]
    pub extension: Vec<String>,

    /// exclude extensions (comma separated, repeatable)
    #[argh(option)]
    pub not_extension: Vec<String>,

    /// include names (comma separated, repeatable)
    #[argh(option)]
    pub name: Vec<String>,

    /// load rules from file (glob, default exclude)
    #[argh(option, short = 'F')]
    pub filter_file: Option<String>,

    /// size filter (repeatable)
    #[argh(option, short = 's')]
    pub size: Vec<String>,

    /// fuzzy size filter
    #[argh(option)]
    pub fuzzy_size: Option<String>,

    /// mtime filter (repeatable)
    #[argh(option)]
    pub mtime: Vec<String>,

    /// ctime filter (repeatable)
    #[argh(option)]
    pub ctime: Vec<String>,

    /// atime filter (repeatable)
    #[argh(option)]
    pub atime: Vec<String>,

    /// depth filter
    #[argh(option, short = 'd')]
    pub depth: Option<String>,

    /// attribute filter (e.g. +h,-r)
    #[argh(option)]
    pub attribute: Option<String>,

    /// only empty files
    #[argh(switch)]
    pub empty_files: bool,

    /// exclude empty files
    #[argh(switch)]
    pub not_empty_files: bool,

    /// only empty directories
    #[argh(switch)]
    pub empty_dirs: bool,

    /// exclude empty directories
    #[argh(switch)]
    pub not_empty_dirs: bool,

    /// case sensitive matching
    #[argh(switch)]
    pub case: bool,

    /// count only
    #[argh(switch, short = 'c')]
    pub count: bool,

    /// dry run (no filesystem scan)
    #[argh(switch)]
    pub dry_run: bool,

    /// test path for dry run
    #[argh(option)]
    pub test_path: Option<String>,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}
