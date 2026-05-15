//! Proxy CLI 定义（clap derive）+ ProxyInfo 输出类型
//!
//! 新架构的 proxy 命令定义，替代 argh 版本。

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// Proxy 管理命令。
#[derive(Parser, Debug, Clone)]
#[command(name = "proxy", about = "Proxy management")]
pub struct ProxyCmd {
    #[command(subcommand)]
    pub cmd: ProxySubCommand,
}

/// Proxy 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum ProxySubCommand {
    /// 设置代理
    Set(ProxySetCmd),
    /// 显示当前代理配置
    Show(ProxyShowCmd),
    /// 删除代理配置
    Rm(ProxyRmCmd),
    /// 检测系统代理
    Detect(ProxyDetectCmd),
    /// 显示代理状态
    Status(ProxyStatusCmd),
    /// 测试代理连通性
    Test(ProxyTestCmd),
}

/// proxy set 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProxySetCmd {
    /// 代理 URL（如 http://127.0.0.1:7890）
    pub url: String,

    /// no_proxy 列表
    #[arg(short = 'n', long, default_value = "localhost,127.0.0.1")]
    pub noproxy: String,

    /// 仅为指定工具设置（cargo,git,npm,msys2）
    #[arg(short = 'o', long)]
    pub only: Option<String>,

    /// msys2 root override
    #[arg(short = 'm', long)]
    pub msys2: Option<String>,
}

/// proxy show 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProxyShowCmd {
    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// proxy rm 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProxyRmCmd {
    /// 仅为指定工具删除（cargo,git,npm,msys2）
    #[arg(short = 'o', long)]
    pub only: Option<String>,

    /// msys2 root override
    #[arg(short = 'm', long)]
    pub msys2: Option<String>,
}

/// proxy detect 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProxyDetectCmd {
    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// proxy status 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProxyStatusCmd {
    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// proxy test 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProxyTestCmd {
    /// proxy url
    pub url: String,

    /// test targets (comma-separated)
    #[arg(short = 't', long)]
    pub targets: Option<String>,

    /// timeout in seconds
    #[arg(long, default_value = "10")]
    pub timeout: u64,

    /// parallel jobs
    #[arg(short = 'j', long, default_value = "4")]
    pub jobs: usize,
}

// ============================================================
// ProxyInfo — 代理信息输出类型
// ============================================================

/// 代理配置信息（用于 show 命令输出）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInfo {
    /// 代理 URL
    pub url: String,
    /// no_proxy 列表
    pub noproxy: String,
    /// 来源（环境变量/git config/手动）
    pub source: String,
    /// 是否启用
    pub enabled: bool,
}

impl ProxyInfo {
    pub fn new(url: impl Into<String>, noproxy: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            noproxy: noproxy.into(),
            source: source.into(),
            enabled: true,
        }
    }
}

impl TableRow for ProxyInfo {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("url", ValueKind::String),
            ColumnDef::new("noproxy", ValueKind::String),
            ColumnDef::new("source", ValueKind::String),
            ColumnDef::new("enabled", ValueKind::Bool),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.url.clone()),
            Value::String(self.noproxy.clone()),
            Value::String(self.source.clone()),
            Value::Bool(self.enabled),
        ]
    }
}

// ============================================================
// Proxy 简写命令类型（pon/poff/px）
// ============================================================

use clap::Args;

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

// ============================================================
// CommandSpec 实现
// ============================================================

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;
use crate::xun_core::services::proxy as proxy_svc;

/// proxy show 命令。
pub struct ProxyShowCmdSpec {
    pub args: ProxyShowCmd,
}

impl CommandSpec for ProxyShowCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let info = proxy_svc::show_proxy()?;
        let table = info.to_table();
        Ok(Value::List(
            table.rows.into_iter().map(Value::Record).collect(),
        ))
    }
}

/// proxy set 命令。
pub struct ProxySetCmdSpec {
    pub args: ProxySetCmd,
}

impl CommandSpec for ProxySetCmdSpec {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.url.is_empty() {
            return Err(XunError::user("proxy URL is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        proxy_svc::set_proxy_service(
            &self.args.url,
            &self.args.noproxy,
            self.args.only.as_deref(),
        )?;
        Ok(Value::Null)
    }
}

/// proxy rm 命令。
pub struct ProxyRmCmdSpec {
    pub args: ProxyRmCmd,
}

impl CommandSpec for ProxyRmCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        proxy_svc::rm_proxy_service(self.args.only.as_deref())?;
        Ok(Value::Null)
    }
}

/// pon 命令。
pub struct ProxyOnCmdSpec {
    pub args: ProxyOnCmd,
}

impl CommandSpec for ProxyOnCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::proxy::cmd_proxy_on(self.args.clone())?;
        Ok(Value::Null)
    }
}

/// poff 命令。
pub struct ProxyOffCmdSpec {
    pub args: ProxyOffCmd,
}

impl CommandSpec for ProxyOffCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::proxy::cmd_proxy_off(self.args.clone())?;
        Ok(Value::Null)
    }
}

/// px 命令。
pub struct ProxyExecCmdSpec {
    pub args: ProxyExecCmd,
}

impl CommandSpec for ProxyExecCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::proxy::cmd_proxy_exec(self.args.clone())?;
        Ok(Value::Null)
    }
}

/// proxy detect 命令。
pub struct ProxyDetectCmdSpec {
    pub args: ProxyDetectCmd,
}

impl CommandSpec for ProxyDetectCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::proxy::ops::cmd_proxy_detect(self.args.clone())?;
        Ok(Value::Null)
    }
}

/// proxy status 命令。
pub struct ProxyStatusCmdSpec {
    pub args: ProxyStatusCmd,
}

impl CommandSpec for ProxyStatusCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::proxy::ops::cmd_proxy_status(self.args.clone())?;
        Ok(Value::Null)
    }
}
