//! Config CLI 定义（clap derive）+ ConfigEntry 输出类型
//!
//! 新架构的 config 命令定义，替代 argh 版本。

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// Config 管理命令。
#[derive(Parser, Debug, Clone)]
#[command(name = "config", about = "Manage configuration")]
pub struct ConfigCmd {
    #[command(subcommand)]
    pub sub: ConfigSubCommand,
}

/// Config 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum ConfigSubCommand {
    /// 获取配置值
    Get(ConfigGetArgs),
    /// 设置配置值
    Set(ConfigSetArgs),
    /// 编辑配置文件
    Edit(ConfigEditArgs),
}

/// config get 参数。
#[derive(Parser, Debug, Clone)]
pub struct ConfigGetArgs {
    /// 配置键路径（点分隔，如 proxy.defaultUrl）
    pub key: String,
}

/// config set 参数。
#[derive(Parser, Debug, Clone)]
pub struct ConfigSetArgs {
    /// 配置键路径（点分隔）
    pub key: String,
    /// 配置值（JSON 格式或字符串）
    pub value: String,
}

/// config edit 参数。
#[derive(Parser, Debug, Clone)]
pub struct ConfigEditArgs {}

// ============================================================
// ConfigEntry — 配置条目输出类型
// ============================================================

/// 配置条目（用于 get 命令输出）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    /// 配置键路径
    pub key: String,
    /// 配置值（JSON 字符串表示）
    pub value: String,
}

impl ConfigEntry {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl TableRow for ConfigEntry {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("key", ValueKind::String),
            ColumnDef::new("value", ValueKind::String),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.key.clone()),
            Value::String(self.value.clone()),
        ]
    }
}
