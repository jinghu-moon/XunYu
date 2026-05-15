//! Redirect CLI 定义（clap derive）
//!
//! 新架构的 redirect 命令定义，替代 argh 版本。
//! RedirectCmd 单命令，20+ 参数。

use clap::Parser;

// ── Redirect 命令 ────────────────────────────────────────────────

/// Redirect files in a directory into categorized subfolders.
#[derive(Parser, Debug, Clone)]
#[command(name = "redirect", about = "Redirect files into categorized subfolders")]
pub struct RedirectCmd {
    /// source directory (default: current directory)
    pub source: Option<String>,

    /// profile name under config.redirect.profiles (default: "default")
    #[arg(long, default_value = "default")]
    pub profile: String,

    /// explain why a file would match or not match rules (pure string mode)
    #[arg(long)]
    pub explain: Option<String>,

    /// show rules coverage summary after a run (printed to stderr)
    #[arg(long)]
    pub stats: bool,

    /// show preview summary and require confirmation before executing
    #[arg(long)]
    pub confirm: bool,

    /// review each planned file action interactively (y/n/a/q)
    #[arg(long)]
    pub review: bool,

    /// query audit log (redirect tx history)
    #[arg(long)]
    pub log: bool,

    /// filter audit log by tx id (use with --log)
    #[arg(long)]
    pub tx: Option<String>,

    /// show last N tx summaries (use with --log)
    #[arg(long)]
    pub last: Option<usize>,

    /// validate config only (no scan/no watch)
    #[arg(long)]
    pub validate: bool,

    /// write a plan file instead of executing (json)
    #[arg(long)]
    pub plan: Option<String>,

    /// apply a previously generated plan file (json)
    #[arg(long)]
    pub apply: Option<String>,

    /// undo a previous redirect by tx id (read from audit.jsonl)
    #[arg(long)]
    pub undo: Option<String>,

    /// watch mode (daemon: continuously apply redirect rules)
    #[arg(long)]
    pub watch: bool,

    /// show watch status instead of starting watcher (use with --watch)
    #[arg(long)]
    pub status: bool,

    /// simulate matching for file names read from stdin (pure string mode)
    #[arg(long)]
    pub simulate: bool,

    /// dry run (no changes)
    #[arg(long)]
    pub dry_run: bool,

    /// copy instead of move
    #[arg(long)]
    pub copy: bool,

    /// skip confirmations (required for overwrite in non-interactive mode)
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

// ============================================================
// CommandSpec 实现
// ============================================================

#[cfg(feature = "redirect")]
use crate::xun_core::command::CommandSpec;
#[cfg(feature = "redirect")]
use crate::xun_core::context::CmdContext;
#[cfg(feature = "redirect")]
use crate::xun_core::error::XunError;
#[cfg(feature = "redirect")]
use crate::xun_core::value::Value;

/// redirect 命令。
#[cfg(feature = "redirect")]
pub struct RedirectCmdSpec {
    pub args: RedirectCmd,
}

#[cfg(feature = "redirect")]
impl CommandSpec for RedirectCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::redirect::cmd_redirect(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}
