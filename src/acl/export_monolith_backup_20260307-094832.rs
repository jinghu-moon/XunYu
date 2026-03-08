/// Phase 3 — Backup, restore and CSV export helpers.
///
/// # Backup format
/// ```json
/// { "version": 1, "created_at": "2026-02-25T10:00:00+08:00",
///   "original_path": "D:\\Data", "acl": { … AclSnapshot … } }
/// ```
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::acl::error::AclError;
use crate::acl::orphan::OrphanEntry;
use crate::acl::types::DiffResult;
use crate::acl::types::{AceType, AclSnapshot, RepairStats};

// ── Backup envelope ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AclBackup {
    pub version: u32,
    pub created_at: String,
    pub original_path: String,
    pub acl: AclSnapshot,
}

const BACKUP_VERSION: u32 = 1;

// ── Backup & restore ──────────────────────────────────────────────────────────

/// Serialize `snapshot` to a JSON file at `dest`.
pub fn backup_acl(snapshot: &AclSnapshot, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("backup_acl: cannot create dir {}", parent.display()))?;
    }
    let envelope = AclBackup {
        version: BACKUP_VERSION,
        created_at: Local::now().to_rfc3339(),
        original_path: snapshot.path.to_string_lossy().into_owned(),
        acl: snapshot.clone(),
    };
    let json =
        serde_json::to_string_pretty(&envelope).context("backup_acl: serialization failed")?;
    std::fs::write(dest, json)
        .with_context(|| format!("backup_acl: cannot write to {}", dest.display()))?;
    Ok(())
}

/// Restore an ACL from a JSON backup file onto `target_path`.
///
/// Only **explicit** (non-inherited) ACEs from the backup are applied.
/// Inherited ACEs are skipped — they come from the parent and will be
/// re-applied automatically once the parent's ACL is intact.
///
/// Steps:
/// 1. Parse + version-check the backup.
/// 2. Build a brand-new DACL from the explicit ACEs in the backup.
/// 3. Apply owner + DACL in one `SetNamedSecurityInfoW` call.
pub fn restore_acl(backup_path: &Path, target_path: &Path) -> Result<()> {
    let raw = std::fs::read_to_string(backup_path)
        .with_context(|| format!("restore_acl: cannot read {}", backup_path.display()))?;
    let envelope: AclBackup =
        serde_json::from_str(&raw).context("restore_acl: invalid backup JSON")?;

    if envelope.version != BACKUP_VERSION {
        return Err(AclError::BackupVersionMismatch {
            expected: BACKUP_VERSION,
            found: envelope.version,
        }
        .into());
    }

    let snapshot = &envelope.acl;
    let explicit_entries: Vec<_> = snapshot
        .entries
        .iter()
        .filter(|e| !e.is_inherited)
        .collect();

    apply_dacl_from_entries(target_path, &explicit_entries, &snapshot.owner)
}

/// Build and apply a new DACL from `entries` onto `target_path`,
/// also setting `owner`.
fn apply_dacl_from_entries(
    target_path: &Path,
    entries: &[&crate::acl::types::AceEntry],
    owner: &str,
) -> Result<()> {
    use crate::acl::privilege::enable_privilege;
    use crate::acl::writer::lookup_account_sid;
    use windows::Win32::Security::Authorization::{SE_FILE_OBJECT, SetNamedSecurityInfoW};
    use windows::Win32::Security::{
        ACE_FLAGS, ACL, ACL_REVISION, AddAccessAllowedAceEx, AddAccessDeniedAceEx,
        DACL_SECURITY_INFORMATION, GetLengthSid, InitializeAcl, OWNER_SECURITY_INFORMATION, PSID,
    };
    use windows::core::PCWSTR;

    let _ = enable_privilege("SeRestorePrivilege");
    let _ = enable_privilege("SeBackupPrivilege");

    // Resolve all SIDs upfront (fail fast before touching the target)
    let mut sid_bufs: Vec<(Vec<u8>, &crate::acl::types::AceEntry)> = Vec::new();
    for e in entries {
        // Try by principal name first, fall back to raw SID string
        let bytes = lookup_account_sid(&e.principal)
            .or_else(|_| lookup_account_sid(&e.raw_sid))
            .with_context(|| format!("restore_acl: cannot resolve principal '{}'", e.principal))?;
        sid_bufs.push((bytes, e));
    }

    // Calculate required ACL buffer size
    let acl_header_size = std::mem::size_of::<ACL>() as u32;
    let mut needed: u32 = acl_header_size + 8; // header + alignment padding
    for (sid_bytes, _) in &sid_bufs {
        let sid = PSID(sid_bytes.as_ptr() as *mut _);
        // ACCESS_ALLOWED_ACE header is 8 bytes (type+flags+size+mask);
        // SID starts at SidStart (offset 8); total = 8 + SID length
        let sid_len = unsafe { GetLengthSid(sid) };
        needed += 8 + sid_len + 4; // +4 padding per ACE
    }
    needed = needed.max(8); // minimum meaningful ACL

    let mut acl_buf = vec![0u8; needed as usize];
    let new_acl = acl_buf.as_mut_ptr() as *mut ACL;

    unsafe {
        InitializeAcl(new_acl, needed, ACL_REVISION)
            .map_err(|_| AclError::last_win32())
            .context("restore_acl: InitializeAcl failed")?;

        for (sid_bytes, entry) in &sid_bufs {
            let sid = PSID(sid_bytes.as_ptr() as *mut _);
            let ace_flags = (entry.inheritance.0 & 0x3) | ((entry.propagation.0 & 0x3) << 2);

            match entry.ace_type {
                AceType::Allow => {
                    AddAccessAllowedAceEx(
                        new_acl,
                        ACL_REVISION,
                        ACE_FLAGS(ace_flags),
                        entry.rights_mask,
                        sid,
                    )
                    .map_err(|_| AclError::last_win32())
                    .with_context(|| {
                        format!(
                            "restore_acl: AddAccessAllowedAceEx for '{}'",
                            entry.principal
                        )
                    })?;
                }
                AceType::Deny => {
                    AddAccessDeniedAceEx(
                        new_acl,
                        ACL_REVISION,
                        ACE_FLAGS(ace_flags),
                        entry.rights_mask,
                        sid,
                    )
                    .map_err(|_| AclError::last_win32())
                    .with_context(|| {
                        format!(
                            "restore_acl: AddAccessDeniedAceEx for '{}'",
                            entry.principal
                        )
                    })?;
                }
            }
        }

        // Resolve owner SID
        let owner_bytes = lookup_account_sid(owner)
            .with_context(|| format!("restore_acl: cannot resolve owner '{owner}'"))?;
        let owner_sid = PSID(owner_bytes.as_ptr() as *mut _);

        let pw: Vec<u16> = target_path
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let status = SetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION,
            owner_sid,
            PSID::default(),
            Some(new_acl),
            None,
        );
        if status.0 != 0 {
            let err = anyhow::Error::new(AclError::from_win32(status.0));
            return Err(err).with_context(|| {
                format!(
                    "restore_acl: SetNamedSecurityInfoW for {}",
                    target_path.display()
                )
            });
        }
    }
    Ok(())
}

// ── CSV exports ───────────────────────────────────────────────────────────────

/// Export a `DiffResult` as CSV. Returns the number of data rows written.
pub fn export_diff_csv(diff: &DiffResult, dest: &Path) -> Result<usize> {
    let mut wtr = csv::Writer::from_path(dest)
        .with_context(|| format!("export_diff_csv: cannot open {}", dest.display()))?;
    wtr.write_record(["差异方向", "主体", "权限", "访问类型", "来源", "SID"])?;
    let mut count = 0usize;
    for e in &diff.only_in_a {
        let rights = e.rights_display();
        let ace_type = e.ace_type.to_string();
        let src = if e.is_inherited { "继承" } else { "显式" };
        wtr.write_record([
            "仅在A",
            e.principal.as_str(),
            rights.as_str(),
            ace_type.as_str(),
            src,
            e.raw_sid.as_str(),
        ])?;
        count += 1;
    }
    for e in &diff.only_in_b {
        let rights = e.rights_display();
        let ace_type = e.ace_type.to_string();
        let src = if e.is_inherited { "继承" } else { "显式" };
        wtr.write_record([
            "仅在B",
            e.principal.as_str(),
            rights.as_str(),
            ace_type.as_str(),
            src,
            e.raw_sid.as_str(),
        ])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

/// Export a list of orphaned SID entries as CSV.
pub fn export_orphans_csv(orphans: &[OrphanEntry], dest: &Path) -> Result<usize> {
    let mut wtr = csv::Writer::from_path(dest)
        .with_context(|| format!("export_orphans_csv: cannot open {}", dest.display()))?;
    wtr.write_record(["路径", "孤儿SID", "访问类型", "权限", "权限掩码"])?;
    for o in orphans {
        let path = o.path.to_string_lossy().into_owned();
        let ace_type = o.ace.ace_type.to_string();
        let rights = o.ace.rights_display();
        let mask = format!("{:#010x}", o.ace.rights_mask);
        wtr.write_record([
            path.as_str(),
            o.ace.raw_sid.as_str(),
            ace_type.as_str(),
            rights.as_str(),
            mask.as_str(),
        ])?;
    }
    wtr.flush()?;
    Ok(orphans.len())
}

/// Export failed paths from a `RepairStats` as CSV.
pub fn export_repair_errors_csv(stats: &RepairStats, dest: &Path) -> Result<usize> {
    let mut wtr = csv::Writer::from_path(dest)
        .with_context(|| format!("export_repair_errors_csv: cannot open {}", dest.display()))?;
    wtr.write_record(["阶段", "路径", "错误信息"])?;
    let mut count = 0usize;
    for (p, e) in &stats.owner_fail {
        wtr.write_record(["夺权", &p.to_string_lossy(), e.as_str()])?;
        count += 1;
    }
    for (p, e) in &stats.acl_fail {
        wtr.write_record(["赋权", &p.to_string_lossy(), e.as_str()])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

/// Export all ACE entries from a snapshot as CSV.
pub fn export_acl_csv(snapshot: &AclSnapshot, dest: &Path) -> Result<usize> {
    let mut wtr = csv::Writer::from_path(dest)
        .with_context(|| format!("export_acl_csv: cannot open {}", dest.display()))?;
    wtr.write_record([
        "访问类型",
        "来源",
        "主体",
        "权限名",
        "权限掩码",
        "继承标志",
        "传播标志",
        "是否孤儿",
        "SID",
    ])?;
    for e in &snapshot.entries {
        let ace_type = e.ace_type.to_string();
        let rights = e.rights_display();
        let mask = format!("{:#010x}", e.rights_mask);
        let inheritance = e.inheritance.to_string();
        let propagation = e.propagation.to_string();
        let src = if e.is_inherited { "继承" } else { "显式" };
        let orphan = if e.is_orphan { "是" } else { "否" };
        wtr.write_record([
            ace_type.as_str(),
            src,
            e.principal.as_str(),
            rights.as_str(),
            mask.as_str(),
            inheritance.as_str(),
            propagation.as_str(),
            orphan,
            e.raw_sid.as_str(),
        ])?;
    }
    wtr.flush()?;
    Ok(snapshot.entries.len())
}

/// Generate a timestamped backup filename in `dir`.
pub fn backup_filename(dir: &Path, path_label: &str) -> std::path::PathBuf {
    // Sanitize the label: keep only alphanumeric + underscore
    let label: String = path_label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    dir.join(format!("ACL_{label}_{ts}.json"))
}

/// Generate a timestamped error CSV filename in `dir`.
pub fn error_csv_filename(dir: &Path, tag: &str) -> std::path::PathBuf {
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    dir.join(format!("ACLErrors_{tag}_{ts}.csv"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

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
        assert_eq!(envelope.version, BACKUP_VERSION);
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
