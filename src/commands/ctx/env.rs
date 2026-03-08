use std::fs;
use std::path::Path;

use crate::output::{CliError, CliResult};

use super::validate::validate_env_key;

pub(super) fn parse_env_kv(raw: &str) -> CliResult<(String, String)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_details(
            2,
            "Empty --env value.".to_string(),
            &["Fix: Use `--env KEY=VALUE`."],
        ));
    }
    let line = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let mut parts = line.splitn(2, '=');
    let key = parts.next().unwrap_or("").trim();
    let value = parts.next().unwrap_or("");
    if key.is_empty() || value.is_empty() && !line.ends_with('=') {
        return Err(CliError::with_details(
            2,
            format!("Invalid --env: {}", raw),
            &["Fix: Use `--env KEY=VALUE`."],
        ));
    }
    validate_env_key(key)?;
    Ok((key.to_string(), unquote(value.trim()).to_string()))
}

pub(super) fn load_env_file(path: &Path) -> CliResult<Vec<(String, String)>> {
    let content = fs::read_to_string(path).map_err(|e| {
        CliError::with_details(
            2,
            format!("Failed to read env file: {}", path.display()),
            &[format!("Details: {e}")],
        )
    })?;
    let mut out = Vec::new();
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("");
        if key.is_empty() || value.is_empty() && !line.ends_with('=') {
            return Err(CliError::with_details(
                2,
                format!("Invalid env line at {}:{}", path.display(), idx + 1),
                &["Fix: Use KEY=VALUE per line (comments start with #)."],
            ));
        }
        validate_env_key(key)?;
        out.push((key.to_string(), unquote(value.trim()).to_string()));
    }
    Ok(out)
}

fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &value[1..value.len() - 1];
        }
    }
    value
}
