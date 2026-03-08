// cli/cstat.rs

use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "cstat")]
/// Code statistics and project cleanup scanner
pub struct CstatCmd {
    /// directory to scan (default: current directory)
    #[argh(positional, default = "String::from(\".\")")]
    pub path: String,

    /// find empty files (0 bytes)
    #[argh(switch)]
    pub empty: bool,

    /// find files with more than N lines
    #[argh(option)]
    pub large: Option<usize>,

    /// find duplicate files by content hash (BLAKE3)
    #[argh(switch)]
    pub dup: bool,

    /// find temporary/leftover files
    #[argh(switch)]
    pub tmp: bool,

    /// enable all issue detections
    #[argh(switch)]
    pub all: bool,

    /// only scan files with these extensions (repeatable)
    #[argh(option)]
    pub ext: Vec<String>,

    /// max directory recursion depth
    #[argh(option)]
    pub depth: Option<usize>,

    /// output format: auto, table, json
    #[argh(option, short = 'f', default = "String::from(\"auto\")")]
    pub format: String,

    /// export JSON report to file
    #[argh(option, short = 'o')]
    pub output: Option<String>,
}
