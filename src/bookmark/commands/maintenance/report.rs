use crate::model::{ListFormat, parse_list_format};
use crate::output::CliError;
use crate::output::prefer_table_output;

pub(super) fn resolve_output_format(raw: &str) -> Result<ListFormat, CliError> {
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
