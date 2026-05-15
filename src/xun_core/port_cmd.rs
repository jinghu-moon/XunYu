//! Port CLI 定义（clap derive）+ PortInfo 输出类型

use clap::Args;
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

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
    #[arg(short = 'f', long, default_value = "auto")]
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

// ============================================================
// PortInfo — 端口信息输出类型
// ============================================================

/// 端口占用信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    /// 端口号
    pub port: u16,
    /// 协议（tcp/udp）
    pub protocol: String,
    /// 进程 ID
    pub pid: u32,
    /// 进程名
    pub process_name: String,
    /// 本地地址
    pub local_addr: String,
}

impl PortInfo {
    pub fn new(
        port: u16,
        protocol: impl Into<String>,
        pid: u32,
        process_name: impl Into<String>,
        local_addr: impl Into<String>,
    ) -> Self {
        Self {
            port,
            protocol: protocol.into(),
            pid,
            process_name: process_name.into(),
            local_addr: local_addr.into(),
        }
    }
}

impl TableRow for PortInfo {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("port", ValueKind::Int),
            ColumnDef::new("protocol", ValueKind::String),
            ColumnDef::new("pid", ValueKind::Int),
            ColumnDef::new("process_name", ValueKind::String),
            ColumnDef::new("local_addr", ValueKind::String),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::Int(self.port as i64),
            Value::String(self.protocol.clone()),
            Value::Int(self.pid as i64),
            Value::String(self.process_name.clone()),
            Value::String(self.local_addr.clone()),
        ]
    }
}

// ============================================================
// CommandSpec 实现
// ============================================================

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;

/// ports 命令。
pub struct PortsCmdSpec {
    pub args: PortsCmd,
}

impl CommandSpec for PortsCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::ports::cmd_ports(self.args.clone())?;
        Ok(Value::Null)
    }
}

/// kill 命令。
pub struct KillCmdSpec {
    pub args: KillCmd,
}

impl CommandSpec for KillCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::ports::cmd_kill(self.args.clone())?;
        Ok(Value::Null)
    }
}
