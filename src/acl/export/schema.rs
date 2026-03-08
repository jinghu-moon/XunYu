use serde::{Deserialize, Serialize};

use crate::acl::types::AclSnapshot;

#[derive(Debug, Serialize, Deserialize)]
pub struct AclBackup {
    pub version: u32,
    pub created_at: String,
    pub original_path: String,
    pub acl: AclSnapshot,
}

pub(super) const BACKUP_VERSION: u32 = 1;
