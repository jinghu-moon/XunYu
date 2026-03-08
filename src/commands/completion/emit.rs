use std::env;

use crate::output::{CliError, CliResult};

use super::debug::DebugContext;
use super::types::CompletionItem;
use super::{HARD_LIMIT, MAX_DESC_LEN, MAX_VALUE_LEN};

pub(super) fn emit_response(
    mut items: Vec<CompletionItem>,
    directive: u32,
    ext: Option<&str>,
    value_prefix: Option<&str>,
    debug: &DebugContext,
) -> CliResult {
    apply_limit(&mut items);
    let mut emitted = 0usize;
    for item in items {
        let value = if let Some(prefix) = value_prefix {
            format!("{prefix}{}", item.value)
        } else {
            item.value
        };
        if value.len() > MAX_VALUE_LEN || contains_control(&value) {
            continue;
        }
        let mut desc = item.desc;
        if desc.len() > MAX_DESC_LEN {
            desc.truncate(MAX_DESC_LEN);
        }
        if contains_control(&desc) {
            desc = sanitize_control(&desc);
        }
        if desc.is_empty() {
            out_println!("{value}");
        } else {
            out_println!("{value}\t{desc}");
        }
        emitted += 1;
    }
    let mut line = format!("__XUN_COMPLETE__=ok\tdirective={directive}");
    if let Some(ext) = ext {
        line.push_str("\text=");
        line.push_str(ext);
    }
    line.push_str("\tv=1");
    out_println!("{line}");
    debug.log(format!(
        "status=ok items={} directive={} ext={}",
        emitted,
        directive,
        ext.unwrap_or("")
    ));
    Ok(())
}

pub(super) fn emit_fallback(debug: &DebugContext, reason: &str) -> CliResult {
    out_println!("__XUN_COMPLETE__=fallback");
    debug.log(format!("status=fallback reason={reason}"));
    Err(CliError::new(1, ""))
}

fn apply_limit(items: &mut Vec<CompletionItem>) {
    let limit = env::var("XUN_COMPLETE_LIMIT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    let max_len = if limit == 0 {
        HARD_LIMIT
    } else {
        limit.min(HARD_LIMIT)
    };
    if items.len() > max_len {
        items.truncate(max_len);
    }
}

fn contains_control(s: &str) -> bool {
    s.contains('\t') || s.contains('\n') || s.contains('\r')
}

fn sanitize_control(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}
