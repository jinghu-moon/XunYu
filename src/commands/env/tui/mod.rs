#[cfg(not(feature = "tui"))]
use crate::output::{CliError, CliResult};

#[cfg(not(feature = "tui"))]
pub(crate) fn run_env_tui() -> CliResult {
    Err(CliError::with_details(
        2,
        "tui feature is not enabled".to_string(),
        &["Fix: Run `cargo run --features tui -- env tui`."],
    ))
}

#[cfg(feature = "tui")]
mod imp;

#[cfg(feature = "tui")]
pub(crate) use imp::run_env_tui;
