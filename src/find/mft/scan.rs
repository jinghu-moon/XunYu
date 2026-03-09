use std::path::PathBuf;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;

use crate::output::CliResult;
use crate::util::normalize_glob_path;

use super::super::filters::{
    EmptyFilterMode, FindFilters, attr_filter_match, depth_filter_match, size_filters_match,
    system_time_to_secs, time_filters_match,
};
use super::super::ignore::IgnoreSet;
use super::super::matcher::determine_path_state;
use super::super::rules::{CompiledRules, RuleKind};
use super::super::walker::{ScanItem, ScanOutput};
use super::resolve::resolve_base_ref;
use super::types::{ChildrenIndex, DfsEntry};
use super::win::{
    enumerate_mft, extract_drive_letter, is_volume_root, open_volume_handle, wide_to_string,
};

pub(crate) fn try_scan_mft(
    base_dirs: &[String],
    rules: &CompiledRules,
    filters: &FindFilters,
    force_meta: bool,
    count_only: bool,
) -> CliResult<Option<ScanOutput>> {
    if base_dirs.is_empty() {
        return Ok(None);
    }

    let mut drive_letter: Option<u8> = None;
    let mut base_infos: Vec<(PathBuf, String)> = Vec::new();
    for base in base_dirs {
        let base_path = PathBuf::from(base);
        if !base_path.is_dir() {
            return Ok(None);
        }
        let canonical = match std::fs::canonicalize(&base_path) {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        if !is_volume_root(&canonical) {
            return Ok(None);
        }
        let Some(letter) = extract_drive_letter(&canonical) else {
            return Ok(None);
        };
        if let Some(existing) = drive_letter {
            if existing != letter {
                return Ok(None);
            }
        } else {
            drive_letter = Some(letter);
        }
        base_infos.push((canonical, base.clone()));
    }

    let drive_letter = drive_letter.unwrap_or(b'C');
    let volume_handle = match open_volume_handle(drive_letter) {
        Some(h) => h,
        None => return Ok(None),
    };

    let enum_result = enumerate_mft(volume_handle);
    unsafe { CloseHandle(volume_handle) };
    let (records, pool) = match enum_result {
        Some(v) => v,
        None => return Ok(None),
    };
    if records.is_empty() {
        return Ok(None);
    }

    let children = ChildrenIndex::build(&records);

    let inherited = if rules.default_include {
        RuleKind::Include
    } else {
        RuleKind::Exclude
    };

    let mut output = ScanOutput {
        items: Vec::new(),
        count: 0,
    };
    let max_depth = filters.depth.as_ref().and_then(|d| d.max).unwrap_or(-1);

    for (base_root, base_display) in base_infos {
        let base_ref =
            resolve_base_ref(&base_root, &records, &pool, &children, rules.case_sensitive);
        if base_ref == 0 {
            return Ok(None);
        }
        let ignore = IgnoreSet::new(&base_root);

        let mut stack = Vec::new();
        stack.push(DfsEntry {
            dir_ref: base_ref,
            rel_prefix: String::new(),
            inherited,
            depth: 0,
        });

        while let Some(entry) = stack.pop() {
            if max_depth >= 0 && entry.depth >= max_depth {
                continue;
            }
            let (lo, hi) = children.find_range(entry.dir_ref);
            if lo == hi {
                continue;
            }

            for idx in lo..hi {
                let rec = records[children.record_index_at(idx)];
                let is_dir = (rec.attrs & FILE_ATTRIBUTE_DIRECTORY) != 0;
                let name = wide_to_string(pool.slice(rec.name_offset, rec.name_len));
                if name.is_empty() || name == "." || name == ".." {
                    continue;
                }

                let rel = if entry.rel_prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", entry.rel_prefix, name)
                };
                if !ignore.is_empty() {
                    let rel_norm = normalize_glob_path(&rel);
                    let name_lower = name.to_ascii_lowercase();
                    if ignore.should_ignore(&rel_norm, &name_lower, is_dir) {
                        continue;
                    }
                }

                let decision = determine_path_state(rules, &rel, is_dir, entry.inherited);
                let depth_val = entry.depth + 1;
                let next_dir = if is_dir && !should_prune_dir(&decision) {
                    Some(DfsEntry {
                        dir_ref: rec.file_ref,
                        rel_prefix: rel.clone(),
                        inherited: decision.final_state,
                        depth: depth_val,
                    })
                } else {
                    None
                };

                let mut should_output = decision.final_state == RuleKind::Include;
                if should_output && !depth_filter_match(filters.depth.as_ref(), depth_val) {
                    should_output = false;
                }

                let mut size = None;
                let mut mtime = None;
                let mut ctime = None;
                let mut atime = None;
                let mut meta_ok = true;

                let need_meta = need_fs_metadata(filters, is_dir, force_meta);
                if need_meta {
                    let full_path = base_root.join(rel.as_str());
                    if let Ok(meta) = std::fs::metadata(&full_path) {
                        size = Some(meta.len());
                        mtime = meta.modified().ok().and_then(system_time_to_secs);
                        ctime = meta.created().ok().and_then(system_time_to_secs);
                        atime = meta.accessed().ok().and_then(system_time_to_secs);
                    } else {
                        meta_ok = false;
                    }
                }

                if should_output && !attr_filter_match(filters.attr.as_ref(), rec.attrs) {
                    should_output = false;
                }
                if should_output && (!meta_ok && need_meta) {
                    should_output = false;
                }
                if should_output && !time_filters_match(&filters.time_filters, mtime, ctime, atime)
                {
                    should_output = false;
                }
                if !is_dir
                    && should_output
                    && !size_filters_match(&filters.size_filters, size.unwrap_or(0))
                {
                    should_output = false;
                }
                if should_output
                    && !passes_empty_filter_mft(filters, is_dir, rec.file_ref, size, &children)
                {
                    should_output = false;
                }

                if should_output {
                    if count_only {
                        output.count += 1;
                    } else {
                        output.items.push(ScanItem {
                            base_dir: base_display.clone(),
                            rel_path: rel,
                            is_dir,
                            depth: depth_val,
                            size,
                            mtime,
                            rule_idx: decision.rule_idx,
                            final_state: decision.final_state,
                            explicit: decision.explicit,
                        });
                    }
                }

                if let Some(next) = next_dir {
                    stack.push(next);
                }
            }
        }
    }

    if !count_only {
        output.count = output.items.len();
    }
    Ok(Some(output))
}

fn need_fs_metadata(filters: &FindFilters, is_dir: bool, force_meta: bool) -> bool {
    if force_meta || !filters.time_filters.is_empty() {
        return true;
    }
    if !is_dir {
        if !filters.size_filters.is_empty() {
            return true;
        }
        if filters.empty_files != EmptyFilterMode::None {
            return true;
        }
    }
    false
}

fn passes_empty_filter_mft(
    filters: &FindFilters,
    is_dir: bool,
    file_ref: u64,
    size: Option<u64>,
    children: &ChildrenIndex,
) -> bool {
    if is_dir {
        match filters.empty_dirs {
            EmptyFilterMode::None => true,
            EmptyFilterMode::Only => children.is_empty(file_ref),
            EmptyFilterMode::Exclude => !children.is_empty(file_ref),
        }
    } else {
        match filters.empty_files {
            EmptyFilterMode::None => true,
            EmptyFilterMode::Only => size.unwrap_or(1) == 0,
            EmptyFilterMode::Exclude => size.unwrap_or(0) != 0,
        }
    }
}

fn should_prune_dir(decision: &super::super::matcher::MatchDecision) -> bool {
    decision.explicit && decision.final_state == RuleKind::Exclude
}
