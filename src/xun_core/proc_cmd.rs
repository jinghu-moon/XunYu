//! Proc CLI 定义（clap derive）
//!
//! 新架构的 proc 命令定义，替代 argh 版本。

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// 进程管理命令。
#[derive(Parser, Debug, Clone)]
#[command(name = "proc", about = "Process management")]
pub struct ProcCmd {
    #[command(subcommand)]
    pub sub: ProcSubCommand,
}

/// Proc 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum ProcSubCommand {
    /// 列出进程
    List(ProcListArgs),
    /// 杀死进程
    Kill(ProcKillArgs),
}

/// proc list 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProcListArgs {
    /// 模糊匹配进程名
    pub pattern: Option<String>,
    /// 精确 PID 查找
    #[arg(long)]
    pub pid: Option<u32>,
    /// 模糊匹配窗口标题
    #[arg(short = 'w', long)]
    pub win: Option<String>,
    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// proc kill 参数。
#[derive(Parser, Debug, Clone)]
pub struct ProcKillArgs {
    /// 目标：进程名、PID 或窗口标题（配合 --window）
    pub target: String,
    /// 将 target 视为窗口标题
    #[arg(short = 'w', long)]
    pub window: bool,
    /// 跳过确认
    #[arg(short = 'f', long)]
    pub force: bool,
}

// ============================================================
// ProcInfo — 进程信息输出类型
// ============================================================

/// 进程信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcInfo {
    /// 进程 ID
    pub pid: u32,
    /// 父进程 ID
    pub ppid: u32,
    /// 进程名
    pub name: String,
    /// 可执行文件路径
    pub exe_path: String,
    /// 线程数
    pub thread_count: u32,
    /// 窗口标题
    pub window_title: String,
}

impl ProcInfo {
    pub fn new(
        pid: u32,
        ppid: u32,
        name: impl Into<String>,
        exe_path: impl Into<String>,
        thread_count: u32,
        window_title: impl Into<String>,
    ) -> Self {
        Self {
            pid,
            ppid,
            name: name.into(),
            exe_path: exe_path.into(),
            thread_count,
            window_title: window_title.into(),
        }
    }
}

impl TableRow for ProcInfo {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("pid", ValueKind::Int),
            ColumnDef::new("ppid", ValueKind::Int),
            ColumnDef::new("name", ValueKind::String),
            ColumnDef::new("exe_path", ValueKind::String),
            ColumnDef::new("thread_count", ValueKind::Int),
            ColumnDef::new("window_title", ValueKind::String),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::Int(self.pid as i64),
            Value::Int(self.ppid as i64),
            Value::String(self.name.clone()),
            Value::String(self.exe_path.clone()),
            Value::Int(self.thread_count as i64),
            Value::String(self.window_title.clone()),
        ]
    }
}
