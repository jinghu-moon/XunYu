use crate::cli::SubCommand;
use crate::output::CliResult;

use super::super::{ctx, env as env_cmd};

pub(super) fn try_dispatch(cmd: SubCommand) -> Result<CliResult, SubCommand> {
    match cmd {
        SubCommand::Ctx(a) => Ok(ctx::cmd_ctx(a)),
        SubCommand::Env(a) => Ok(env_cmd::cmd_env(a)),
        other => Err(other),
    }
}
