//! Delete (rm) CommandSpec 实现

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;
use crate::xun_core::value::Value;

/// DeleteCmd is defined in bookmark/cli_namespace.rs, re-exported via cli.rs.

/// rm / delete 命令。
pub struct DeleteCmdSpec {
    pub args: crate::cli::DeleteCmd,
}

impl CommandSpec for DeleteCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::delete::cmd_delete(self.args.clone())?;
        Ok(Value::Null)
    }
}
