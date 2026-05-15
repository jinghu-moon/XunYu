//! Dashboard CLI 定义（clap derive）
//!
//! 新架构的 dashboard 命令定义，替代 argh 版本。
//! ServeCmd 独立命令。

use clap::Parser;

// ── Serve 命令 ───────────────────────────────────────────────────

/// Start web dashboard server.
#[derive(Parser, Debug, Clone)]
#[command(name = "serve", about = "Start web dashboard server")]
pub struct ServeCmd {
    /// listen port (default: 9527)
    #[arg(short = 'p', long, default_value_t = 9527)]
    pub port: u16,
}

// ============================================================
// CommandSpec 实现
// ============================================================

#[cfg(feature = "dashboard")]
use crate::xun_core::command::CommandSpec;
#[cfg(feature = "dashboard")]
use crate::xun_core::context::CmdContext;
#[cfg(feature = "dashboard")]
use crate::xun_core::error::XunError;
#[cfg(feature = "dashboard")]
use crate::xun_core::value::Value;

/// serve 命令。
#[cfg(feature = "dashboard")]
pub struct ServeCmdSpec {
    pub args: ServeCmd,
}

#[cfg(feature = "dashboard")]
impl CommandSpec for ServeCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::dashboard::cmd_serve(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}
