use std::collections::HashSet;
use std::path::PathBuf;

use regex::Regex;

use crate::output::emit_warning;
use crate::path_guard::string_check::reserved_names as path_guard_reserved_names;
use crate::util::split_csv;

use super::DEFAULT_EXCLUDES;
use super::scanner;

pub(super) fn reserved_names() -> HashSet<String> {
    path_guard_reserved_names()
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

pub(super) fn parse_name_filter(raw: &[String]) -> HashSet<String> {
    split_csv(raw)
        .into_iter()
        .map(|s| s.to_lowercase())
        .collect()
}

pub(super) fn build_target_names(
    reserved: &HashSet<String>,
    name_filter: &HashSet<String>,
    any: bool,
) -> (HashSet<String>, bool) {
    if any {
        if name_filter.is_empty() {
            return (HashSet::new(), true);
        }
        return (name_filter.clone(), false);
    }

    if name_filter.is_empty() {
        return (reserved.clone(), false);
    }

    let mut out = HashSet::new();
    for n in name_filter {
        if reserved.contains(n) {
            out.insert(n.clone());
        }
    }
    (out, false)
}

pub(super) fn build_exclude_dirs(raw: &[String], no_default: bool) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
    if !no_default {
        for d in DEFAULT_EXCLUDES {
            out.insert((*d).to_string());
        }
    }
    for d in split_csv(raw) {
        out.insert(d.to_lowercase());
    }
    out
}

pub(super) fn filter_direct_files(
    files: Vec<PathBuf>,
    target_names: &HashSet<String>,
    match_all: bool,
    patterns: &[Regex],
) -> Vec<PathBuf> {
    if !match_all && target_names.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for path in files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_lowercase());
        let Some(name) = name else {
            continue;
        };
        if !match_all && !target_names.contains(&name) {
            emit_warning(format!("Skipped non-target file: {}", path.display()), &[]);
            continue;
        }
        if scanner::matches_any(path.to_string_lossy().as_ref(), patterns) {
            continue;
        }
        out.push(path);
    }
    out
}
