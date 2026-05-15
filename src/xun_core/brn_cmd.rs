//! Batch Rename CLI 定义（clap derive）
//!
//! 新架构的 brn 命令定义，替代 argh 版本。
//! 单命令，30+ 参数。

use clap::Parser;

// ── Brn 主命令 ────────────────────────────────────────────────────

/// Batch file renamer — dry-run by default, --apply to execute.
#[derive(Parser, Debug, Clone)]
#[command(name = "brn", about = "Batch file renamer")]
pub struct BrnCmd {
    /// directory to scan (default: current directory)
    #[arg(default_value = ".")]
    pub path: String,

    // ── Rename steps (combinable, applied in fixed order) ────────────────
    /// trim whitespace (or specific chars) from stem ends
    #[arg(long)]
    pub trim: bool,

    /// characters to trim (default: whitespace, requires --trim)
    #[arg(long)]
    pub trim_chars: Option<String>,

    /// strip bracketed content from stem (round/square/curly/all, comma-separated)
    #[arg(long)]
    pub strip_brackets: Option<String>,

    /// remove a prefix from the file stem
    #[arg(long)]
    pub strip_prefix: Option<String>,

    /// remove a suffix from the file stem (before extension)
    #[arg(long)]
    pub strip_suffix: Option<String>,

    /// remove all occurrences of specified characters from the stem
    #[arg(long)]
    pub remove_chars: Option<String>,

    /// literal string to find (use with --to for replacement)
    #[arg(long)]
    pub from: Option<String>,

    /// literal string to replace --from with (default: empty string)
    #[arg(long)]
    pub to: Option<String>,

    /// regex pattern to match against file stems
    #[arg(long)]
    pub regex: Option<String>,

    /// replacement string for --regex (supports $1, $2 capture groups)
    #[arg(long)]
    pub replace: Option<String>,

    /// regex flags: i=case-insensitive, m=multiline (combine: "im")
    #[arg(long)]
    pub regex_flags: Option<String>,

    /// convert naming convention: kebab, snake, pascal, upper, lower
    #[arg(long)]
    pub case: Option<String>,

    /// apply case transformation to extension only: upper, lower
    #[arg(long)]
    pub ext_case: Option<String>,

    /// rename file extension (format: old:new, e.g. jpeg:jpg)
    #[arg(long)]
    pub rename_ext: Option<String>,

    /// add extension to files that have no extension
    #[arg(long)]
    pub add_ext: Option<String>,

    /// prepend a string to the file stem
    #[arg(long)]
    pub prefix: Option<String>,

    /// append a string to the file stem (before extension)
    #[arg(long)]
    pub suffix: Option<String>,

    /// insert text at a character position in the stem (format: pos:text)
    #[arg(long)]
    pub insert_at: Option<String>,

    /// rename using a template (vars: {stem} {ext} {n} {date} {mtime})
    #[arg(long)]
    pub template: Option<String>,

    /// template sequence start value (default: 1)
    #[arg(long, default_value_t = 1)]
    pub template_start: usize,

    /// template sequence zero-padding width (default: 3)
    #[arg(long, default_value_t = 3)]
    pub template_pad: usize,

    /// slice the stem using Python-style indices (format: start:end)
    #[arg(long)]
    pub slice: Option<String>,

    /// insert file date into stem (format: prefix|suffix:fmt)
    #[arg(long)]
    pub insert_date: Option<String>,

    /// use file creation time instead of modification time
    #[arg(long)]
    pub ctime: bool,

    /// pad the last numeric group in each stem to a fixed width
    #[arg(long)]
    pub normalize_seq: Option<usize>,

    /// normalize Unicode form of stem: nfc, nfd, nfkc, nfkd
    #[arg(long)]
    pub normalize_unicode: Option<String>,

    /// append zero-padded sequence number to each stem
    #[arg(long)]
    pub seq: bool,

    /// sequence start value (default: 1, requires --seq)
    #[arg(long, default_value_t = 1)]
    pub start: usize,

    /// zero-padding width (default: 3, requires --seq)
    #[arg(long, default_value_t = 3)]
    pub pad: usize,

    // ── Filters ─────────────────────────────────────────────────────────
    /// only process files with these extensions (repeatable)
    #[arg(long)]
    pub ext: Vec<String>,

    /// only process files matching this glob pattern
    #[arg(long)]
    pub filter: Option<String>,

    /// exclude files matching this glob pattern
    #[arg(long)]
    pub exclude: Option<String>,

    /// recurse into subdirectories
    #[arg(short = 'r', long)]
    pub recursive: bool,

    /// maximum recursion depth (default: unlimited; implies --recursive)
    #[arg(long)]
    pub depth: Option<usize>,

    /// also rename directories (default: files only)
    #[arg(long)]
    pub include_dirs: bool,

    /// sort files by: name (default), mtime, ctime
    #[arg(long)]
    pub sort_by: Option<String>,

    // ── Output ──────────────────────────────────────────────────────────
    /// output format for preview: table (default), json, csv
    #[arg(long)]
    pub output_format: Option<String>,

    // ── Execution ───────────────────────────────────────────────────────
    /// execute renames (default: dry-run preview)
    #[arg(long)]
    pub apply: bool,

    /// skip confirmation prompt (requires --apply)
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// undo the last N rename operations in the target directory
    #[arg(long)]
    pub undo: Option<usize>,

    /// redo the last N undone rename operations in the target directory
    #[arg(long)]
    pub redo: Option<usize>,
}

// ============================================================
// CommandSpec 实现
// ============================================================

#[cfg(feature = "batch_rename")]
use crate::xun_core::command::CommandSpec;
#[cfg(feature = "batch_rename")]
use crate::xun_core::context::CmdContext;
#[cfg(feature = "batch_rename")]
use crate::xun_core::error::XunError;
#[cfg(feature = "batch_rename")]
use crate::xun_core::value::Value;

/// brn 命令。
#[cfg(feature = "batch_rename")]
pub struct BrnCmdSpec {
    pub args: BrnCmd,
}

#[cfg(feature = "batch_rename")]
impl CommandSpec for BrnCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::batch_rename::cmd_brn(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}
