//! Completion CommandSpec 实现

use clap::Args;

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;
use crate::xun_core::value::Value;

/// Generate shell completion script.
#[derive(Args, Debug, Clone)]
pub struct CompletionCmd {
    /// shell type: powershell | bash | zsh | fish
    pub shell: String,
}

/// Internal completion entry (shell-pre-tokenized args).
#[derive(Args, Debug, Clone)]
pub struct CompleteCmd {
    /// pre-tokenized args after command name
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

/// completion 命令 — 生成 shell 补全脚本。
pub struct CompletionCmdSpec {
    pub args: CompletionCmd,
}

impl CommandSpec for CompletionCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::completion::cmd_completion(self.args.clone())?;
        Ok(Value::Null)
    }
}

/// complete 命令 — 内部动态补全入口。
pub struct CompleteCmdSpec {
    pub args: CompleteCmd,
}

impl CommandSpec for CompleteCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::completion::cmd_complete(self.args.clone())?;
        Ok(Value::Null)
    }
}
