// batch_rename/output_format.rs
//
// Serialization helpers for --output-format json/csv.

use crate::batch_rename::types::RenameOp;
use serde_json::json;

/// Serialize ops to a JSON string.
///
/// - `extra_skipped`: additional skipped count from conflict-skip strategy.
/// - noop ops (from == to) are counted as skipped and excluded from `ops`.
pub fn ops_to_json(ops: &[RenameOp], extra_skipped: usize) -> String {
    let effective: Vec<&RenameOp> = ops.iter().filter(|o| o.from != o.to).collect();
    let noop_count = ops.len() - effective.len();
    let skipped = noop_count + extra_skipped;

    let ops_json: Vec<serde_json::Value> = effective
        .iter()
        .map(|o| {
            json!({
                "from": o.from.to_string_lossy().as_ref(),
                "to":   o.to.to_string_lossy().as_ref(),
            })
        })
        .collect();

    let result = json!({
        "total":     ops.len(),
        "effective": effective.len(),
        "skipped":   skipped,
        "ops":       ops_json,
    });

    result.to_string()
}

/// Serialize ops to CSV (from,to header + one row per effective op).
pub fn ops_to_csv(ops: &[RenameOp]) -> String {
    let mut out = String::from("from,to\n");
    for op in ops.iter().filter(|o| o.from != o.to) {
        let from = op.from.to_string_lossy();
        let to = op.to.to_string_lossy();
        out.push_str(&csv_field(&from));
        out.push(',');
        out.push_str(&csv_field(&to));
        out.push('\n');
    }
    out
}

fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}
