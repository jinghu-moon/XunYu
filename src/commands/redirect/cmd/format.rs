use crate::model::{ListFormat, parse_list_format};
use crate::output::{CliResult, prefer_table_output};

use super::super::errors::invalid_format_err;

pub(super) fn resolve_format(format_raw: &str) -> CliResult<ListFormat> {
    let mut format = parse_list_format(format_raw).ok_or_else(|| invalid_format_err(format_raw))?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }
    Ok(format)
}
