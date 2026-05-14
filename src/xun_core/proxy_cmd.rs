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
    pub sub: ProxySubCommand,
}

/// Proxy 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum ProxySubCommand {
    /// 设置代理
    Set(ProxySetArgs),
    /// 显示当前代理配置
    Show(ProxyShowArgs),
    /// 删除代理配置
    Rm(ProxyRmArgs),
}

/// proxy set 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProxySetArgs {
    /// 代理 URL（如 http://127.0.0.1:7890）
    pub url: String,

    /// no_proxy 列表
    #[arg(short = 'n', long, default_value = "localhost,127.0.0.1")]
    pub noproxy: String,

    /// 仅为指定工具设置（cargo,git,npm,msys2）
    #[arg(short = 'o', long)]
    pub only: Option<String>,
}

/// proxy show 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProxyShowArgs {
    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// proxy rm 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProxyRmArgs {
    /// 仅为指定工具删除（cargo,git,npm,msys2）
    #[arg(short = 'o', long)]
    pub only: Option<String>,
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
// CommandSpec 实现
// ============================================================

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;
use crate::xun_core::services::proxy as proxy_svc;

/// proxy show — 显示当前代理配置。
pub struct ProxyShowCmd {
    pub args: ProxyShowArgs,
}

impl CommandSpec for ProxyShowCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let info = proxy_svc::show_proxy()?;
        let table = info.to_table();
        Ok(Value::List(
            table.rows.into_iter().map(Value::Record).collect(),
        ))
    }
}

/// proxy set — 设置代理配置。
pub struct ProxySetCmd {
    pub args: ProxySetArgs,
}

impl CommandSpec for ProxySetCmd {
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

/// proxy rm — 删除代理配置。
pub struct ProxyRmCmd {
    pub args: ProxyRmArgs,
}

impl CommandSpec for ProxyRmCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        proxy_svc::rm_proxy_service(self.args.only.as_deref())?;
        Ok(Value::Null)
    }
}
