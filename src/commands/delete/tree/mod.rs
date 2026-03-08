use std::path::PathBuf;

mod execute;
mod plan;
mod render;
mod walk;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NodeKind {
    Dir,
    File,
    TargetFile,
    ExcludedDir,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CheckState {
    Checked,
    Unchecked,
    Indeterminate,
}

#[derive(Debug, Clone)]
pub(crate) struct TreeNode {
    pub(crate) id: usize,
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) kind: NodeKind,
    pub(crate) depth: usize,
    pub(crate) expanded: bool,
    pub(crate) check: CheckState,
    pub(crate) children: Vec<usize>,
    pub(crate) parent: Option<usize>,
    pub(crate) size: Option<u64>,
    pub(crate) target_count: usize,
}

pub(crate) struct FileTree {
    pub(crate) nodes: Vec<TreeNode>,
    pub(crate) cursor: usize,
    pub(crate) filter: String,
    pub(crate) filter_active: bool,
}
