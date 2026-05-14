use clap::Args;

use super::defaults::default_output_format;

/// List listening ports (TCP by default).
#[derive(Args, Debug, Clone)]
pub struct PortsCmd {
    /// show all TCP listening ports
    #[arg(long)]
    pub all: bool,

    /// show UDP bound ports
    #[arg(long)]
    pub udp: bool,

    /// filter port range (e.g. 3000-3999)
    #[arg(long)]
    pub range: Option<String>,

    /// filter by pid
    #[arg(long)]
    pub pid: Option<u32>,

    /// filter by process name (substring)
    #[arg(long)]
    pub name: Option<String>,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

/// Kill processes that occupy ports.
#[derive(Args, Debug, Clone)]
pub struct KillCmd {
    /// port list, e.g. 3000,8080,5173
    pub ports: String,

    /// skip confirmation
    #[arg(short = 'f', long)]
    pub force: bool,

    /// tcp only
    #[arg(long)]
    pub tcp: bool,

    /// udp only
    #[arg(long)]
    pub udp: bool,
}

/// List running processes by name, PID, or window title.
#[derive(Args, Debug, Clone)]
pub struct PsCmd {
    /// fuzzy match by process name
    pub pattern: Option<String>,

    /// exact PID lookup
    #[arg(long)]
    pub pid: Option<u32>,

    /// fuzzy match by window title
    #[arg(short = 'w', long)]
    pub win: Option<String>,
}

/// Kill processes by name, PID, or window title.
#[derive(Args, Debug, Clone)]
pub struct PkillCmd {
    /// process name, PID, or window title when --window is set
    pub target: String,

    /// treat target as window title
    #[arg(short = 'w', long)]
    pub window: bool,

    /// skip interactive confirmation
    #[arg(short = 'f', long)]
    pub force: bool,
}
