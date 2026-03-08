use crate::cli::SubCommand;
use crate::output::CliResult;

#[cfg(feature = "cstat")]
use super::super::cstat;
#[cfg(feature = "dashboard")]
use super::super::dashboard;
#[cfg(feature = "img")]
use super::super::img;

pub(super) fn try_dispatch(cmd: SubCommand) -> Result<CliResult, SubCommand> {
    match cmd {
        #[cfg(feature = "dashboard")]
        SubCommand::Serve(a) => Ok(dashboard::cmd_serve(a)),
        #[cfg(feature = "cstat")]
        SubCommand::Cstat(a) => Ok(cstat::cmd_cstat(a)),
        #[cfg(feature = "img")]
        SubCommand::Img(a) => Ok(img::cmd_img(a)),
        other => Err(other),
    }
}
