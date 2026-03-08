use crate::cli::FindCmd;
use crate::output::CliResult;

pub(crate) fn cmd_find(args: FindCmd) -> CliResult {
    crate::find::cmd_find(args)
}
