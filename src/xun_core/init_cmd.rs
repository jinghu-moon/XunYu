//! Init CommandSpec 实现

use clap::Args;

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;
use crate::xun_core::value::Value;

/// Initialize shell integration (print wrapper function).
#[derive(Args, Debug, Clone)]
pub struct InitCmd {
    /// shell type: powershell | bash | zsh
    pub shell: String,
}

/// init 命令 — 打印 shell 集成脚本。
pub struct InitCmdSpec {
    pub args: InitCmd,
}

impl CommandSpec for InitCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let script = super::dispatch::render_init_script(&self.args.shell)?;
        println!("{script}");
        Ok(Value::Null)
    }
}
