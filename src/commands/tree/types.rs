use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SortKey {
    Name,
    Mtime,
    Size,
}

pub(super) struct TreeFilters {
    pub(super) hidden: bool,
    pub(super) exclude_names: Vec<String>,
    pub(super) exclude_paths: Vec<String>,
    pub(super) exclude_exts: Vec<String>,
    pub(super) exclude_patterns: Vec<String>,
    pub(super) include_patterns: Vec<String>,
}

pub(super) struct TreeItem {
    pub(super) path: PathBuf,
    pub(super) name: String,
    pub(super) is_dir: bool,
    pub(super) mtime: u64,
    pub(super) size: u64,
}

pub(super) enum TreeOutput<'a> {
    Buffer(&'a mut Vec<String>),
    Stream,
}
