use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::collect::{collect_items, dir_total_size};
use super::constants::{BRANCH_END, BRANCH_MID};
use super::format::format_bytes;
use super::types::{SortKey, TreeFilters, TreeOutput};

pub(super) fn build_tree_inner(
    dir: &Path,
    prefix: &mut String,
    depth: usize,
    max_depth: usize,
    root: &Path,
    filters: &TreeFilters,
    sort: SortKey,
    fast: bool,
    show_size: bool,
    plain: bool,
    count: &mut usize,
    max_items: Option<usize>,
    output: &mut TreeOutput,
    size_memo: &mut HashMap<PathBuf, u64>,
) {
    if max_depth > 0 && depth > max_depth {
        return;
    }
    if let Some(max) = max_items {
        if *count >= max {
            return;
        }
    }

    let items = collect_items(dir, root, filters, sort, fast, show_size);
    let total = items.len();
    for (i, item) in items.into_iter().enumerate() {
        if let Some(max) = max_items {
            if *count >= max {
                break;
            }
        }
        let is_last = i + 1 == total;
        let (branch, child_prefix) = if plain {
            ("", "  ")
        } else if is_last {
            (BRANCH_END, "    ")
        } else {
            (BRANCH_MID, "\u{2502}   ")
        };

        if item.is_dir {
            let line = if show_size {
                let sz = dir_total_size(&item.path, depth + 1, max_depth, root, filters, size_memo);
                format!("{prefix}{branch} [{}] {}/", format_bytes(sz), item.name)
            } else {
                format!("{prefix}{branch} {}/", item.name)
            };
            match output {
                TreeOutput::Buffer(lines) => lines.push(line),
                TreeOutput::Stream => out_println!("{line}"),
            }
            *count += 1;

            let prev_len = prefix.len();
            prefix.push_str(child_prefix);
            build_tree_inner(
                &item.path,
                prefix,
                depth + 1,
                max_depth,
                root,
                filters,
                sort,
                fast,
                show_size,
                plain,
                count,
                max_items,
                output,
                size_memo,
            );
            prefix.truncate(prev_len);
        } else {
            let line = if show_size {
                format!(
                    "{prefix}{branch} [{}] {}",
                    format_bytes(item.size),
                    item.name
                )
            } else {
                format!("{prefix}{branch} {}", item.name)
            };
            match output {
                TreeOutput::Buffer(lines) => lines.push(line),
                TreeOutput::Stream => out_println!("{line}"),
            }
            *count += 1;
        }
    }
}

pub(super) fn count_tree_inner(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    root: &Path,
    filters: &TreeFilters,
    count: &mut usize,
    max_items: Option<usize>,
) {
    if max_depth > 0 && depth > max_depth {
        return;
    }
    if let Some(max) = max_items {
        if *count >= max {
            return;
        }
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let need_rel = super::filters::needs_rel(filters);
    for e in entries.flatten() {
        if let Some(max) = max_items {
            if *count >= max {
                break;
            }
        }
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
        if super::filters::should_exclude(&rel, &name, &name_lower, is_dir, filters) {
            continue;
        }

        *count += 1;
        if is_dir {
            count_tree_inner(&path, depth + 1, max_depth, root, filters, count, max_items);
        }
    }
}
