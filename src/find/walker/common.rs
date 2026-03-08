use std::fs;
use std::path::{Path, PathBuf};

use super::super::filters::{EmptyFilterMode, FindFilters};
use super::super::matcher::MatchDecision;
use super::super::rules::RuleKind;

#[derive(Clone, Debug)]
pub(crate) struct ScanItem {
    pub(crate) base_dir: String,
    pub(crate) rel_path: String,
    pub(crate) is_dir: bool,
    pub(crate) depth: i32,
    pub(crate) size: Option<u64>,
    pub(crate) mtime: Option<i64>,
    pub(crate) rule_idx: Option<usize>,
    pub(crate) final_state: RuleKind,
    pub(crate) explicit: bool,
}

pub(crate) struct ScanOutput {
    pub(crate) items: Vec<ScanItem>,
    pub(crate) count: usize,
}

pub(super) struct EntryOutcome {
    pub(super) next_dir: Option<(PathBuf, RuleKind)>,
    pub(super) item: Option<ScanItem>,
    pub(super) count_inc: bool,
}

pub(super) fn should_prune_dir(decision: &MatchDecision) -> bool {
    decision.explicit && decision.final_state == RuleKind::Exclude
}

pub(super) fn passes_empty_filter(
    filters: &FindFilters,
    is_dir: bool,
    path: &Path,
    size: Option<u64>,
) -> bool {
    if is_dir {
        match filters.empty_dirs {
            EmptyFilterMode::None => true,
            EmptyFilterMode::Only => is_dir_empty(path),
            EmptyFilterMode::Exclude => !is_dir_empty(path),
        }
    } else {
        match filters.empty_files {
            EmptyFilterMode::None => true,
            EmptyFilterMode::Only => size.unwrap_or(1) == 0,
            EmptyFilterMode::Exclude => size.unwrap_or(0) != 0,
        }
    }
}

fn is_dir_empty(path: &Path) -> bool {
    let Ok(mut rd) = fs::read_dir(path) else {
        return false;
    };
    rd.next().is_none()
}

pub(super) fn rel_path(base: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(base).unwrap_or(path);
    let mut s = rel.to_string_lossy().replace('\\', "/");
    if s.starts_with("./") {
        s = s.trim_start_matches("./").to_string();
    }
    if s.starts_with('/') {
        s = s.trim_start_matches('/').to_string();
    }
    s
}

pub(super) fn build_rel_path<'a>(prefix: &str, name: &str, buffer: &'a mut String) -> &'a str {
    buffer.clear();
    if !prefix.is_empty() {
        buffer.push_str(prefix);
        buffer.push('/');
    }
    buffer.push_str(name);
    buffer.as_str()
}

pub(super) fn resolve_base_root(base_raw: &str, base_path: &Path) -> (PathBuf, String) {
    if base_path.is_file() {
        if let Some(parent) = base_path.parent() {
            return (parent.to_path_buf(), parent.to_string_lossy().into_owned());
        }
    }
    (base_path.to_path_buf(), base_raw.to_string())
}
