use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_ATTRIBUTE_SYSTEM,
};

use crate::output::{CliError, CliResult};

use super::types::AttrFilter;

pub(super) fn parse_attr_filter(expr: &str) -> CliResult<AttrFilter> {
    let raw = expr.trim();
    if raw.is_empty() {
        return Err(CliError::new(2, "Empty --attribute value."));
    }
    let mut required: u32 = 0;
    let mut forbidden: u32 = 0;
    for token in raw.split(|c: char| c == ',' || c.is_whitespace()) {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        let mut chars = t.chars();
        let sign = chars
            .next()
            .ok_or_else(|| CliError::new(2, "Invalid attribute token."))?;
        if sign != '+' && sign != '-' {
            return Err(CliError::new(2, format!("Invalid attribute token: {t}")));
        }
        let attr = chars
            .next()
            .ok_or_else(|| CliError::new(2, format!("Invalid attribute token: {t}")))?;
        if chars.next().is_some() {
            return Err(CliError::new(2, format!("Invalid attribute token: {t}")));
        }
        let mask = match attr.to_ascii_lowercase() {
            'h' => FILE_ATTRIBUTE_HIDDEN,
            'r' => FILE_ATTRIBUTE_READONLY,
            's' => FILE_ATTRIBUTE_SYSTEM,
            'l' => FILE_ATTRIBUTE_REPARSE_POINT,
            _ => return Err(CliError::new(2, format!("Unknown attribute: {attr}"))),
        };
        if sign == '+' {
            required |= mask;
        } else {
            forbidden |= mask;
        }
    }
    if required & forbidden != 0 {
        return Err(CliError::new(
            2,
            "Attribute conflict: cannot be both required and forbidden.",
        ));
    }
    Ok(AttrFilter {
        required,
        forbidden,
    })
}
