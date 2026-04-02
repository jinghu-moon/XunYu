use argh::FromArgs;

use super::defaults::{default_io_format, default_output_format};

/// List all bookmarks.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct ListCmd {
    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// sort by: name | last | visits
    #[argh(option, short = 's', default = "String::from(\"name\")")]
    pub sort: String,

    /// limit results
    #[argh(option, short = 'n')]
    pub limit: Option<usize>,

    /// offset results
    #[argh(option)]
    pub offset: Option<usize>,

    /// reverse sort order
    #[argh(switch)]
    pub reverse: bool,

    /// output as TSV (Fast Path)
    #[argh(switch)]
    pub tsv: bool,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

/// Jump to a bookmark (fuzzy match).
#[derive(FromArgs)]
#[argh(subcommand, name = "z")]
pub struct ZCmd {
    /// fuzzy pattern
    #[argh(positional)]
    pub patterns: Vec<String>,

    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// list matches instead of executing
    #[argh(switch, short = 'l')]
    pub list: bool,

    /// show factor scores
    #[argh(switch, short = 's')]
    pub score: bool,

    /// explain top-1 result
    #[argh(switch)]
    pub why: bool,

    /// preview only; do not execute
    #[argh(switch)]
    pub preview: bool,

    /// limit listed results
    #[argh(option, short = 'n')]
    pub limit: Option<usize>,

    /// output json
    #[argh(switch)]
    pub json: bool,

    /// output tsv
    #[argh(switch)]
    pub tsv: bool,

    /// use global scope
    #[argh(switch, short = 'g')]
    pub global: bool,

    /// prefer child scope
    #[argh(switch, short = 'c')]
    pub child: bool,

    /// restrict to base dir
    #[argh(option)]
    pub base: Option<String>,

    /// workspace scope
    #[argh(option, short = 'w')]
    pub workspace: Option<String>,
}

/// Open in Explorer.
#[derive(FromArgs)]
#[argh(subcommand, name = "o")]
pub struct OpenCmd {
    /// fuzzy pattern
    #[argh(positional)]
    pub patterns: Vec<String>,

    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// list matches instead of executing
    #[argh(switch, short = 'l')]
    pub list: bool,

    /// show factor scores
    #[argh(switch, short = 's')]
    pub score: bool,

    /// explain top-1 result
    #[argh(switch)]
    pub why: bool,

    /// preview only; do not execute
    #[argh(switch)]
    pub preview: bool,

    /// limit listed results
    #[argh(option, short = 'n')]
    pub limit: Option<usize>,

    /// output json
    #[argh(switch)]
    pub json: bool,

    /// output tsv
    #[argh(switch)]
    pub tsv: bool,

    /// use global scope
    #[argh(switch, short = 'g')]
    pub global: bool,

    /// prefer child scope
    #[argh(switch, short = 'c')]
    pub child: bool,

    /// restrict to base dir
    #[argh(option)]
    pub base: Option<String>,

    /// workspace scope
    #[argh(option, short = 'w')]
    pub workspace: Option<String>,
}

/// Save current directory as bookmark.
#[derive(FromArgs)]
#[argh(subcommand, name = "save")]
pub struct SaveCmd {
    /// bookmark name (optional, defaults to current dir name)
    #[argh(positional)]
    pub name: Option<String>,

    /// tags (comma separated)
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// description
    #[argh(option)]
    pub desc: Option<String>,

    /// workspace label
    #[argh(option, short = 'w')]
    pub workspace: Option<String>,
}

/// Save current directory or specific path as bookmark.
#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
pub struct SetCmd {
    /// bookmark name
    #[argh(positional)]
    pub name: String,

    /// path (optional, defaults to current dir)
    #[argh(positional)]
    pub path: Option<String>,

    /// tags (comma separated)
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// description
    #[argh(option)]
    pub desc: Option<String>,

    /// workspace label
    #[argh(option, short = 'w')]
    pub workspace: Option<String>,
}

/// Force delete files or delete bookmarks with --bookmark (-bm).
#[derive(FromArgs)]
#[argh(subcommand, name = "delete")]
pub struct DeleteCmd {
    /// delete bookmark instead of files
    #[argh(switch, long = "bookmark")]
    pub bookmark: bool,

    /// target paths (files or directories)
    #[argh(positional, greedy)]
    pub paths: Vec<String>,

    /// only delete Windows reserved names (default)
    #[argh(switch)]
    pub reserved: bool,

    /// allow deleting non-reserved names (dangerous)
    #[argh(switch)]
    pub any: bool,

    /// match file names (comma separated, repeatable)
    #[argh(option)]
    pub name: Vec<String>,

    /// exclude directory names (comma separated, repeatable)
    #[argh(option, short = 'e')]
    pub exclude: Vec<String>,

    /// exclude path glob pattern (repeatable)
    #[argh(option, short = 'p')]
    pub pattern: Vec<String>,

    /// skip built-in default excludes
    #[argh(switch)]
    pub no_default_excludes: bool,

    /// skip TUI and run CLI pipeline directly
    #[argh(switch)]
    pub no_tui: bool,

    /// simulate run without deleting
    #[argh(switch)]
    pub dry_run: bool,

    /// alias for --dry-run
    #[argh(switch, long = "what-if")]
    pub what_if: bool,

    /// collect file info (sha256 + kind) before delete
    #[argh(switch)]
    pub collect_info: bool,

    /// write results to CSV log file
    #[argh(option)]
    pub log: Option<String>,

    /// max delete level (1-6), default 2
    #[argh(option, default = "2")]
    pub level: u8,

    /// schedule delete on reboot (requires admin)
    #[argh(switch)]
    pub on_reboot: bool,

    /// skip confirmations
    #[argh(switch, short = 'y')]
    pub yes: bool,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,

    /// force operation bypass protection
    #[argh(switch)]
    pub force: bool,

    /// reason for bypass protection
    #[argh(option)]
    pub reason: Option<String>,
}

/// Clean up dead links.
#[derive(FromArgs)]
#[argh(subcommand, name = "gc")]
pub struct GcCmd {
    /// delete all dead links without confirmation
    #[argh(switch)]
    pub purge: bool,

    /// preview only; do not delete
    #[argh(switch)]
    pub dry_run: bool,

    /// only clean learned/imported records
    #[argh(switch)]
    pub learned: bool,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

/// Check bookmark health (missing paths, duplicates, stale).
#[derive(FromArgs)]
#[argh(subcommand, name = "check")]
pub struct CheckCmd {
    /// stale threshold in days
    #[argh(option, short = 'd', default = "90")]
    pub days: u64,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

/// Update frecency (touch).
#[derive(FromArgs)]
#[argh(subcommand, name = "touch")]
pub struct TouchCmd {
    /// bookmark name
    #[argh(positional)]
    pub name: String,
}

/// Rename a bookmark.
#[derive(FromArgs)]
#[argh(subcommand, name = "rename")]
pub struct RenameCmd {
    /// old name
    #[argh(positional)]
    pub old: String,

    /// new name
    #[argh(positional)]
    pub new: String,
}

/// Tag management.
#[derive(FromArgs)]
#[argh(subcommand, name = "tag")]
pub struct TagCmd {
    #[argh(subcommand)]
    pub cmd: TagSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum TagSubCommand {
    Add(TagAddCmd),
    Remove(TagRemoveCmd),
    List(TagListCmd),
    Rename(TagRenameCmd),
}

/// Add tags to a bookmark.
#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
pub struct TagAddCmd {
    /// bookmark name
    #[argh(positional)]
    pub name: String,

    /// tags (comma separated)
    #[argh(positional)]
    pub tags: String,
}

/// Remove tags from a bookmark.
#[derive(FromArgs)]
#[argh(subcommand, name = "remove")]
pub struct TagRemoveCmd {
    /// bookmark name
    #[argh(positional)]
    pub name: String,

    /// tags (comma separated)
    #[argh(positional)]
    pub tags: String,
}

/// List all tags and counts.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct TagListCmd {}

/// Rename a tag across all bookmarks.
#[derive(FromArgs)]
#[argh(subcommand, name = "rename")]
pub struct TagRenameCmd {
    /// old tag
    #[argh(positional)]
    pub old: String,

    /// new tag
    #[argh(positional)]
    pub new: String,
}

/// Show recent bookmarks.
#[derive(FromArgs)]
#[argh(subcommand, name = "recent")]
pub struct RecentCmd {
    /// limit results
    #[argh(option, short = 'n', default = "10")]
    pub limit: usize,

    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// filter by workspace
    #[argh(option, short = 'w')]
    pub workspace: Option<String>,

    /// only include records since duration (e.g. 7d, 24h, 30m)
    #[argh(option)]
    pub since: Option<String>,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

/// Show statistics.
#[derive(FromArgs)]
#[argh(subcommand, name = "stats")]
pub struct StatsCmd {
    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

/// Deduplicate bookmarks.
#[derive(FromArgs)]
#[argh(subcommand, name = "dedup")]
pub struct DedupCmd {
    /// mode: path | name
    #[argh(option, short = 'm', default = "String::from(\"path\")")]
    pub mode: String,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,

    /// skip confirmation (interactive mode only)
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// Export bookmarks.
#[derive(FromArgs)]
#[argh(subcommand, name = "export")]
pub struct ExportCmd {
    /// format: json | tsv
    #[argh(option, short = 'f', default = "default_io_format()")]
    pub format: String,

    /// output file (optional)
    #[argh(option, short = 'o')]
    pub out: Option<String>,
}

/// Import bookmarks.
#[derive(FromArgs)]
#[argh(subcommand, name = "import")]
pub struct ImportCmd {
    /// format: json | tsv
    #[argh(option, short = 'f', default = "default_io_format()")]
    pub format: String,

    /// import source: autojump | zoxide | z | fasd | history
    #[argh(option)]
    pub from: Option<String>,

    /// input file (optional, default stdin)
    #[argh(option, short = 'i')]
    pub input: Option<String>,

    /// mode: merge | overwrite
    #[argh(option, short = 'm', default = "String::from(\"merge\")")]
    pub mode: String,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// List all keys (for tab completion).
#[derive(FromArgs)]
#[argh(subcommand, name = "keys")]
pub struct KeysCmd {}

/// All bookmarks (machine output).
#[derive(FromArgs)]
#[argh(subcommand, name = "all")]
pub struct AllCmd {
    /// filter by tag
    #[argh(positional)]
    pub tag: Option<String>,
}
