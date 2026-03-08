use std::fs;
use std::path::Path;

use crate::model::{ListFormat, parse_list_format};
use crate::output::{CliError, prefer_table_output};

pub(super) fn ensure_parent_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

pub(super) fn resolve_ctx_list_format(raw: &str) -> Result<ListFormat, CliError> {
    let mut format = parse_list_format(raw).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", raw),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }
    Ok(format)
}
