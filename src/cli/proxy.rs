use clap::{Args, Parser, Subcommand};

use super::defaults::default_output_format;

/// Proxy management.
#[derive(Parser, Debug, Clone)]
pub struct ProxyCmd {
    #[command(subcommand)]
    pub cmd: ProxySubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ProxySubCommand {
    Set(ProxySetCmd),
    #[command(name = "rm", alias = "del")]
    Rm(ProxyDelCmd),
    #[command(name = "show", alias = "get")]
    Show(ProxyGetCmd),
    Detect(ProxyDetectCmd),
    Test(ProxyTestCmd),
}

/// Proxy On (pon)
#[derive(Args, Debug, Clone)]
pub struct ProxyOnCmd {
    /// proxy url (optional, auto-detect system proxy)
    pub url: Option<String>,

    /// skip connectivity test after enabling proxy
    #[arg(long)]
    pub no_test: bool,

    /// no_proxy list
    #[arg(short = 'n', long, default_value = "localhost,127.0.0.1,::1,.local")]
    pub noproxy: String,

    /// msys2 root override
    #[arg(short = 'm', long)]
    pub msys2: Option<String>,
}

/// Proxy Off (poff)
#[derive(Args, Debug, Clone)]
pub struct ProxyOffCmd {
    /// msys2 root override
    #[arg(short = 'm', long)]
    pub msys2: Option<String>,
}

/// Proxy Status (pst)
#[derive(Args, Debug, Clone)]
pub struct ProxyStatusCmd {
    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

/// Proxy Exec (px)
#[derive(Args, Debug, Clone)]
pub struct ProxyExecCmd {
    /// proxy url (optional)
    #[arg(short = 'u', long)]
    pub url: Option<String>,

    /// no_proxy list
    #[arg(short = 'n', long, default_value = "localhost,127.0.0.1,::1,.local")]
    pub noproxy: String,

    /// command and args
    #[arg(trailing_var_arg = true)]
    pub cmd: Vec<String>,
}

/// Set proxy.
#[derive(Args, Debug, Clone)]
pub struct ProxySetCmd {
    /// proxy url (e.g. http://127.0.0.1:7890)
    pub url: String,

    /// no_proxy list (default: localhost,127.0.0.1)
    #[arg(short = 'n', long, default_value = "localhost,127.0.0.1")]
    pub noproxy: String,

    /// msys2 root override
    #[arg(short = 'm', long)]
    pub msys2: Option<String>,

    /// only set for: cargo,git,npm,msys2 (comma separated)
    #[arg(short = 'o', long)]
    pub only: Option<String>,
}

/// Delete proxy.
#[derive(Args, Debug, Clone)]
pub struct ProxyDelCmd {
    /// msys2 root override
    #[arg(short = 'm', long)]
    pub msys2: Option<String>,

    /// only delete for: cargo,git,npm,msys2 (comma separated)
    #[arg(short = 'o', long)]
    pub only: Option<String>,
}

/// Get current git proxy config.
#[derive(Args, Debug, Clone)]
pub struct ProxyGetCmd {}

/// Detect system proxy.
#[derive(Args, Debug, Clone)]
pub struct ProxyDetectCmd {
    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value_t = default_output_format())]
    pub format: String,
}

/// Test proxy latency.
#[derive(Args, Debug, Clone)]
pub struct ProxyTestCmd {
    /// proxy url
    pub url: String,

    /// targets (comma separated), use "proxy" to test proxy itself
    #[arg(short = 't', long)]
    pub targets: Option<String>,

    /// timeout seconds
    #[arg(short = 'w', long, default_value_t = 5)]
    pub timeout: u64,

    /// max concurrent probes
    #[arg(short = 'j', long, default_value_t = 3)]
    pub jobs: usize,
}
