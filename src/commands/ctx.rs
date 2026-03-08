mod cmd;
mod common;
mod env;
mod proxy;
mod session;
mod validate;

pub(crate) use cmd::cmd_ctx;

const DEFAULT_NOPROXY: &str = "localhost,127.0.0.1,::1,.local";

const RESERVED_NAMES: &[&str] = &[
    "set", "use", "off", "list", "show", "del", "delete", "rename", "help",
];
