mod core;
mod dashboard;
mod env;
mod misc;

use crate::cli::Xun;
use crate::output::CliResult;

pub(crate) fn dispatch(args: Xun) -> CliResult {
    let cmd = args.cmd;

    let cmd = match core::try_dispatch(cmd) {
        Ok(result) => return result,
        Err(cmd) => cmd,
    };

    let cmd = match env::try_dispatch(cmd) {
        Ok(result) => return result,
        Err(cmd) => cmd,
    };

    let cmd = match dashboard::try_dispatch(cmd) {
        Ok(result) => return result,
        Err(cmd) => cmd,
    };

    misc::dispatch(cmd)
}
