use crate::cli::RestoreCmd;
use crate::output::CliResult;

pub(crate) fn cmd_restore(args: RestoreCmd) -> CliResult {
    crate::backup::app::restore::cmd_restore(args)
}
