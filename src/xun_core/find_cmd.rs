//! Find CLI 定义（clap derive）
//!
//! 新架构的 find 命令定义，替代 argh 版本。

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// 查找文件和目录。
#[derive(Parser, Debug, Clone)]
#[command(name = "find", about = "Find files and directories by pattern")]
pub struct FindCmd {
    /// 基础目录（默认当前目录）
    pub paths: Vec<String>,

    /// 包含 glob 模式
    #[arg(short = 'i', long)]
    pub include: Vec<String>,

    /// 排除 glob 模式
    #[arg(short = 'e', long)]
    pub exclude: Vec<String>,

    /// 包含扩展名（逗号分隔）
    #[arg(long)]
    pub extension: Vec<String>,

    /// 深度过滤
    #[arg(short = 'd', long)]
    pub depth: Option<String>,

    /// 仅计数
    #[arg(short = 'c', long)]
    pub count: bool,

    /// 输出格式
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

// ============================================================
// FindResult — 查找结果输出类型
// ============================================================

/// 查找结果条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResult {
    /// 文件路径
    pub path: String,
    /// 文件类型（file/dir/link）
    pub kind: String,
    /// 文件大小（字节）
    pub size: u64,
}

impl FindResult {
    pub fn new(path: impl Into<String>, kind: impl Into<String>, size: u64) -> Self {
        Self {
            path: path.into(),
            kind: kind.into(),
            size,
        }
    }
}

impl TableRow for FindResult {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("path", ValueKind::String),
            ColumnDef::new("kind", ValueKind::String),
            ColumnDef::new("size", ValueKind::Filesize),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.path.clone()),
            Value::String(self.kind.clone()),
            Value::Filesize(self.size),
        ]
    }
}
