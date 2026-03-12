use crate::output::CliError;

pub(crate) fn run_command(command: &str) -> Result<(), CliError> {
    let command = command.trim();
    if command.is_empty() {
        return Err(CliError::new(2, "Command is required."));
    }
    let result = std::process::Command::new("cmd")
        .args(["/C", command])
        .spawn();
    match result {
        Ok(_) => Ok(()),
        Err(err) => Err(CliError::new(2, format!("Failed to run command: {err}"))),
    }
}

