use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffChangeKind {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSegmentDiff {
    pub segment: String,
    pub kind: DiffChangeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub name: String,
    pub kind: DiffChangeKind,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path_diff: Vec<PathSegmentDiff>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvDiff {
    #[serde(default)]
    pub added: Vec<DiffEntry>,
    #[serde(default)]
    pub removed: Vec<DiffEntry>,
    #[serde(default)]
    pub changed: Vec<DiffEntry>,
}

impl EnvDiff {
    #[cfg(feature = "tui")]
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.removed.len() + self.changed.len()
    }
}
