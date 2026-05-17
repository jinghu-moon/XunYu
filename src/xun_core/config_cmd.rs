//! Config CLI 定义（clap derive）+ ConfigEntry 输出类型

use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// Manage ~/.xun.config.json.
#[derive(Parser, Debug, Clone)]
#[command(
    after_help = "EXAMPLES:\n    \
        xun config get proxy.defaultUrl     # get a config value\n    \
        xun config set tree.defaultDepth 3  # set a config value\n    \
        xun config edit                     # open config in editor"
)]
pub struct ConfigCmd {
    #[command(subcommand)]
    pub cmd: ConfigSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigSubCommand {
    Get(ConfigGetCmd),
    Set(ConfigSetCmd),
    Edit(ConfigEditCmd),
}

/// Get a config value by dot path (e.g. proxy.defaultUrl).
#[derive(Args, Debug, Clone)]
pub struct ConfigGetCmd {
    /// key path (dot separated)
    pub key: String,
}

/// Set a config value by dot path (e.g. tree.defaultDepth 3).
#[derive(Args, Debug, Clone)]
pub struct ConfigSetCmd {
    /// key path (dot separated)
    pub key: String,

    /// value (JSON if possible, otherwise string)
    pub value: String,
}

/// Open config file in an editor.
#[derive(Args, Debug, Clone)]
pub struct ConfigEditCmd {}

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

// ============================================================
// CommandSpec 实现
// ============================================================

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;

/// config get 命令。
pub struct ConfigGetCmdSpec {
    pub args: ConfigGetCmd,
}

impl CommandSpec for ConfigGetCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let cmd = ConfigCmd {
            cmd: ConfigSubCommand::Get(self.args.clone()),
        };
        crate::commands::app_config::cmd_config(cmd)?;
        Ok(Value::Null)
    }
}

/// config set 命令。
pub struct ConfigSetCmdSpec {
    pub args: ConfigSetCmd,
}

impl CommandSpec for ConfigSetCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let cmd = ConfigCmd {
            cmd: ConfigSubCommand::Set(self.args.clone()),
        };
        crate::commands::app_config::cmd_config(cmd)?;
        Ok(Value::Null)
    }
}

/// config edit 命令。
pub struct ConfigEditCmdSpec {
    pub args: ConfigEditCmd,
}

impl CommandSpec for ConfigEditCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let cmd = ConfigCmd {
            cmd: ConfigSubCommand::Edit(self.args.clone()),
        };
        crate::commands::app_config::cmd_config(cmd)?;
        Ok(Value::Null)
    }
}
