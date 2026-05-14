//! Ctx CLI 定义（clap derive）
//!
//! 新架构的 ctx 命令定义，替代 argh 版本。

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// Context 配置管理命令。
#[derive(Parser, Debug, Clone)]
#[command(name = "ctx", about = "Context switch profiles")]
pub struct CtxCmd {
    #[command(subcommand)]
    pub sub: CtxSubCommand,
}

/// Ctx 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum CtxSubCommand {
    /// 定义或更新配置
    Set(CtxSetArgs),
    /// 激活配置
    Use(CtxUseArgs),
    /// 停用当前配置
    Off(CtxOffArgs),
    /// 列出所有配置
    List(CtxListArgs),
    /// 显示配置详情
    Show(CtxShowArgs),
    /// 删除配置
    Del(CtxDelArgs),
    /// 重命名配置
    Rename(CtxRenameArgs),
}

/// ctx set 参数。
#[derive(Parser, Debug, Clone)]
pub struct CtxSetArgs {
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
#[derive(Parser, Debug, Clone)]
pub struct CtxUseArgs {
    /// 配置名称
    pub name: String,
}

/// ctx off 参数。
#[derive(Parser, Debug, Clone)]
pub struct CtxOffArgs {}

/// ctx list 参数。
#[derive(Parser, Debug, Clone)]
pub struct CtxListArgs {
    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// ctx show 参数。
#[derive(Parser, Debug, Clone)]
pub struct CtxShowArgs {
    /// 配置名称（默认当前激活）
    pub name: Option<String>,
    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// ctx del 参数。
#[derive(Parser, Debug, Clone)]
pub struct CtxDelArgs {
    /// 配置名称
    pub name: String,
}

/// ctx rename 参数。
#[derive(Parser, Debug, Clone)]
pub struct CtxRenameArgs {
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
