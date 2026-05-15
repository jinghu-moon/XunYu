//! FsRm CLI 定义（clap derive）+ CommandSpec

use clap::Args;

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;
use crate::xun_core::value::Value;

/// Delete a file or directory.
#[derive(Args, Debug, Clone)]
pub struct RmCmd {
    /// target path
    pub path: String,

    /// unlock file if locked
    #[cfg(feature = "lock")]
    #[arg(long)]
    pub unlock: bool,

    /// force kill blocking processes
    #[cfg(feature = "lock")]
    #[arg(long)]
    pub force_kill: bool,

    /// schedule deletion on reboot
    #[arg(long)]
    pub on_reboot: bool,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,

    /// force operation bypass protection
    #[arg(long)]
    pub force: bool,

    /// reason for bypass protection
    #[arg(long)]
    pub reason: Option<String>,
}

/// fs rm 命令。
#[cfg(feature = "fs")]
pub struct FsRmCmdSpec {
    pub args: RmCmd,
}

#[cfg(feature = "fs")]
impl CommandSpec for FsRmCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::fs::cmd_rm(self.args.clone())?;
        Ok(Value::Null)
    }
}
