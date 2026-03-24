use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::util::normalize_glob_path;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Filesystem,
    DirArtifact,
    XunbakArtifact,
    ZipArtifact,
    SevenZArtifact,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEntry {
    pub path: String,
    pub source_path: Option<PathBuf>,
    pub size: u64,
    pub mtime_ns: Option<u64>,
    pub created_time_ns: Option<u64>,
    pub win_attributes: u32,
    pub content_hash: Option<[u8; 32]>,
    pub kind: SourceKind,
}

impl SourceEntry {
    pub fn normalized_path(&self) -> String {
        normalize_glob_path(&self.path)
    }
}

pub(crate) fn system_time_to_unix_ns(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}

pub(crate) fn metadata_created_time_ns(metadata: &fs::Metadata) -> Option<u64> {
    metadata.created().ok().map(system_time_to_unix_ns)
}

#[cfg(windows)]
pub(crate) fn file_attributes(metadata: &fs::Metadata) -> u32 {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes()
}

#[cfg(not(windows))]
pub(crate) fn file_attributes(_metadata: &fs::Metadata) -> u32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_path_uses_forward_slashes_and_lowercase() {
        let entry = SourceEntry {
            path: r".\Src\Main.RS".to_string(),
            source_path: None,
            size: 0,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::Filesystem,
        };
        assert_eq!(entry.normalized_path(), "src/main.rs");
    }
}
