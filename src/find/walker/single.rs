use std::fs;
use std::path::{Path, PathBuf};

use crate::output::{CliError, CliResult};
use crate::util::normalize_glob_path;

use super::super::filters::{
    FindFilters, attr_filter_match, depth_filter_match, needs_metadata_for_entry,
    size_filters_match, system_time_to_secs, time_filters_match,
};
use super::super::ignore::IgnoreSet;
use super::super::matcher::determine_path_state;
use super::super::rules::{CompiledRules, RuleKind};
use super::common::{ScanItem, ScanOutput, passes_empty_filter, rel_path, resolve_base_root};

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

#[cfg(not(windows))]
use super::dir_std::scan_dir_std;
#[cfg(windows)]
use super::dir_windows::scan_dir_windows;

pub(super) fn scan_single_thread(
    base_dirs: &[String],
    rules: &CompiledRules,
    filters: &FindFilters,
    force_meta: bool,
    count_only: bool,
) -> CliResult<ScanOutput> {
    let mut items = Vec::new();
    let mut count = 0usize;
    for base in base_dirs {
        scan_one_base(
            base, rules, filters, force_meta, count_only, &mut items, &mut count,
        )?;
    }
    let count = if count_only { count } else { items.len() };
    Ok(ScanOutput { items, count })
}

pub(super) fn scan_one_base(
    base: &str,
    rules: &CompiledRules,
    filters: &FindFilters,
    force_meta: bool,
    count_only: bool,
    items: &mut Vec<ScanItem>,
    count: &mut usize,
) -> CliResult {
    let base_path = PathBuf::from(base);
    if !base_path.exists() {
        return Err(CliError::new(2, format!("Path not found: {base}")));
    }

    let (base_root, base_display) = resolve_base_root(base, &base_path);
    let ignore = IgnoreSet::new(base_root.as_path());
    let inherited = if rules.default_include {
        RuleKind::Include
    } else {
        RuleKind::Exclude
    };

    if base_path.is_file() {
        scan_single_file(
            &base_path,
            &base_root,
            &base_display,
            rules,
            filters,
            force_meta,
            count_only,
            inherited,
            &ignore,
            items,
            count,
        );
        return Ok(());
    }

    let max_depth = filters.depth.as_ref().and_then(|d| d.max).unwrap_or(-1);
    let mut stack: Vec<(PathBuf, i32, RuleKind)> = vec![(base_path.clone(), 0, inherited)];
    while let Some((dir, depth, inherited_state)) = stack.pop() {
        if max_depth >= 0 && depth >= max_depth {
            continue;
        }
        let mut push_dir = |child: PathBuf, state: RuleKind| {
            stack.push((child, depth + 1, state));
        };
        let mut push_item = |item: ScanItem| {
            items.push(item);
        };
        #[cfg(windows)]
        {
            scan_dir_windows(
                &dir,
                &base_root,
                &base_display,
                rules,
                filters,
                &ignore,
                inherited_state,
                depth,
                force_meta,
                count_only,
                count,
                &mut push_dir,
                &mut push_item,
            );
        }
        #[cfg(not(windows))]
        {
            scan_dir_std(
                &dir,
                &base_root,
                &base_display,
                rules,
                filters,
                &ignore,
                inherited_state,
                depth,
                force_meta,
                count_only,
                count,
                &mut push_dir,
                &mut push_item,
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn scan_single_file(
    path: &Path,
    base_root: &Path,
    base_display: &str,
    rules: &CompiledRules,
    filters: &FindFilters,
    force_meta: bool,
    count_only: bool,
    inherited: RuleKind,
    ignore: &IgnoreSet,
    items: &mut Vec<ScanItem>,
    count: &mut usize,
) {
    let rel = rel_path(base_root, path);
    let rel_norm = normalize_glob_path(&rel);
    let name_lower = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ignore.should_ignore(&rel_norm, &name_lower, false) {
        return;
    }
    let decision = determine_path_state(rules, &rel, false, inherited);
    if decision.final_state != RuleKind::Include {
        return;
    }
    if !depth_filter_match(filters.depth.as_ref(), 1) {
        return;
    }

    let mut size = None;
    let mut mtime = None;
    let mut ctime = None;
    let mut atime = None;
    let mut attrs = 0u32;

    if needs_metadata_for_entry(filters, false) || force_meta {
        let Ok(meta) = fs::metadata(path) else {
            return;
        };
        size = Some(meta.len());
        mtime = meta.modified().ok().and_then(system_time_to_secs);
        ctime = meta.created().ok().and_then(system_time_to_secs);
        atime = meta.accessed().ok().and_then(system_time_to_secs);
        #[cfg(windows)]
        {
            attrs = meta.file_attributes();
        }
    }

    if !attr_filter_match(filters.attr.as_ref(), attrs) {
        return;
    }
    if !time_filters_match(&filters.time_filters, mtime, ctime, atime) {
        return;
    }
    if !size_filters_match(&filters.size_filters, size.unwrap_or(0)) {
        return;
    }
    if !passes_empty_filter(filters, false, path, size) {
        return;
    }

    if count_only {
        *count += 1;
        return;
    }
    items.push(ScanItem {
        base_dir: base_display.to_string(),
        rel_path: rel,
        is_dir: false,
        depth: 1,
        size,
        mtime,
        rule_idx: decision.rule_idx,
        final_state: decision.final_state,
        explicit: decision.explicit,
    });
}
