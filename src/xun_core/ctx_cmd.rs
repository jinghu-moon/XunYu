//! Ctx CLI 定义（clap derive）+ CtxProfile 输出类型

use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// Context 配置管理命令。
#[derive(Parser, Debug, Clone)]
#[command(name = "ctx", about = "Context switch profiles")]
pub struct CtxCmd {
    #[command(subcommand)]
    pub cmd: CtxSubCommand,
}

/// Ctx 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum CtxSubCommand {
    /// 定义或更新配置
    Set(CtxSetCmd),
    /// 激活配置
    Use(CtxUseCmd),
    /// 停用当前配置
    Off(CtxOffCmd),
    /// 列出所有配置
    List(CtxListCmd),
    /// 显示配置详情
    Show(CtxShowCmd),
    /// 删除配置
    Del(CtxDelCmd),
    /// 重命名配置
    Rename(CtxRenameCmd),
}

/// ctx set 参数。
#[derive(Args, Debug, Clone)]
pub struct CtxSetCmd {
    /// 配置名称
    pub name: String,
    /// 工作目录
    #[arg(long)]
    pub path: Option<String>,
    /// 代理设置
    #[arg(long)]
    pub proxy: Option<String>,
    /// NO_PROXY
    #[arg(long)]
    pub noproxy: Option<String>,
    /// 默认标签（逗号分隔）
    #[arg(short = 't', long)]
    pub tag: Option<String>,
    /// 环境变量（KEY=VALUE）
    #[arg(long)]
    pub env: Vec<String>,
    /// 从文件导入环境变量
    #[arg(long)]
    pub env_file: Option<String>,
}

/// ctx use 参数。
#[derive(Args, Debug, Clone)]
pub struct CtxUseCmd {
    /// 配置名称
    pub name: String,
}

/// ctx off 参数。
#[derive(Args, Debug, Clone)]
pub struct CtxOffCmd {}

/// ctx list 参数。
#[derive(Args, Debug, Clone)]
pub struct CtxListCmd {
    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// ctx show 参数。
#[derive(Args, Debug, Clone)]
pub struct CtxShowCmd {
    /// 配置名称（默认当前激活）
    pub name: Option<String>,
    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// ctx del 参数。
#[derive(Args, Debug, Clone)]
pub struct CtxDelCmd {
    /// 配置名称
    pub name: String,
}

/// ctx rename 参数。
#[derive(Args, Debug, Clone)]
pub struct CtxRenameCmd {
    /// 旧名称
    pub old: String,
    /// 新名称
    pub new: String,
}

// ============================================================
// CtxProfile — 配置概要输出类型
// ============================================================

/// 配置概要（用于 list 命令输出）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtxProfile {
    /// 配置名称
    pub name: String,
    /// 工作目录
    pub path: String,
    /// 是否激活
    pub active: bool,
}

impl CtxProfile {
    pub fn new(name: impl Into<String>, path: impl Into<String>, active: bool) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            active,
        }
    }
}

impl TableRow for CtxProfile {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("name", ValueKind::String),
            ColumnDef::new("path", ValueKind::String),
            ColumnDef::new("active", ValueKind::Bool),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.name.clone()),
            Value::String(self.path.clone()),
            Value::Bool(self.active),
        ]
    }
}

// ============================================================
// CommandSpec 实现
// ============================================================

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;

/// ctx 命令。
pub struct CtxCmdSpec {
    pub args: CtxCmd,
}

impl CommandSpec for CtxCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::ctx::cmd_ctx(self.args.clone())?;
        Ok(Value::Null)
    }
}
