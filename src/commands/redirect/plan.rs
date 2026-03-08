use serde::{Deserialize, Serialize};
use std::path::Path;

pub(crate) const PLAN_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlanKind {
    Move,
    Copy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConflictAction {
    Skip,
    RenameNew,
    RenameDate,
    RenameExisting,
    Overwrite,
    Trash,
    HashDedup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FileFingerprint {
    pub(crate) size: u64,
    pub(crate) mtime_ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConflictInfo {
    pub(crate) existing: FileFingerprint,
    pub(crate) incoming: FileFingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlanItem {
    pub(crate) kind: PlanKind,
    pub(crate) src: String,
    pub(crate) dst: String,
    pub(crate) rule: String,
    pub(crate) src_fp: Option<FileFingerprint>,
    /// Optional conflict fingerprints (for human review and external tooling).
    /// Execution currently relies on `dst` existence + `conflict_action` + `src_fp` stale checks.
    pub(crate) conflict: Option<ConflictInfo>,
    pub(crate) conflict_action: Option<ConflictAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlanFile {
    pub(crate) version: u32,
    pub(crate) created_ts: u64,
    pub(crate) source: String,
    pub(crate) profile: String,
    pub(crate) items: Vec<PlanItem>,
}

pub(crate) fn path_to_string(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

pub(crate) fn fingerprint_path(path: &Path) -> Option<FileFingerprint> {
    let meta = std::fs::metadata(path).ok()?;
    let size = meta.len();
    let mtime = meta.modified().ok()?;
    let mtime_ts = mtime
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Some(FileFingerprint { size, mtime_ts })
}
