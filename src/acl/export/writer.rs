use std::path::Path;

use anyhow::{Context, Result};
use chrono::Local;

use crate::acl::error::AclError;
use crate::acl::types::{AceType, AclSnapshot};

use super::schema::{AclBackup, BACKUP_VERSION};

/// Serialize `snapshot` to a JSON file at `dest`.
pub(super) fn backup_acl(snapshot: &AclSnapshot, dest: &Path) -> Result<()> {
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
pub(super) fn restore_acl(backup_path: &Path, target_path: &Path) -> Result<()> {
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

/// Generate a timestamped backup filename in `dir`.
pub(super) fn backup_filename(dir: &Path, path_label: &str) -> std::path::PathBuf {
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
pub(super) fn error_csv_filename(dir: &Path, tag: &str) -> std::path::PathBuf {
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    dir.join(format!("ACLErrors_{tag}_{ts}.csv"))
}
