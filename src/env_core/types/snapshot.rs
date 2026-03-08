use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub name: String,
    pub raw_value: String,
    pub reg_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub description: String,
    pub created_at: String,
    #[serde(default)]
    pub user_vars: Vec<SnapshotEntry>,
    #[serde(default)]
    pub system_vars: Vec<SnapshotEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub id: String,
    pub description: String,
    pub created_at: String,
    pub path: PathBuf,
}
