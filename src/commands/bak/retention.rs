use std::fs;
use std::path::{Path, PathBuf};

use super::version::parse_version;

pub(crate) fn apply_retention(
    backups_root: &Path,
    prefix: &str,
    max: usize,
    batch: usize,
) -> usize {
    if max == 0 {
        return 0;
    }

    let mut items: Vec<(u32, PathBuf)> = Vec::new();
    if let Ok(rd) = fs::read_dir(backups_root) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if let Some(n) = parse_version(&name, prefix) {
                items.push((n, e.path()));
            }
        }
    }
    items.sort_by_key(|i| i.0);
    let total = items.len();
    if total <= max {
        return 0;
    }

    let overflow = total - max;
    let to_delete = overflow.max(batch).min(total - 1);
    let mut cleaned = 0;
    for (_, p) in items.iter().take(to_delete) {
        if p.is_dir() {
            let _ = fs::remove_dir_all(p);
        } else {
            let _ = fs::remove_file(p);
        }
        cleaned += 1;
    }
    cleaned
}
