// cli/batch_rename.rs
//
// argh parameter definitions for `xun brn` (batch rename).

use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "brn")]
/// Batch file renamer — dry-run by default, --apply to execute
pub struct BrnCmd {
    /// directory to scan (default: current directory)
    #[argh(positional, default = "String::from(\".\")") ]
    pub path: String,

    // ── Rename steps (combinable, applied in fixed order) ────────────────
    /// trim whitespace (or specific chars) from stem ends; use --trim-chars to specify
    #[argh(switch)]
    pub trim: bool,

    /// characters to trim (default: whitespace, requires --trim)
    #[argh(option)]
    pub trim_chars: Option<String>,

    /// strip bracketed content from stem (round/square/curly/all, comma-separated)
    #[argh(option)]
    pub strip_brackets: Option<String>,

    /// remove a prefix from the file stem
    #[argh(option)]
    pub strip_prefix: Option<String>,

    /// remove a suffix from the file stem (before extension)
    #[argh(option)]
    pub strip_suffix: Option<String>,

    /// remove all occurrences of specified characters from the stem
    #[argh(option)]
    pub remove_chars: Option<String>,

    /// literal string to find (use with --to for replacement)
    #[argh(option)]
    pub from: Option<String>,

    /// literal string to replace --from with (default: empty string)
    #[argh(option)]
    pub to: Option<String>,

    /// regex pattern to match against file stems
    #[argh(option)]
    pub regex: Option<String>,

    /// replacement string for --regex (supports $1, $2 capture groups)
    #[argh(option)]
    pub replace: Option<String>,

    /// regex flags: i=case-insensitive, m=multiline (combine: "im")
    #[argh(option)]
    pub regex_flags: Option<String>,

    /// convert naming convention: kebab, snake, pascal, upper, lower
    #[argh(option)]
    pub case: Option<String>,

    /// apply case transformation to extension only: upper, lower
    #[argh(option)]
    pub ext_case: Option<String>,

    /// rename file extension (format: old:new, e.g. jpeg:jpg)
    #[argh(option)]
    pub rename_ext: Option<String>,

    /// add extension to files that have no extension
    #[argh(option)]
    pub add_ext: Option<String>,

    /// prepend a string to the file stem
    #[argh(option)]
    pub prefix: Option<String>,

    /// append a string to the file stem (before extension)
    #[argh(option)]
    pub suffix: Option<String>,

    /// insert text at a character position in the stem (format: pos:text, negative=from end)
    #[argh(option)]
    pub insert_at: Option<String>,

    /// rename using a template (vars: {stem} {ext} {n} {date} {mtime}; e.g. "{n:03}_{stem}")
    #[argh(option)]
    pub template: Option<String>,

    /// template sequence start value (default: 1)
    #[argh(option, default = "1")]
    pub template_start: usize,

    /// template sequence zero-padding width (default: 3)
    #[argh(option, default = "3")]
    pub template_pad: usize,

    /// slice the stem using Python-style indices (format: start:end, e.g. 0:8 or -4:)
    #[argh(option)]
    pub slice: Option<String>,

    /// insert file date into stem (format: prefix|suffix:fmt, e.g. prefix:%Y%m%d; use --ctime for creation time)
    #[argh(option)]
    pub insert_date: Option<String>,

    /// use file creation time instead of modification time (requires --insert-date)
    #[argh(switch)]
    pub ctime: bool,

    /// pad the last numeric group in each stem to a fixed width
    #[argh(option)]
    pub normalize_seq: Option<usize>,

    /// normalize Unicode form of stem: nfc, nfd, nfkc, nfkd
    #[argh(option)]
    pub normalize_unicode: Option<String>,

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

    /// only process files matching this glob pattern (e.g. "IMG_*")
    #[argh(option)]
    pub filter: Option<String>,

    /// exclude files matching this glob pattern
    #[argh(option)]
    pub exclude: Option<String>,

    /// recurse into subdirectories
    #[argh(switch, short = 'r')]
    pub recursive: bool,

    /// maximum recursion depth (default: unlimited; implies --recursive)
    #[argh(option)]
    pub depth: Option<usize>,

    /// also rename directories (default: files only)
    #[argh(switch)]
    pub include_dirs: bool,

    /// sort files by: name (default), mtime, ctime (affects sequence numbering)
    #[argh(option)]
    pub sort_by: Option<String>,

    // ── Output ──────────────────────────────────────────────────────────
    /// output format for preview: table (default), json, csv
    #[argh(option)]
    pub output_format: Option<String>,

    // ── Execution ───────────────────────────────────────────────────────
    /// execute renames (default: dry-run preview)
    #[argh(switch)]
    pub apply: bool,

    /// skip confirmation prompt (requires --apply)
    #[argh(switch, short = 'y')]
    pub yes: bool,

    /// undo the last N rename operations in the target directory (default: 1)
    #[argh(option)]
    pub undo: Option<usize>,

    /// redo the last N undone rename operations in the target directory (default: 1)
    #[argh(option)]
    pub redo: Option<usize>,
}
