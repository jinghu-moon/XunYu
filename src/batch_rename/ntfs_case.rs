// batch_rename/ntfs_case.rs
//
// On Windows NTFS, renaming "photo.JPG" to "photo.jpg" is a no-op because
// the filesystem is case-insensitive. We detect such case-only renames and
// inject a temporary intermediate step to force the change.

use std::path::PathBuf;

use crate::batch_rename::types::RenameOp;

/// Scan ops for case-only renames and expand them into two-step ops via a tmp name.
pub fn normalize_case_ops(ops: Vec<RenameOp>) -> Vec<RenameOp> {
    let mut result: Vec<RenameOp> = Vec::with_capacity(ops.len());

    for op in ops {
        if is_case_only(&op) {
            let tmp = tmp_name(&op.from);
            result.push(RenameOp {
                from: op.from.clone(),
                to: tmp.clone(),
            });
            result.push(RenameOp {
                from: tmp,
                to: op.to,
            });
        } else {
            result.push(op);
        }
    }

    result
}

/// Returns true when `from` and `to` differ only in case (same Unicode codepoints, different case).
fn is_case_only(op: &RenameOp) -> bool {
    let from_name = op.from.file_name().and_then(|n| n.to_str());
    let to_name = op.to.file_name().and_then(|n| n.to_str());
    match (from_name, to_name) {
        (Some(f), Some(t)) => f != t && f.to_lowercase() == t.to_lowercase(),
        _ => false,
    }
}

fn tmp_name(base: &std::path::Path) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = format!("__xun_brn_ntfs_tmp_{:x}__", ts);
    match base.parent() {
        Some(p) if p != std::path::Path::new("") => p.join(name),
        _ => PathBuf::from(name),
    }
}
