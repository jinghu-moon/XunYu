use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use super::super::types::{EnvError, EnvResult, EnvScope, ParsedImport, ParsedImportVar};

pub(super) fn parse_import_file(path: &Path) -> EnvResult<ParsedImport> {
    let content = std::fs::read_to_string(path)?;
    parse_import_content(&content)
}

pub(super) fn parse_import_content(content: &str) -> EnvResult<ParsedImport> {
    let trimmed = content.trim_start();
    if trimmed.starts_with('{') {
        parse_json_import(content)
    } else if trimmed.starts_with("Windows Registry Editor") {
        parse_reg_import(content)
    } else if looks_like_csv(trimmed) {
        parse_csv_import(content)
    } else {
        parse_env_import(content)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum JsonImportEnvelope {
    Wrapped {
        scope: Option<EnvScope>,
        vars: HashMap<String, serde_json::Value>,
    },
    Flat(HashMap<String, serde_json::Value>),
}

fn parse_json_import(content: &str) -> EnvResult<ParsedImport> {
    let mut vars = Vec::new();
    let mut scope_hint = None;
    let parsed: JsonImportEnvelope = serde_json::from_str(content)?;
    let map = match parsed {
        JsonImportEnvelope::Wrapped { scope, vars: map } => {
            scope_hint = scope;
            map
        }
        JsonImportEnvelope::Flat(map) => map,
    };

    for (name, value) in map {
        if name.is_empty() {
            continue;
        }
        let value = match value {
            serde_json::Value::String(s) => s,
            other => other.to_string(),
        };
        let reg_type = infer_reg_type(&name, &value);
        vars.push(ParsedImportVar {
            name,
            value,
            reg_type,
        });
    }
    Ok(ParsedImport {
        format: "json".to_string(),
        scope_hint,
        vars,
    })
}

fn parse_env_import(content: &str) -> EnvResult<ParsedImport> {
    let mut vars = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let raw = line.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        let Some(eq) = raw.find('=') else {
            return Err(EnvError::InvalidInput(format!(
                ".env line {} is not KEY=VALUE",
                idx + 1
            )));
        };
        let name = raw[..eq].trim().to_string();
        if name.is_empty() {
            return Err(EnvError::InvalidInput(format!(
                ".env line {} has empty key",
                idx + 1
            )));
        }
        let value = strip_wrapping_quotes(raw[eq + 1..].trim()).to_string();
        let reg_type = infer_reg_type(&name, &value);
        vars.push(ParsedImportVar {
            name,
            value,
            reg_type,
        });
    }

    Ok(ParsedImport {
        format: "env".to_string(),
        scope_hint: None,
        vars,
    })
}

fn parse_reg_import(content: &str) -> EnvResult<ParsedImport> {
    let mut vars = Vec::new();
    let mut scope_hint = None;
    for line in content.lines().map(str::trim) {
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            let upper = line.to_uppercase();
            if upper.contains("HKEY_CURRENT_USER") {
                scope_hint = Some(EnvScope::User);
            } else if upper.contains("HKEY_LOCAL_MACHINE") {
                scope_hint = Some(EnvScope::System);
            }
            continue;
        }
        if let Some(v) = parse_reg_sz_line(line) {
            vars.push(v);
            continue;
        }
        if let Some(v) = parse_reg_expand_line(line) {
            vars.push(v);
        }
    }

    Ok(ParsedImport {
        format: "reg".to_string(),
        scope_hint,
        vars,
    })
}

fn parse_csv_import(content: &str) -> EnvResult<ParsedImport> {
    let mut vars = Vec::new();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());
    let headers = reader.headers().cloned().map_err(EnvError::Csv)?;
    let idx_name = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("name"))
        .ok_or_else(|| EnvError::InvalidInput("csv missing name column".to_string()))?;
    let idx_value = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("value"))
        .ok_or_else(|| EnvError::InvalidInput("csv missing value column".to_string()))?;
    let idx_type = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("reg_type"));

    for row in reader.records() {
        let row = row?;
        let name = row.get(idx_name).unwrap_or("").trim().to_string();
        let value = row.get(idx_value).unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        let reg_type = idx_type
            .and_then(|i| row.get(i))
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or_else(|| infer_reg_type(&name, &value));
        vars.push(ParsedImportVar {
            name,
            value,
            reg_type,
        });
    }

    Ok(ParsedImport {
        format: "csv".to_string(),
        scope_hint: None,
        vars,
    })
}

fn parse_reg_sz_line(line: &str) -> Option<ParsedImportVar> {
    if !line.starts_with('"') || !line.contains("\"=\"") {
        return None;
    }
    let name_end = line.find("\"=\"")?;
    let name = line[1..name_end].to_string();
    let raw = line[name_end + 3..].trim();
    let value = raw
        .trim_start_matches('"')
        .trim_end_matches('"')
        .replace("\\\"", "\"")
        .replace("\\\\", "\\");
    Some(ParsedImportVar {
        name,
        value,
        reg_type: 1,
    })
}

fn parse_reg_expand_line(line: &str) -> Option<ParsedImportVar> {
    if !line.starts_with('"') || !line.contains("=hex(2):") {
        return None;
    }
    let name_end = line.find("\"=hex(2):")?;
    let name = line[1..name_end].to_string();
    let hex = &line[name_end + 9..];
    let bytes: Vec<u8> = hex
        .split(',')
        .filter_map(|h| u8::from_str_radix(h.trim(), 16).ok())
        .collect();
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .take_while(|u| *u != 0)
        .collect();
    let value = String::from_utf16_lossy(&units);
    Some(ParsedImportVar {
        name,
        value,
        reg_type: 2,
    })
}

fn looks_like_csv(content: &str) -> bool {
    let first = content.lines().next().unwrap_or("").trim();
    first.eq_ignore_ascii_case("name,value")
        || first.eq_ignore_ascii_case("name,value,reg_type")
        || first.eq_ignore_ascii_case("name,value,reg_type,scope")
}

fn strip_wrapping_quotes(raw: &str) -> &str {
    if raw.len() >= 2
        && ((raw.starts_with('"') && raw.ends_with('"'))
            || (raw.starts_with('\'') && raw.ends_with('\'')))
    {
        &raw[1..raw.len() - 1]
    } else {
        raw
    }
}

fn infer_reg_type(name: &str, value: &str) -> u32 {
    if name.eq_ignore_ascii_case("PATH") || value.contains('%') {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_flat() {
        let parsed = parse_import_content(r#"{"A":"1","B":"2"}"#).expect("json");
        assert_eq!(parsed.format, "json");
        assert_eq!(parsed.vars.len(), 2);
    }

    #[test]
    fn parse_env_file() {
        let parsed = parse_import_content("A=1\nB='2'\n").expect("env");
        assert_eq!(parsed.format, "env");
        assert_eq!(parsed.vars[1].value, "2");
    }

    #[test]
    fn parse_csv_file() {
        let parsed = parse_import_content("name,value\nA,1\n").expect("csv");
        assert_eq!(parsed.format, "csv");
        assert_eq!(parsed.vars[0].name, "A");
    }
}
