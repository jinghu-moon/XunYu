mod apply;
mod detect;
mod format;
mod state;

pub(crate) use apply::{cmd_proxy, cmd_proxy_exec, cmd_proxy_off, cmd_proxy_on};
pub(crate) use detect::cmd_proxy_status;
