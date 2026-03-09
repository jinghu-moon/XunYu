use std::io::{self, BufRead};

use crate::security::audit::audit_file_path;

use super::report::AuditEntry;

pub(super) fn load_audit_entries_for_tx(tx: &str) -> Vec<AuditEntry> {
    let p = audit_file_path();
    let f = match std::fs::File::open(&p) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let rdr = io::BufReader::new(f);
    let mut out = Vec::new();
    for line in rdr.lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let action = v.get("action").and_then(|v| v.as_str()).unwrap_or("");
        if !action.starts_with("redirect_") {
            continue;
        }
        let params = v
            .get("params_json")
            .cloned()
            .or_else(|| v.get("params").cloned())
            .unwrap_or(serde_json::Value::Null);
        if !params_matches_tx(&params, tx) {
            continue;
        }
        out.push(AuditEntry {
            action: action.to_string(),
            target: v
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            params,
            result: v
                .get("result")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            reason: v
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }
    out
}

fn parse_tx_from_params_text(params: &str) -> Option<String> {
    let idx = params.find("tx=")?;
    let rest = &params[(idx + 3)..];
    let end = rest.find(' ').unwrap_or(rest.len());
    let tx = rest[..end].trim();
    if tx.is_empty() {
        None
    } else {
        Some(tx.to_string())
    }
}

fn params_matches_tx(params: &serde_json::Value, tx: &str) -> bool {
    match params {
        serde_json::Value::Object(map) => map
            .get("tx")
            .and_then(|v| v.as_str())
            .map(|v| v == tx)
            .unwrap_or(false),
        serde_json::Value::String(s) => parse_tx_from_params_text(s).as_deref() == Some(tx),
        _ => false,
    }
}

pub(super) fn parse_dst_copy_from_params(params: &serde_json::Value) -> Option<(String, bool)> {
    match params {
        serde_json::Value::Object(map) => {
            let dst = map.get("dst").and_then(|v| v.as_str())?.trim();
            if dst.is_empty() {
                return None;
            }
            let copy = match map.get("copy") {
                Some(serde_json::Value::Bool(b)) => *b,
                Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0) != 0,
                Some(serde_json::Value::String(s)) => matches!(
                    s.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                ),
                _ => false,
            };
            Some((dst.to_string(), copy))
        }
        serde_json::Value::String(s) => parse_dst_copy_from_params_text(s),
        _ => None,
    }
}

fn parse_dst_copy_from_params_text(params: &str) -> Option<(String, bool)> {
    // Expected format: "tx=<id> dst=<path> copy=<bool>"
    // dst may contain spaces; copy is typically the last key.
    let copy_idx = find_bool_key_from_end(params, "copy")?;
    let copy_raw = params[(copy_idx + "copy=".len())..].trim();
    let copy = match copy_raw.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => return None,
    };

    let dst_idx = find_key(params, "dst=")?;
    let dst_start = dst_idx + "dst=".len();
    if dst_start >= copy_idx {
        return None;
    }
    let dst = params[dst_start..copy_idx].trim().to_string();
    if dst.is_empty() {
        return None;
    }
    Some((dst, copy))
}

fn find_key(params: &str, key: &str) -> Option<usize> {
    for (idx, _) in params.match_indices(key) {
        if idx == 0 {
            return Some(idx);
        }
        if params
            .as_bytes()
            .get(idx - 1)
            .map(|b| b.is_ascii_whitespace())
            .unwrap_or(false)
        {
            return Some(idx);
        }
    }
    None
}

fn find_bool_key_from_end(params: &str, key: &str) -> Option<usize> {
    let needle = format!("{key}=");
    let mut last_valid: Option<usize> = None;
    for (idx, _) in params.match_indices(&needle) {
        if idx == 0
            || params
                .as_bytes()
                .get(idx - 1)
                .map(|b| b.is_ascii_whitespace())
                .unwrap_or(false)
        {
            last_valid = Some(idx);
        }
    }
    last_valid
}
