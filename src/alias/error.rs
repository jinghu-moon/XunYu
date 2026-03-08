use crate::output::CliError;

pub(crate) fn to_cli_error(err: anyhow::Error) -> CliError {
    let mut details: Vec<String> = Vec::new();
    for cause in err.chain().skip(1) {
        details.push(format!("Cause: {cause}"));
    }
    if details.is_empty() {
        CliError::new(1, err.to_string())
    } else {
        CliError {
            code: 1,
            message: err.to_string(),
            details,
        }
    }
}
