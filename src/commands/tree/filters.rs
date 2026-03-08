use std::path::Path;

use crate::util::matches_patterns;

use super::types::{SortKey, TreeFilters};

pub(super) fn is_version_dir(name: &str) -> bool {
    name.starts_with('v')
        && name.len() > 1
        && name[1..]
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_digit())
}

pub(super) fn parse_sort(raw: &str) -> Option<SortKey> {
    match raw.to_lowercase().as_str() {
        "name" => Some(SortKey::Name),
        "mtime" => Some(SortKey::Mtime),
        "size" => Some(SortKey::Size),
        _ => None,
    }
}

pub(super) fn needs_rel(filters: &TreeFilters) -> bool {
    !filters.exclude_paths.is_empty()
        || !filters.exclude_patterns.is_empty()
        || !filters.include_patterns.is_empty()
}

pub(super) fn should_exclude(
    rel: &str,
    name: &str,
    name_lower: &str,
    is_dir: bool,
    filters: &TreeFilters,
) -> bool {
    if !filters.hidden && name.starts_with('.') {
        return true;
    }
    if filters
        .exclude_names
        .iter()
        .any(|e| e.eq_ignore_ascii_case(name))
    {
        return true;
    }
    if is_dir && is_version_dir(name) {
        return true;
    }
    if !is_dir {
        if let Some(ext) = Path::new(name).extension() {
            let dot_ext = format!(".{}", ext.to_string_lossy());
            if filters
                .exclude_exts
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&dot_ext))
            {
                return true;
            }
        }
    }

    if !filters.include_patterns.is_empty()
        && matches_patterns(rel, name_lower, &filters.include_patterns, is_dir)
    {
        return false;
    }

    if filters
        .exclude_paths
        .iter()
        .any(|p| rel == p || rel.starts_with(&format!("{p}/")))
    {
        return true;
    }
    if matches_patterns(rel, name_lower, &filters.exclude_patterns, is_dir) {
        return true;
    }
    false
}
