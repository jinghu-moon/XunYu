use argh::FromArgs;

use super::bookmarks::{
    AllCmd, CheckCmd, DedupCmd, ExportCmd, GcCmd, ImportCmd, KeysCmd, ListCmd, OpenCmd,
    RecentCmd, RenameCmd, SaveCmd, SetCmd, StatsCmd, TagCmd, TouchCmd, ZCmd,
};

/// Bookmark management and navigation.
#[derive(FromArgs)]
#[argh(subcommand, name = "bookmark")]
pub struct BookmarkCmd {
    #[argh(subcommand)]
    pub cmd: BookmarkSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum BookmarkSubCommand {
    Z(ZCmd),
    Zi(ZiCmd),
    O(OpenCmd),
    Oi(OiCmd),
    Open(OpenLongCmd),
    Save(SaveCmd),
    Set(SetCmd),
    Delete(BookmarkDeleteCmd),
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
#[derive(FromArgs)]
#[argh(subcommand, name = "zi")]
pub struct ZiCmd {
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

/// Open a bookmark with interactive selection.
#[derive(FromArgs)]
#[argh(subcommand, name = "oi")]
pub struct OiCmd {
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

/// Open in file manager.
#[derive(FromArgs)]
#[argh(subcommand, name = "open")]
pub struct OpenLongCmd {
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

/// Pin a bookmark.
#[derive(FromArgs)]
#[argh(subcommand, name = "pin")]
pub struct PinCmd {
    /// bookmark name
    #[argh(positional)]
    pub name: String,
}

/// Remove pin from a bookmark.
#[derive(FromArgs)]
#[argh(subcommand, name = "unpin")]
pub struct UnpinCmd {
    /// bookmark name
    #[argh(positional)]
    pub name: String,
}

/// Delete a bookmark.
#[derive(FromArgs)]
#[argh(subcommand, name = "delete")]
pub struct BookmarkDeleteCmd {
    /// bookmark name
    #[argh(positional)]
    pub name: String,

    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

/// Undo previous bookmark mutations.
#[derive(FromArgs)]
#[argh(subcommand, name = "undo")]
pub struct UndoCmd {
    /// number of undo steps
    #[argh(option, short = 'n', default = "1")]
    pub steps: usize,
}

/// Redo previously undone bookmark mutations.
#[derive(FromArgs)]
#[argh(subcommand, name = "redo")]
pub struct RedoCmd {
    /// number of redo steps
    #[argh(option, short = 'n', default = "1")]
    pub steps: usize,
}

/// Generate bookmark shell integration.
#[derive(FromArgs)]
#[argh(subcommand, name = "init")]
pub struct BookmarkInitCmd {
    /// shell type: powershell | bash | zsh | fish
    #[argh(positional)]
    pub shell: String,

    /// custom command prefix (e.g. j -> j/ji/jo/joi)
    #[argh(option)]
    pub cmd: Option<String>,
}

/// Record a visited directory for auto-learn.
#[derive(FromArgs)]
#[argh(subcommand, name = "learn")]
pub struct LearnCmd {
    /// path to learn
    #[argh(option)]
    pub path: String,
}
