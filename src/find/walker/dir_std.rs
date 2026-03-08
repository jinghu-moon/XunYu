use std::fs;
use std::path::{Path, PathBuf};

use crate::util::normalize_glob_path;

use super::super::filters::{
    FindFilters, attr_filter_match, depth_filter_match, needs_metadata_for_entry,
    size_filters_match, system_time_to_secs, time_filters_match,
};
use super::super::ignore::IgnoreSet;
use super::super::matcher::determine_path_state;
use super::super::rules::{CompiledRules, RuleKind};
use super::common::{
    EntryOutcome, ScanItem, build_rel_path, passes_empty_filter, rel_path, should_prune_dir,
};

pub(super) fn evaluate_entry(
    entry: &fs::DirEntry,
    rel: &str,
    name: &str,
    base_display: &str,
    rules: &CompiledRules,
    filters: &FindFilters,
    ignore: &IgnoreSet,
    inherited_state: RuleKind,
    depth: i32,
    force_meta: bool,
    count_only: bool,
) -> EntryOutcome {
    let ft = match entry.file_type() {
        Ok(v) => v,
        Err(_) => {
            return EntryOutcome {
                next_dir: None,
                item: None,
                count_inc: false,
            };
        }
    };
    let is_dir = ft.is_dir();
    let path = entry.path();
    if !ignore.is_empty() {
        let rel_norm = normalize_glob_path(rel);
        let name_lower = name.to_ascii_lowercase();
        if ignore.should_ignore(&rel_norm, &name_lower, is_dir) {
            return EntryOutcome {
                next_dir: None,
                item: None,
                count_inc: false,
            };
        }
    }

    let decision = determine_path_state(rules, rel, is_dir, inherited_state);
    let depth_val = depth + 1;
    let next_dir = if is_dir && !should_prune_dir(&decision) {
        Some((path.clone(), decision.final_state))
    } else {
        None
    };

    if decision.final_state != RuleKind::Include {
        return EntryOutcome {
            next_dir,
            item: None,
            count_inc: false,
        };
    }
    if !depth_filter_match(filters.depth.as_ref(), depth_val) {
        return EntryOutcome {
            next_dir,
            item: None,
            count_inc: false,
        };
    }

    let mut size = None;
    let mut mtime = None;
    let mut ctime = None;
    let mut atime = None;
    let mut attrs = 0u32;

    if needs_metadata_for_entry(filters, is_dir) || force_meta {
        let Ok(meta) = entry.metadata() else {
            return EntryOutcome {
                next_dir,
                item: None,
                count_inc: false,
            };
        };
        size = Some(meta.len());
        mtime = meta.modified().ok().and_then(system_time_to_secs);
        ctime = meta.created().ok().and_then(system_time_to_secs);
        atime = meta.accessed().ok().and_then(system_time_to_secs);
    }

    if !attr_filter_match(filters.attr.as_ref(), attrs) {
        return EntryOutcome {
            next_dir,
            item: None,
            count_inc: false,
        };
    }
    if !time_filters_match(&filters.time_filters, mtime, ctime, atime) {
        return EntryOutcome {
            next_dir,
            item: None,
            count_inc: false,
        };
    }
    if !is_dir && !size_filters_match(&filters.size_filters, size.unwrap_or(0)) {
        return EntryOutcome {
            next_dir,
            item: None,
            count_inc: false,
        };
    }
    if !passes_empty_filter(filters, is_dir, &path, size) {
        return EntryOutcome {
            next_dir,
            item: None,
            count_inc: false,
        };
    }

    if count_only {
        return EntryOutcome {
            next_dir,
            item: None,
            count_inc: true,
        };
    }

    EntryOutcome {
        next_dir,
        item: Some(ScanItem {
            base_dir: base_display.to_string(),
            rel_path: rel.to_string(),
            is_dir,
            depth: depth_val,
            size,
            mtime,
            rule_idx: decision.rule_idx,
            final_state: decision.final_state,
            explicit: decision.explicit,
        }),
        count_inc: true,
    }
}

pub(super) fn scan_dir_std(
    dir: &Path,
    base_root: &Path,
    base_display: &str,
    rules: &CompiledRules,
    filters: &FindFilters,
    ignore: &IgnoreSet,
    inherited_state: RuleKind,
    depth: i32,
    force_meta: bool,
    count_only: bool,
    count: &mut usize,
    on_dir: &mut dyn FnMut(PathBuf, RuleKind),
    on_item: &mut dyn FnMut(ScanItem),
) {
    let dir_rel = rel_path(base_root, dir);
    let mut rel_buf = String::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = build_rel_path(&dir_rel, &name, &mut rel_buf);
        let outcome = evaluate_entry(
            &entry,
            rel,
            &name,
            base_display,
            rules,
            filters,
            ignore,
            inherited_state,
            depth,
            force_meta,
            count_only,
        );
        if let Some((child, state)) = outcome.next_dir {
            on_dir(child, state);
        }
        if outcome.count_inc {
            if count_only {
                *count += 1;
            } else if let Some(item) = outcome.item {
                on_item(item);
            }
        }
    }
}
