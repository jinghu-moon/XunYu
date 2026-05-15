use clap::{Args, Parser, Subcommand};

fn default_output_format() -> String {
    "auto".to_string()
}

fn default_io_format() -> String {
    "json".to_string()
}

/// List all bookmarks.
#[derive(Args, Debug, Clone)]
pub struct ListCmd {
    /// filter by tag
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// sort by: name | last | visits
    #[arg(short = 's', long, default_value = "name")]
    pub sort: String,

    /// limit results
    #[arg(short = 'n', long)]
    pub limit: Option<usize>,

    /// offset results
    #[arg(long)]
    pub offset: Option<usize>,

    /// reverse sort order
    #[arg(long)]
    pub reverse: bool,

    /// output as TSV (Fast Path)
    #[arg(long)]
    pub tsv: bool,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

/// Jump to a bookmark (fuzzy match).
#[derive(Args, Debug, Clone)]
pub struct ZCmd {
    /// fuzzy pattern
    pub patterns: Vec<String>,

    /// filter by tag
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// list matches instead of executing
    #[arg(short = 'l', long)]
    pub list: bool,

    /// show factor scores
    #[arg(short = 's', long)]
    pub score: bool,

    /// explain top-1 result
    #[arg(long)]
    pub why: bool,

    /// preview only; do not execute
    #[arg(long)]
    pub preview: bool,

    /// limit listed results
    #[arg(short = 'n', long)]
    pub limit: Option<usize>,

    /// output json
    #[arg(long)]
    pub json: bool,

    /// output tsv
    #[arg(long)]
    pub tsv: bool,

    /// use global scope
    #[arg(short = 'g', long)]
    pub global: bool,

    /// prefer child scope
    #[arg(short = 'c', long)]
    pub child: bool,

    /// restrict to base dir
    #[arg(long)]
    pub base: Option<String>,

    /// workspace scope
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,

    /// use config preset
    #[arg(long)]
    pub preset: Option<String>,
}

/// Open in Explorer.
#[derive(Args, Debug, Clone)]
pub struct OpenCmd {
    /// fuzzy pattern
    pub patterns: Vec<String>,

    /// filter by tag
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// list matches instead of executing
    #[arg(short = 'l', long)]
    pub list: bool,

    /// show factor scores
    #[arg(short = 's', long)]
    pub score: bool,

    /// explain top-1 result
    #[arg(long)]
    pub why: bool,

    /// preview only; do not execute
    #[arg(long)]
    pub preview: bool,

    /// limit listed results
    #[arg(short = 'n', long)]
    pub limit: Option<usize>,

    /// output json
    #[arg(long)]
    pub json: bool,

    /// output tsv
    #[arg(long)]
    pub tsv: bool,

    /// use global scope
    #[arg(short = 'g', long)]
    pub global: bool,

    /// prefer child scope
    #[arg(short = 'c', long)]
    pub child: bool,

    /// restrict to base dir
    #[arg(long)]
    pub base: Option<String>,

    /// workspace scope
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,

    /// use config preset
    #[arg(long)]
    pub preset: Option<String>,
}

/// Save current directory as bookmark.
#[derive(Args, Debug, Clone)]
pub struct SaveCmd {
    /// bookmark name (optional, defaults to current dir name)
    pub name: Option<String>,

    /// tags (comma separated)
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// description
    #[arg(long)]
    pub desc: Option<String>,

    /// workspace label
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,
}

/// Save current directory or specific path as bookmark.
#[derive(Args, Debug, Clone)]
pub struct SetCmd {
    /// bookmark name
    pub name: String,

    /// path (optional, defaults to current dir)
    pub path: Option<String>,

    /// tags (comma separated)
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// description
    #[arg(long)]
    pub desc: Option<String>,

    /// workspace label
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,
}

/// Force delete files or delete bookmarks with --bookmark (-bm).
#[derive(Args, Debug, Clone)]
pub struct DeleteCmd {
    /// delete bookmark instead of files
    #[arg(long)]
    pub bookmark: bool,

    /// target paths (files or directories)
    pub paths: Vec<String>,

    /// only delete Windows reserved names (default)
    #[arg(long)]
    pub reserved: bool,

    /// allow deleting non-reserved names (dangerous)
    #[arg(long)]
    pub any: bool,

    /// match file names (comma separated, repeatable)
    #[arg(long)]
    pub name: Vec<String>,

    /// exclude directory names (comma separated, repeatable)
    #[arg(short = 'e', long)]
    pub exclude: Vec<String>,

    /// exclude path glob pattern (repeatable)
    #[arg(short = 'p', long)]
    pub pattern: Vec<String>,

    /// skip built-in default excludes
    #[arg(long)]
    pub no_default_excludes: bool,

    /// skip TUI and run CLI pipeline directly
    #[arg(long)]
    pub no_tui: bool,

    /// simulate run without deleting
    #[arg(long)]
    pub dry_run: bool,

    /// alias for --dry-run
    #[arg(long)]
    pub what_if: bool,

    /// collect file info (sha256 + kind) before delete
    #[arg(long)]
    pub collect_info: bool,

    /// write results to CSV log file
    #[arg(long)]
    pub log: Option<String>,

    /// max delete level (1-6), default 2
    #[arg(long, default_value_t = 2)]
    pub level: u8,

    /// schedule delete on reboot (requires admin)
    #[arg(long)]
    pub on_reboot: bool,

    /// skip confirmations
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,

    /// force operation bypass protection
    #[arg(long)]
    pub force: bool,

    /// reason for bypass protection
    #[arg(long)]
    pub reason: Option<String>,
}

/// Clean up dead links.
#[derive(Args, Debug, Clone)]
pub struct GcCmd {
    /// delete all dead links without confirmation
    #[arg(long)]
    pub purge: bool,

    /// preview only; do not delete
    #[arg(long)]
    pub dry_run: bool,

    /// only clean learned/imported records
    #[arg(long)]
    pub learned: bool,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

/// Check bookmark health (missing paths, duplicates, stale).
#[derive(Args, Debug, Clone)]
pub struct CheckCmd {
    /// stale threshold in days
    #[arg(short = 'd', long, default_value_t = 90)]
    pub days: u64,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

/// Update frecency (touch).
#[derive(Args, Debug, Clone)]
pub struct TouchCmd {
    /// bookmark name
    pub name: String,
}

/// Rename a bookmark.
#[derive(Args, Debug, Clone)]
pub struct RenameCmd {
    /// old name
    pub old: String,

    /// new name
    pub new: String,
}

/// Tag management.
#[derive(Parser, Debug, Clone)]
pub struct TagCmd {
    #[command(subcommand)]
    pub cmd: TagSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum TagSubCommand {
    Add(TagAddCmd),
    AddBatch(TagAddBatchCmd),
    Remove(TagRemoveCmd),
    List(TagListCmd),
    Rename(TagRenameCmd),
}

/// Add tags to a bookmark.
#[derive(Args, Debug, Clone)]
pub struct TagAddCmd {
    /// bookmark name
    pub name: String,

    /// tags (comma separated)
    pub tags: String,
}

/// Add tags to multiple bookmarks.
#[derive(Args, Debug, Clone)]
pub struct TagAddBatchCmd {
    /// tags (comma separated)
    pub tags: String,

    /// bookmark names
    pub names: Vec<String>,
}

/// Remove tags from a bookmark.
#[derive(Args, Debug, Clone)]
pub struct TagRemoveCmd {
    /// bookmark name
    pub name: String,

    /// tags (comma separated)
    pub tags: String,
}

/// List all tags and counts.
#[derive(Args, Debug, Clone)]
pub struct TagListCmd {}

/// Rename a tag across all bookmarks.
#[derive(Args, Debug, Clone)]
pub struct TagRenameCmd {
    /// old tag
    pub old: String,

    /// new tag
    pub new: String,
}

/// Show recent bookmarks.
#[derive(Args, Debug, Clone)]
pub struct RecentCmd {
    /// limit results
    #[arg(short = 'n', long, default_value_t = 10)]
    pub limit: usize,

    /// filter by tag
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// filter by workspace
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,

    /// only include records since duration (e.g. 7d, 24h, 30m)
    #[arg(long)]
    pub since: Option<String>,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

/// Show statistics.
#[derive(Args, Debug, Clone)]
pub struct StatsCmd {
    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,

    /// show usage insights and suggestions
    #[arg(long)]
    pub insights: bool,
}

/// Deduplicate bookmarks.
#[derive(Args, Debug, Clone)]
pub struct DedupCmd {
    /// mode: path | name
    #[arg(short = 'm', long, default_value = "path")]
    pub mode: String,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,

    /// skip confirmation (interactive mode only)
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Export bookmarks.
#[derive(Args, Debug, Clone)]
pub struct ExportCmd {
    /// format: json | tsv
    #[arg(short = 'f', long, default_value_t = default_io_format())]
    pub format: String,

    /// output file (optional)
    #[arg(short = 'o', long)]
    pub out: Option<String>,
}

/// Import bookmarks.
#[derive(Args, Debug, Clone)]
pub struct ImportCmd {
    /// format: json | tsv
    #[arg(short = 'f', long, default_value_t = default_io_format())]
    pub format: String,

    /// import source: autojump | zoxide | z | fasd | history
    #[arg(long)]
    pub from: Option<String>,

    /// input file (optional, default stdin)
    #[arg(short = 'i', long)]
    pub input: Option<String>,

    /// mode: merge | overwrite
    #[arg(short = 'm', long, default_value = "merge")]
    pub mode: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// List all keys (for tab completion).
#[derive(Args, Debug, Clone)]
pub struct KeysCmd {}

/// All bookmarks (machine output).
#[derive(Args, Debug, Clone)]
pub struct AllCmd {
    /// filter by tag
    pub tag: Option<String>,
}
