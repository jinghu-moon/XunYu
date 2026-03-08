use console::{Key, Term};

use crate::cli::DeleteCmd;
use crate::output::{CliError, CliResult, can_interact, emit_warning};

use super::super::winapi;

pub(super) fn validate_level(level: u8) -> CliResult<u8> {
    if !(1..=6).contains(&level) {
        return Err(CliError::with_details(
            2,
            format!("Invalid level: {level}."),
            &["Fix: Use a level between 1 and 6."],
        ));
    }
    Ok(level)
}

pub(super) fn maybe_relaunch_elevated() -> CliResult<bool> {
    if winapi::is_elevated() {
        return Ok(false);
    }

    emit_warning(
        "Not running as administrator.",
        &["Hint: Levels 3-6 and USN scan require elevation."],
    );

    if !can_interact() {
        return Ok(false);
    }

    ui_println!("Relaunch elevated? [y/N]");
    let key = Term::stdout().read_key().unwrap_or(Key::Unknown);
    if matches!(key, Key::Char('y') | Key::Char('Y')) {
        let exe = std::env::current_exe()
            .map_err(|e| CliError::new(1, format!("Failed to get executable path: {e}")))?;
        let args = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
        winapi::relaunch_elevated(exe.to_string_lossy().as_ref(), &args);
        return Ok(true);
    }

    Ok(false)
}

pub(super) fn should_use_tui(args: &DeleteCmd) -> bool {
    if args.no_tui {
        return false;
    }
    if !can_interact() {
        return false;
    }
    cfg!(feature = "delete_tui")
}
