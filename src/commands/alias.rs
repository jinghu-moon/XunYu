use crate::cli::AliasCmd;
use crate::output::CliResult;

pub(crate) fn cmd_alias(args: AliasCmd) -> CliResult {
    crate::alias::cmd_alias(args).map_err(crate::alias::error::to_cli_error)
}
