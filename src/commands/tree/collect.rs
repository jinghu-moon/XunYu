use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::filters::{needs_rel, should_exclude};
use super::types::{SortKey, TreeFilters, TreeItem};

pub(super) fn collect_items(
    dir: &Path,
    root: &Path,
    filters: &TreeFilters,
    sort: SortKey,
    fast: bool,
    show_size: bool,
) -> Vec<TreeItem> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let need_meta = (!fast && sort != SortKey::Name) || show_size;
    let need_rel = needs_rel(filters);
    let mut items = Vec::new();

    for e in entries.flatten() {
        let name_os = e.file_name();
        let name = name_os.to_string_lossy().into_owned();
        let ft = match e.file_type() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let is_dir = ft.is_dir();

        let path = e.path();
        let rel = if need_rel {
            path.strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
                .to_lowercase()
        } else {
            String::new()
        };

        let name_lower = name.to_lowercase();
        if should_exclude(&rel, &name, &name_lower, is_dir, filters) {
            continue;
        }

        let (mtime, size) = if need_meta {
            let meta = e.metadata().ok();
            let mtime = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            (mtime, size)
        } else {
            (0, 0)
        };

        items.push(TreeItem {
            path,
            name,
            is_dir,
            mtime,
            size,
        });
    }

    if !fast {
        items.sort_by(|a, b| {
            b.is_dir.cmp(&a.is_dir).then_with(|| match sort {
                SortKey::Name => a.name.cmp(&b.name),
                SortKey::Mtime => b.mtime.cmp(&a.mtime).then_with(|| a.name.cmp(&b.name)),
                SortKey::Size => b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)),
            })
        });
    }

    items
}

pub(super) fn dir_total_size(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    root: &Path,
    filters: &TreeFilters,
    memo: &mut HashMap<PathBuf, u64>,
) -> u64 {
    if let Some(v) = memo.get(dir) {
        return *v;
    }
    if max_depth > 0 && depth > max_depth {
        return 0;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        memo.insert(dir.to_path_buf(), 0);
        return 0;
    };
    let need_rel = needs_rel(filters);
    let mut sum = 0u64;

    for e in entries.flatten() {
        let ft = match e.file_type() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let path = e.path();
        let is_dir = ft.is_dir();

        let rel = if need_rel {
            path.strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
                .to_lowercase()
        } else {
            String::new()
        };
        let name = e.file_name().to_string_lossy().into_owned();
        let name_lower = name.to_lowercase();
        if should_exclude(&rel, &name, &name_lower, is_dir, filters) {
            continue;
        }

        if is_dir {
            sum = sum.saturating_add(dir_total_size(
                &path,
                depth + 1,
                max_depth,
                root,
                filters,
                memo,
            ));
        } else {
            sum = sum.saturating_add(e.metadata().ok().map(|m| m.len()).unwrap_or(0));
        }
    }

    memo.insert(dir.to_path_buf(), sum);
    sum
}
