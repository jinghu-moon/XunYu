use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{EnvScope, SnapshotEntry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvProfile {
    pub name: String,
    pub scope: EnvScope,
    pub created_at: String,
    #[serde(default)]
    pub vars: Vec<SnapshotEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvProfileMeta {
    pub name: String,
    pub scope: EnvScope,
    pub created_at: String,
    pub path: PathBuf,
    pub var_count: usize,
}
