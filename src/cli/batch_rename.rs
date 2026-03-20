// cli/batch_rename.rs
//
// argh parameter definitions for `xun brn` (batch rename).

use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "brn")]
/// Batch file renamer — dry-run by default, --apply to execute
pub struct BrnCmd {
    /// directory to scan (default: current directory)
    #[argh(positional, default = "String::from(\".\")")]
    pub path: String,

    // ── Rename modes (use exactly one) ──────────────────────────────────
    /// regex pattern to match against file stems
    #[argh(option)]
    pub regex: Option<String>,

    /// replacement string for --regex (supports $1, $2 capture groups)
    #[argh(option)]
    pub replace: Option<String>,

    /// convert naming convention: kebab, snake, pascal, upper, lower
    #[argh(option)]
    pub case: Option<String>,

    /// prepend a string to the file stem
    #[argh(option)]
    pub prefix: Option<String>,

    /// append a string to the file stem (before extension)
    #[argh(option)]
    pub suffix: Option<String>,

    /// remove a prefix from the file stem
    #[argh(option)]
    pub strip_prefix: Option<String>,

    /// append zero-padded sequence number to each stem
    #[argh(switch)]
    pub seq: bool,

    /// sequence start value (default: 1, requires --seq)
    #[argh(option, default = "1")]
    pub start: usize,

    /// zero-padding width (default: 3, requires --seq)
    #[argh(option, default = "3")]
    pub pad: usize,

    // ── Filters ─────────────────────────────────────────────────────────
    /// only process files with these extensions (repeatable)
    #[argh(option)]
    pub ext: Vec<String>,

    /// recurse into subdirectories
    #[argh(switch, short = 'r')]
    pub recursive: bool,

    // ── Execution ───────────────────────────────────────────────────────
    /// execute renames (default: dry-run preview)
    #[argh(switch)]
    pub apply: bool,

    /// skip confirmation prompt (requires --apply)
    #[argh(switch, short = 'y')]
    pub yes: bool,

    /// undo the last rename operation in the target directory
    #[argh(switch)]
    pub undo: bool,
}
