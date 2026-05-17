use crate::output::CliError;
use crate::foundation::suggest::did_you_mean;

pub(crate) fn err2(msg: impl Into<String>, hints: &[&str]) -> CliError {
    CliError::with_details(2, msg, hints)
}

pub(crate) fn invalid_format_err(raw: &str) -> CliError {
    let opts = ["auto", "table", "tsv", "json"];
    let mut details: Vec<String> = Vec::new();
    if let Some(s) = did_you_mean(raw, &opts) {
        details.push(format!("Did you mean: \"{}\"?", s));
    }
    details.push("Fix: Use one of: auto | table | tsv | json".to_string());
    CliError {
        code: 2,
        message: format!("Invalid format: {raw}"),
        details,
    }
}
