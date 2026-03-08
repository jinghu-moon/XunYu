use argh::FromArgs;

use super::defaults::default_output_format;

/// Proxy management.
#[derive(FromArgs)]
#[argh(subcommand, name = "proxy")]
pub struct ProxyCmd {
    #[argh(subcommand)]
    pub cmd: ProxySubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum ProxySubCommand {
    Set(ProxySetCmd),
    Del(ProxyDelCmd),
    Get(ProxyGetCmd),
    Detect(ProxyDetectCmd),
    Test(ProxyTestCmd),
}

/// Proxy On (pon)
#[derive(FromArgs)]
#[argh(subcommand, name = "pon")]
pub struct ProxyOnCmd {
    /// proxy url (optional, auto-detect system proxy)
    #[argh(positional)]
    pub url: Option<String>,

    /// skip connectivity test after enabling proxy
    #[argh(switch)]
    pub no_test: bool,

    /// no_proxy list
    #[argh(
        option,
        short = 'n',
        default = "String::from(\"localhost,127.0.0.1,::1,.local\")"
    )]
    pub noproxy: String,

    /// msys2 root override
    #[argh(option, short = 'm')]
    pub msys2: Option<String>,
}

/// Proxy Off (poff)
#[derive(FromArgs)]
#[argh(subcommand, name = "poff")]
pub struct ProxyOffCmd {
    /// msys2 root override
    #[argh(option, short = 'm')]
    pub msys2: Option<String>,
}

/// Proxy Status (pst)
#[derive(FromArgs)]
#[argh(subcommand, name = "pst")]
pub struct ProxyStatusCmd {
    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

/// Proxy Exec (px)
#[derive(FromArgs)]
#[argh(subcommand, name = "px")]
pub struct ProxyExecCmd {
    /// proxy url (optional)
    #[argh(option, short = 'u')]
    pub url: Option<String>,

    /// no_proxy list
    #[argh(
        option,
        short = 'n',
        default = "String::from(\"localhost,127.0.0.1,::1,.local\")"
    )]
    pub noproxy: String,

    /// command and args
    #[argh(positional)]
    pub cmd: Vec<String>,
}

/// Set proxy.
#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
pub struct ProxySetCmd {
    /// proxy url (e.g. http://127.0.0.1:7890)
    #[argh(positional)]
    pub url: String,

    /// no_proxy list (default: localhost,127.0.0.1)
    #[argh(option, default = "String::from(\"localhost,127.0.0.1\")", short = 'n')]
    pub noproxy: String,

    /// msys2 root override
    #[argh(option, short = 'm')]
    pub msys2: Option<String>,

    /// only set for: cargo,git,npm,msys2 (comma separated)
    #[argh(option, short = 'o')]
    pub only: Option<String>,
}

/// Delete proxy.
#[derive(FromArgs)]
#[argh(subcommand, name = "del")]
pub struct ProxyDelCmd {
    /// msys2 root override
    #[argh(option, short = 'm')]
    pub msys2: Option<String>,

    /// only delete for: cargo,git,npm,msys2 (comma separated)
    #[argh(option, short = 'o')]
    pub only: Option<String>,
}

/// Get current git proxy config.
#[derive(FromArgs)]
#[argh(subcommand, name = "get")]
pub struct ProxyGetCmd {}

/// Detect system proxy.
#[derive(FromArgs)]
#[argh(subcommand, name = "detect")]
pub struct ProxyDetectCmd {
    /// output format: auto|table|tsv|json
    #[argh(option, short = 'f', default = "default_output_format()")]
    pub format: String,
}

/// Test proxy latency.
#[derive(FromArgs)]
#[argh(subcommand, name = "test")]
pub struct ProxyTestCmd {
    /// proxy url
    #[argh(positional)]
    pub url: String,

    /// targets (comma separated), use "proxy" to test proxy itself
    #[argh(option, short = 't')]
    pub targets: Option<String>,

    /// timeout seconds
    #[argh(option, short = 'w', default = "5")]
    pub timeout: u64,

    /// max concurrent probes
    #[argh(option, short = 'j', default = "3")]
    pub jobs: usize,
}
