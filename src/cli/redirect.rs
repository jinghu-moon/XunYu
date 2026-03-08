use argh::FromArgs;

use super::defaults::default_output_format;

#[cfg(feature = "redirect")]
/// Redirect files in a directory into categorized subfolders.
#[derive(FromArgs)]
#[argh(subcommand, name = "redirect")]
pub struct RedirectCmd {
    /// source directory (default: current directory)
    #[argh(positional)]
    pub source: Option<String>,

    /// profile name under config.redirect.profiles (default: "default")
    #[argh(option, default = "String::from(\"default\")")]
    pub profile: String,

    /// explain why a file would match or not match rules (pure string mode)
    #[argh(option)]
    pub explain: Option<String>,

    /// show rules coverage summary after a run (printed to stderr)
    #[argh(switch)]
    pub stats: bool,

    /// show preview summary and require confirmation before executing (interactive unless --yes)
    #[argh(switch)]
    pub confirm: bool,

    /// review each planned file action interactively (y/n/a/q)
    #[argh(switch)]
    pub review: bool,

    /// query audit log (redirect tx history)
    #[argh(switch)]
    pub log: bool,

    /// filter audit log by tx id (use with --log)
    #[argh(option)]
    pub tx: Option<String>,

    /// show last N tx summaries (use with --log)
    #[argh(option)]
    pub last: Option<usize>,

    /// validate config only (no scan/no watch)
    #[argh(switch)]
    pub validate: bool,

    /// write a plan file instead of executing (json)
    #[argh(option)]
    pub plan: Option<String>,

    /// apply a previously generated plan file (json)
    #[argh(option)]
    pub apply: Option<String>,

    /// undo a previous redirect by tx id (read from audit.jsonl)
    #[argh(option)]
    pub undo: Option<String>,

    /// watch mode (daemon: continuously apply redirect rules)
    #[argh(switch)]
    pub watch: bool,

    /// show watch status instead of starting watcher (use with --watch)
    #[argh(switch)]
    pub status: bool,

    /// simulate matching for file names read from stdin (pure string mode)
    #[argh(switch)]
    pub simulate: bool,

    /// dry run (no changes)
    #[argh(switch)]
    pub dry_run: bool,

    /// copy instead of move
    #[argh(switch)]
    pub copy: bool,

    /// skip confirmations (required for overwrite in non-interactive mode)
    #[argh(switch, short = 'y')]
    pub yes: bool,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}
