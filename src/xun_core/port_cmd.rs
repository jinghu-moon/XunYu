//! Port CLI 定义（clap derive）
//!
//! 新架构的 port 命令定义，替代 argh 版本。

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// 端口管理命令。
#[derive(Parser, Debug, Clone)]
#[command(name = "port", about = "Port management")]
pub struct PortCmd {
    #[command(subcommand)]
    pub sub: PortSubCommand,
}

/// Port 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum PortSubCommand {
    /// 列出监听端口
    List(PortListArgs),
    /// 杀死占用指定端口的进程
    Kill(PortKillArgs),
}

/// port list 参数。
#[derive(Parser, Debug, Clone)]
pub struct PortListArgs {
    /// 显示所有 TCP 监听端口
    #[arg(long)]
    pub all: bool,
    /// 显示 UDP 绑定端口
    #[arg(long)]
    pub udp: bool,
    /// 端口范围过滤（如 3000-3999）
    #[arg(long)]
    pub range: Option<String>,
    /// 按 PID 过滤
    #[arg(long)]
    pub pid: Option<u32>,
    /// 按进程名过滤（子串匹配）
    #[arg(long)]
    pub name: Option<String>,
    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// port kill 参数。
#[derive(Parser, Debug, Clone)]
pub struct PortKillArgs {
    /// 端口列表（逗号分隔，如 3000,8080,5173）
    pub ports: String,
    /// 跳过确认
    #[arg(short = 'f', long)]
    pub force: bool,
    /// 仅 TCP
    #[arg(long)]
    pub tcp: bool,
    /// 仅 UDP
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
