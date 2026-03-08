use super::entry::FastEntry;
use super::*;

pub(super) fn evaluate_entry_fast(
    dir: &Path,
    entry: &FastEntry,
    rel: &str,
    base_display: &str,
    rules: &CompiledRules,
    filters: &FindFilters,
    ignore: &IgnoreSet,
    inherited_state: RuleKind,
    depth: i32,
    force_meta: bool,
    count_only: bool,
) -> EntryOutcome {
    if !ignore.is_empty() {
        let rel_norm = normalize_glob_path(rel);
        let name_lower = entry.name.to_ascii_lowercase();
        if ignore.should_ignore(&rel_norm, &name_lower, entry.is_dir) {
            return EntryOutcome {
                next_dir: None,
                item: None,
                count_inc: false,
            };
        }
    }

    let decision = determine_path_state(rules, rel, entry.is_dir, inherited_state);
    let depth_val = depth + 1;
    let need_dir_path = entry.is_dir
        && (filters.empty_dirs != EmptyFilterMode::None || !should_prune_dir(&decision));
    let dir_path = if need_dir_path {
        Some(dir.join(&entry.name))
    } else {
        None
    };

    if decision.final_state != RuleKind::Include {
        return EntryOutcome {
            next_dir: if entry.is_dir && !should_prune_dir(&decision) {
                dir_path.map(|p| (p, decision.final_state))
            } else {
                None
            },
            item: None,
            count_inc: false,
        };
    }
    if !depth_filter_match(filters.depth.as_ref(), depth_val) {
        return EntryOutcome {
            next_dir: if entry.is_dir && !should_prune_dir(&decision) {
                dir_path.map(|p| (p, decision.final_state))
            } else {
                None
            },
            item: None,
            count_inc: false,
        };
    }

    let need_meta = needs_metadata_for_entry(filters, entry.is_dir) || force_meta;
    let (size, mtime, ctime, atime, attrs) = if need_meta {
        (
            Some(entry.size),
            entry.mtime,
            entry.ctime,
            entry.atime,
            entry.attrs,
        )
    } else {
        (None, None, None, None, 0)
    };

    if !attr_filter_match(filters.attr.as_ref(), attrs) {
        return EntryOutcome {
            next_dir: if entry.is_dir && !should_prune_dir(&decision) {
                dir_path.map(|p| (p, decision.final_state))
            } else {
                None
            },
            item: None,
            count_inc: false,
        };
    }
    if !time_filters_match(&filters.time_filters, mtime, ctime, atime) {
        return EntryOutcome {
            next_dir: if entry.is_dir && !should_prune_dir(&decision) {
                dir_path.map(|p| (p, decision.final_state))
            } else {
                None
            },
            item: None,
            count_inc: false,
        };
    }
    if !entry.is_dir && !size_filters_match(&filters.size_filters, size.unwrap_or(0)) {
        return EntryOutcome {
            next_dir: if entry.is_dir && !should_prune_dir(&decision) {
                dir_path.map(|p| (p, decision.final_state))
            } else {
                None
            },
            item: None,
            count_inc: false,
        };
    }
    if entry.is_dir {
        if filters.empty_dirs != EmptyFilterMode::None {
            let Some(path) = dir_path.as_ref() else {
                return EntryOutcome {
                    next_dir: None,
                    item: None,
                    count_inc: false,
                };
            };
            if !passes_empty_filter(filters, entry.is_dir, path, size) {
                return EntryOutcome {
                    next_dir: None,
                    item: None,
                    count_inc: false,
                };
            }
        }
    } else if !passes_empty_filter(filters, false, Path::new(""), size) {
        return EntryOutcome {
            next_dir: None,
            item: None,
            count_inc: false,
        };
    }

    let next_dir = if entry.is_dir && !should_prune_dir(&decision) {
        dir_path.map(|p| (p, decision.final_state))
    } else {
        None
    };

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
            is_dir: entry.is_dir,
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
