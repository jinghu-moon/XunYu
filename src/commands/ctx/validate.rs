use crate::output::{CliError, CliResult};

use super::RESERVED_NAMES;

pub(super) fn validate_name(name: &str) -> CliResult {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_details(
            2,
            "Profile name is empty.".to_string(),
            &["Fix: Use `xun ctx set <name> ...`."],
        ));
    }
    if RESERVED_NAMES
        .iter()
        .any(|r| r.eq_ignore_ascii_case(trimmed))
    {
        return Err(CliError::with_details(
            2,
            format!("Invalid profile name: {trimmed}."),
            &["Fix: Choose a different name."],
        ));
    }
    Ok(())
}

pub(super) fn validate_env_key(key: &str) -> CliResult {
    if key.contains('=') || key.trim().is_empty() {
        return Err(CliError::with_details(
            2,
            format!("Invalid env key: {}", key),
            &["Fix: Use KEY=VALUE with a valid env key."],
        ));
    }
    if ["XUN_DEFAULT_TAG", "XUN_CTX", "XUN_CTX_STATE"]
        .iter()
        .any(|k| k.eq_ignore_ascii_case(key))
    {
        return Err(CliError::with_details(
            2,
            format!("Reserved env key: {}", key),
            &["Fix: Do not override XUN_CTX/XUN_DEFAULT_TAG/XUN_CTX_STATE."],
        ));
    }
    Ok(())
}
