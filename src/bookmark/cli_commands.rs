use clap::{Args, Parser, Subcommand};

use super::bookmark::{
    AllCmd, CheckCmd, DedupCmd, ExportCmd, GcCmd, ImportCmd, KeysCmd, ListCmd, OpenCmd, RecentCmd,
    RenameCmd, SaveCmd, SetCmd, StatsCmd, TagCmd, TouchCmd, ZCmd,
};

/// Bookmark management and navigation.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkCmd {
    #[command(subcommand)]
    pub cmd: BookmarkSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum BookmarkSubCommand {
    Z(ZCmd),
    Zi(ZiCmd),
    O(OpenCmd),
    Oi(OiCmd),
    Open(OpenLongCmd),
    Save(SaveCmd),
    Set(SetCmd),
    #[command(name = "rm", alias = "delete")]
    Rm(BookmarkDeleteCmd),
    Tag(TagCmd),
    Pin(PinCmd),
    Unpin(UnpinCmd),
    Undo(UndoCmd),
    Redo(RedoCmd),
    Rename(RenameCmd),
    List(ListCmd),
    Recent(RecentCmd),
    Stats(StatsCmd),
    Check(CheckCmd),
    Gc(GcCmd),
    Dedup(DedupCmd),
    Export(ExportCmd),
    Import(ImportCmd),
    Init(BookmarkInitCmd),
    Learn(LearnCmd),
    Touch(TouchCmd),
    Keys(KeysCmd),
    All(AllCmd),
}

/// Jump to a bookmark with interactive selection.
#[derive(Args, Debug, Clone)]
pub struct ZiCmd {
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

/// Open a bookmark with interactive selection.
#[derive(Args, Debug, Clone)]
pub struct OiCmd {
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

/// Open in file manager.
#[derive(Args, Debug, Clone)]
pub struct OpenLongCmd {
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

/// Pin a bookmark.
#[derive(Args, Debug, Clone)]
pub struct PinCmd {
    /// bookmark name
    pub name: String,
}

/// Remove pin from a bookmark.
#[derive(Args, Debug, Clone)]
pub struct UnpinCmd {
    /// bookmark name
    pub name: String,
}

/// Delete a bookmark.
#[derive(Args, Debug, Clone)]
pub struct BookmarkDeleteCmd {
    /// bookmark name
    pub name: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Undo previous bookmark mutations.
#[derive(Args, Debug, Clone)]
pub struct UndoCmd {
    /// number of undo steps
    #[arg(short = 'n', long, default_value_t = 1)]
    pub steps: usize,
}

/// Redo previously undone bookmark mutations.
#[derive(Args, Debug, Clone)]
pub struct RedoCmd {
    /// number of redo steps
    #[arg(short = 'n', long, default_value_t = 1)]
    pub steps: usize,
}

/// Generate bookmark shell integration.
#[derive(Args, Debug, Clone)]
pub struct BookmarkInitCmd {
    /// shell type: powershell | bash | zsh | fish
    pub shell: String,

    /// custom command prefix (e.g. j -> j/ji/jo/joi)
    #[arg(long)]
    pub cmd: Option<String>,
}

/// Record a visited directory for auto-learn.
#[derive(Args, Debug, Clone)]
pub struct LearnCmd {
    /// path to learn
    #[arg(long)]
    pub path: String,
}
