/// Phase 3 — Backup, restore and CSV export helpers.
///
/// # Backup format
/// ```json
/// { "version": 1, "created_at": "2026-02-25T10:00:00+08:00",
///   "original_path": "D:\\Data", "acl": { … AclSnapshot … } }
/// ```
mod format;
mod schema;
mod writer;

use std::path::Path;

use anyhow::Result;

use crate::acl::orphan::OrphanEntry;
use crate::acl::types::DiffResult;
use crate::acl::types::{AclSnapshot, RepairStats};

#[allow(unused_imports)]
pub use schema::AclBackup;

pub fn backup_acl(snapshot: &AclSnapshot, dest: &Path) -> Result<()> {
    writer::backup_acl(snapshot, dest)
}

pub fn restore_acl(backup_path: &Path, target_path: &Path) -> Result<()> {
    writer::restore_acl(backup_path, target_path)
}

pub fn export_diff_csv(diff: &DiffResult, dest: &Path) -> Result<usize> {
    format::export_diff_csv(diff, dest)
}

pub fn export_orphans_csv(orphans: &[OrphanEntry], dest: &Path) -> Result<usize> {
    format::export_orphans_csv(orphans, dest)
}

pub fn export_repair_errors_csv(stats: &RepairStats, dest: &Path) -> Result<usize> {
    format::export_repair_errors_csv(stats, dest)
}

pub fn export_acl_csv(snapshot: &AclSnapshot, dest: &Path) -> Result<usize> {
    format::export_acl_csv(snapshot, dest)
}

pub fn backup_filename(dir: &Path, path_label: &str) -> std::path::PathBuf {
    writer::backup_filename(dir, path_label)
}

pub fn error_csv_filename(dir: &Path, tag: &str) -> std::path::PathBuf {
    writer::error_csv_filename(dir, tag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acl::types::{
        AceEntry, AceType, AclSnapshot, DiffResult, InheritanceFlags, PropagationFlags, RepairStats,
    };
    use std::path::PathBuf;

    fn dummy_entry(principal: &str, ace_type: AceType) -> AceEntry {
        AceEntry {
            principal: principal.to_string(),
            raw_sid: "S-1-5-32-544".to_string(),
            rights_mask: 0x1F01FF,
            ace_type,
            inheritance: InheritanceFlags::BOTH,
            propagation: PropagationFlags::NONE,
            is_inherited: false,
            is_orphan: false,
        }
    }

    fn dummy_snapshot() -> AclSnapshot {
        AclSnapshot {
            path: PathBuf::from(r"C:\test"),
            owner: "BUILTIN\\Administrators".to_string(),
            is_protected: false,
            entries: vec![
                dummy_entry("BUILTIN\\Administrators", AceType::Allow),
                dummy_entry("NT AUTHORITY\\SYSTEM", AceType::Allow),
            ],
        }
    }

    fn temp_path(ext: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "aclmgr_export_test_{}.{ext}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ))
    }

    #[test]
    fn backup_roundtrip() {
        let snap = dummy_snapshot();
        let dest = temp_path("json");
        backup_acl(&snap, &dest).expect("backup_acl failed");
        assert!(dest.exists());

        // Parse and check version
        let raw = std::fs::read_to_string(&dest).unwrap();
        let envelope: AclBackup = serde_json::from_str(&raw).unwrap();
        assert_eq!(envelope.version, schema::BACKUP_VERSION);
        assert_eq!(envelope.acl.entries.len(), 2);

        std::fs::remove_file(&dest).ok();
    }

    #[test]
    fn export_diff_csv_writes_rows() {
        let entry_a = dummy_entry("UserA", AceType::Allow);
        let entry_b = dummy_entry("UserB", AceType::Deny);
        let diff = DiffResult {
            only_in_a: vec![entry_a],
            only_in_b: vec![entry_b],
            common_count: 3,
            owner_diff: None,
            inherit_diff: None,
        };
        let dest = temp_path("csv");
        let n = export_diff_csv(&diff, &dest).unwrap();
        assert_eq!(n, 2);
        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("UserA"));
        assert!(content.contains("UserB"));
        std::fs::remove_file(&dest).ok();
    }

    #[test]
    fn export_repair_errors_csv_counts() {
        let mut stats = RepairStats::default();
        stats.owner_fail.push((PathBuf::from("a"), "err1".into()));
        stats.acl_fail.push((PathBuf::from("b"), "err2".into()));
        let dest = temp_path("csv");
        let n = export_repair_errors_csv(&stats, &dest).unwrap();
        assert_eq!(n, 2);
        std::fs::remove_file(&dest).ok();
    }

    #[test]
    fn export_acl_csv_row_count() {
        let snap = dummy_snapshot();
        let dest = temp_path("csv");
        let n = export_acl_csv(&snap, &dest).unwrap();
        assert_eq!(n, 2);
        std::fs::remove_file(&dest).ok();
    }

    #[test]
    fn backup_filename_is_sanitized() {
        let dir = std::env::temp_dir();
        let p = backup_filename(&dir, r"D:\Some Path\With Spaces");
        let name = p.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("ACL_"));
        assert!(name.ends_with(".json"));
        assert!(!name.contains('\\'));
        assert!(!name.contains(' '));
    }
}
