use clap::Args;

use super::defaults::default_output_format;

#[cfg(feature = "redirect")]
/// Redirect files in a directory into categorized subfolders.
#[derive(Args, Debug, Clone)]
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

    /// show preview summary and require confirmation before executing (interactive unless --yes)
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
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}
