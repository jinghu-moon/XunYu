use argh::FromArgs;

use super::defaults::default_output_format;

/// List listening ports (TCP by default).
#[derive(FromArgs)]
#[argh(subcommand, name = "ports")]
pub struct PortsCmd {
    /// show all TCP listening ports
    #[argh(switch)]
    pub all: bool,

    /// show UDP bound ports
    #[argh(switch)]
    pub udp: bool,

    /// filter port range (e.g. 3000-3999)
    #[argh(option)]
    pub range: Option<String>,

    /// filter by pid
    #[argh(option)]
    pub pid: Option<u32>,

    /// filter by process name (substring)
    #[argh(option)]
    pub name: Option<String>,

    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

/// Kill processes that occupy ports.
#[derive(FromArgs)]
#[argh(subcommand, name = "kill")]
pub struct KillCmd {
    /// port list, e.g. 3000,8080,5173
    #[argh(positional)]
    pub ports: String,

    /// skip confirmation
    #[argh(switch, short = 'f')]
    pub force: bool,

    /// tcp only
    #[argh(switch)]
    pub tcp: bool,

    /// udp only
    #[argh(switch)]
    pub udp: bool,
}

/// List running processes by name, PID, or window title.
#[derive(FromArgs)]
#[argh(subcommand, name = "ps")]
pub struct PsCmd {
    /// fuzzy match by process name
    #[argh(positional)]
    pub pattern: Option<String>,

    /// exact PID lookup
    #[argh(option)]
    pub pid: Option<u32>,

    /// fuzzy match by window title
    #[argh(option, short = 'w')]
    pub win: Option<String>,
}

/// Kill processes by name, PID, or window title.
#[derive(FromArgs)]
#[argh(subcommand, name = "pkill")]
pub struct PkillCmd {
    /// process name, PID, or window title when --window is set
    #[argh(positional)]
    pub target: String,

    /// treat target as window title
    #[argh(switch, short = 'w')]
    pub window: bool,

    /// skip interactive confirmation
    #[argh(switch, short = 'f')]
    pub force: bool,
}
