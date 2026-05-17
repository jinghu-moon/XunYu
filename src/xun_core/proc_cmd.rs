//! Proc CLI 定义（clap derive）+ ProcInfo 输出类型

use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// Process management.
#[derive(Parser, Debug, Clone)]
#[command(name = "proc", about = "Process management")]
pub struct PsCmd {
    #[command(subcommand)]
    pub cmd: PsSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum PsSubCommand {
    /// List running processes.
    List(PsListArgs),
    /// Kill processes by name, PID, or window title.
    Kill(PkillCmd),
}

/// List running processes by name, PID, or window title.
#[derive(Args, Debug, Clone)]
pub struct PsListArgs {
    /// fuzzy match by process name
    pub pattern: Option<String>,

    /// exact PID lookup
    #[arg(long)]
    pub pid: Option<u32>,

    /// fuzzy match by window title
    #[arg(short = 'w', long)]
    pub win: Option<String>,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
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

// ============================================================
// CommandSpec 实现
// ============================================================

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;

/// ps 命令。
pub struct PsCmdSpec {
    pub args: PsCmd,
}

impl CommandSpec for PsCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        match &self.args.cmd {
            PsSubCommand::List(args) => {
                crate::commands::ports::cmd_ps(args.clone())?;
            }
            PsSubCommand::Kill(args) => {
                crate::commands::ports::cmd_pkill(args.clone())?;
            }
        }
        Ok(Value::Null)
    }
}

/// pkill 命令。
pub struct PkillCmdSpec {
    pub args: PkillCmd,
}

impl CommandSpec for PkillCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::ports::cmd_pkill(self.args.clone())?;
        Ok(Value::Null)
    }
}
