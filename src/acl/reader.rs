use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
};
use windows::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACCESS_DENIED_ACE, ACL_SIZE_INFORMATION, AclSizeInformation,
    DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION, GetAce, GetAclInformation, IsValidSid,
    LookupAccountSidW, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
};
use windows::core::PWSTR;

use crate::acl::error::AclError;
use crate::acl::types::{
    AceEntry, AceType, AclSnapshot, InheritanceFlags, PROTECTED_NAMES, PropagationFlags,
};

// Win32 ACE header — must be pub so writer.rs / orphan.rs can cast raw pointers.
#[repr(C)]
pub struct AceHeaderPublic {
    pub ace_type: u8,
    pub ace_flags: u8,
    pub ace_size: u16,
}

// Internal alias used within this module
type AceHeader = AceHeaderPublic;

const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
const ACCESS_DENIED_ACE_TYPE: u8 = 1;

// ACE flag bits
const OBJECT_INHERIT_ACE: u8 = 0x01;
const CONTAINER_INHERIT_ACE: u8 = 0x02;
const NO_PROPAGATE_INHERIT: u8 = 0x04;
const INHERIT_ONLY_ACE: u8 = 0x08;
const INHERITED_ACE: u8 = 0x10;

/// Read the full ACL snapshot (owner + DACL entries) for `path`.
pub fn get_acl(path: &Path) -> Result<AclSnapshot> {
    let path = dunce::canonicalize(path)
        .with_context(|| format!("cannot canonicalize path: {}", path.display()))?;

    let path_wide: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut p_owner: PSID = PSID::default();
        let mut p_dacl: *mut windows::Win32::Security::ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();

        let status = GetNamedSecurityInfoW(
            windows::core::PCWSTR(path_wide.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION,
            Some(&mut p_owner),
            None,
            Some(&mut p_dacl),
            None,
            &mut p_sd,
        );
        if status.0 != 0 {
            let err = anyhow::Error::new(AclError::from_win32(status.0));
            return Err(err).context(format!(
                "GetNamedSecurityInfoW failed for {}",
                path.display()
            ));
        }

        // Owner
        let owner = resolve_sid(p_owner).unwrap_or_else(|_| "(unknown)".to_string());

        // Check if DACL inheritance is disabled (protected)
        let is_protected = dacl_is_protected(p_sd);

        // Parse DACL entries
        let entries = if p_dacl.is_null() {
            vec![]
        } else {
            parse_dacl(p_dacl)?
        };

        LocalFree(HLOCAL(p_sd.0 as *mut _));

        Ok(AclSnapshot {
            path,
            owner,
            is_protected,
            entries,
        })
    }
}

/// Convert a `PSID` to a resolved account name `DOMAIN\User`.
///
/// Falls back to the raw SID string on failure, and sets `is_orphan = true`
/// on the returned entry.
pub fn resolve_sid(sid: PSID) -> Result<String> {
    if sid.is_invalid() {
        return Ok("(null SID)".to_string());
    }
    unsafe {
        if IsValidSid(sid).as_bool() == false {
            return Ok("(invalid SID)".to_string());
        }

        let mut name_len: u32 = 0;
        let mut domain_len: u32 = 0;
        let mut sid_use = windows::Win32::Security::SID_NAME_USE(0);

        // First call to get buffer sizes
        let _ = LookupAccountSidW(
            None,
            sid,
            PWSTR::null(),
            &mut name_len,
            PWSTR::null(),
            &mut domain_len,
            &mut sid_use,
        );

        if name_len == 0 {
            return sid_to_string(sid);
        }

        let mut name_buf = vec![0u16; name_len as usize];
        let mut domain_buf = vec![0u16; domain_len as usize];

        LookupAccountSidW(
            None,
            sid,
            PWSTR(name_buf.as_mut_ptr()),
            &mut name_len,
            PWSTR(domain_buf.as_mut_ptr()),
            &mut domain_len,
            &mut sid_use,
        )
        .map_err(|_| AclError::last_win32())?;

        // Trim null terminators
        let name = OsString::from_wide(
            &name_buf[..name_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(name_buf.len())],
        )
        .to_string_lossy()
        .into_owned();

        let domain = OsString::from_wide(
            &domain_buf[..domain_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(domain_buf.len())],
        )
        .to_string_lossy()
        .into_owned();

        if domain.is_empty() {
            Ok(name)
        } else {
            Ok(format!("{domain}\\{name}"))
        }
    }
}

/// Convert a `PSID` to its canonical string form (e.g. `"S-1-5-32-544"`).
pub fn sid_to_string(sid: PSID) -> Result<String> {
    unsafe {
        let mut str_sid = PWSTR::null();
        ConvertSidToStringSidW(sid, &mut str_sid)
            .map_err(|_| AclError::last_win32())
            .context("ConvertSidToStringSidW failed")?;

        let len = (0..).take_while(|&i| *str_sid.0.add(i) != 0).count();
        let slice = std::slice::from_raw_parts(str_sid.0, len);
        let s = OsString::from_wide(slice).to_string_lossy().into_owned();
        LocalFree(HLOCAL(str_sid.0 as *mut _));
        Ok(s)
    }
}

/// Enumerate filesystem objects under `path`.
///
/// * `recursive = false` → only immediate children
/// * `recursive = true`  → all descendants
///
/// Protected paths (e.g. `$RECYCLE.BIN`) are always excluded.
pub fn list_children(path: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let path = dunce::canonicalize(path)
        .with_context(|| format!("cannot canonicalize: {}", path.display()))?;

    let mut results = Vec::new();
    collect_children(&path, recursive, &mut results)?;
    Ok(results)
}

fn collect_children(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Ok(()),
        Err(e) => return Err(e.into()),
    };

    for entry in entries.flatten() {
        let child_path = entry.path();
        if is_protected_path(&child_path) {
            continue;
        }
        out.push(child_path.clone());
        if recursive {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    collect_children(&child_path, true, out)?;
                }
            }
        }
    }
    Ok(())
}

/// Returns `true` when `path`'s leaf name matches a protected system entry.
pub fn is_protected_path(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        return PROTECTED_NAMES
            .iter()
            .any(|&p| name.eq_ignore_ascii_case(p));
    }
    false
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Parse all ACEs from a DACL pointer into `Vec<AceEntry>`.
unsafe fn parse_dacl(dacl: *mut windows::Win32::Security::ACL) -> Result<Vec<AceEntry>> {
    // Get the ACE count from AclSizeInformation
    let mut info = ACL_SIZE_INFORMATION::default();
    unsafe {
        GetAclInformation(
            dacl,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
        .map_err(|_| AclError::last_win32())
        .context("GetAclInformation failed")?;
    }

    let mut entries = Vec::with_capacity(info.AceCount as usize);

    for i in 0..info.AceCount {
        let mut ace_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        if unsafe { GetAce(dacl, i, &mut ace_ptr) }.is_err() {
            continue;
        }
        if ace_ptr.is_null() {
            continue;
        }

        let header = unsafe { &*(ace_ptr as *const AceHeader) };

        let (ace_type, mask, sid_ptr) = match header.ace_type {
            ACCESS_ALLOWED_ACE_TYPE => {
                let ace = unsafe { &*(ace_ptr as *const ACCESS_ALLOWED_ACE) };
                (
                    AceType::Allow,
                    ace.Mask,
                    &ace.SidStart as *const u32 as *const _,
                )
            }
            ACCESS_DENIED_ACE_TYPE => {
                let ace = unsafe { &*(ace_ptr as *const ACCESS_DENIED_ACE) };
                (
                    AceType::Deny,
                    ace.Mask,
                    &ace.SidStart as *const u32 as *const _,
                )
            }
            _ => continue, // Skip object / compound ACE types
        };

        let flags = header.ace_flags;
        let is_inherited = flags & INHERITED_ACE != 0;

        let inheritance = InheritanceFlags(
            if flags & OBJECT_INHERIT_ACE != 0 {
                0x1
            } else {
                0
            } | if flags & CONTAINER_INHERIT_ACE != 0 {
                0x2
            } else {
                0
            },
        );

        let propagation = PropagationFlags(
            if flags & NO_PROPAGATE_INHERIT != 0 {
                0x1
            } else {
                0
            } | if flags & INHERIT_ONLY_ACE != 0 {
                0x2
            } else {
                0
            },
        );

        let sid = PSID(sid_ptr as *mut _);
        let raw_sid = sid_to_string(sid).unwrap_or_else(|_| "(invalid)".to_string());
        let principal_result = resolve_sid(sid);
        let (principal, is_orphan) = match principal_result {
            Ok(name) => (name, false),
            Err(_) => (raw_sid.clone(), true),
        };

        entries.push(AceEntry {
            principal,
            raw_sid,
            rights_mask: mask,
            ace_type,
            inheritance,
            propagation,
            is_inherited,
            is_orphan,
        });
    }

    Ok(entries)
}

/// Determine whether the DACL of a security descriptor is marked as protected
/// (i.e. inheritance from parent is disabled).
unsafe fn dacl_is_protected(sd: PSECURITY_DESCRIPTOR) -> bool {
    use windows::Win32::Security::{GetSecurityDescriptorControl, SE_DACL_PROTECTED};
    let mut control: u16 = 0;
    let mut revision = 0u32;
    if unsafe { GetSecurityDescriptorControl(sd, &mut control, &mut revision) }.is_ok() {
        return (control & SE_DACL_PROTECTED.0) != 0;
    }
    false
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn is_protected_path_matches_recycle_bin() {
        let path = PathBuf::from(r"D:\$RECYCLE.BIN");
        assert!(is_protected_path(&path));
    }

    #[test]
    fn is_protected_path_case_insensitive() {
        let path = PathBuf::from(r"D:\$recycle.bin");
        assert!(is_protected_path(&path));
    }

    #[test]
    fn is_protected_path_normal_dir_is_false() {
        let path = PathBuf::from(r"C:\Users\Public");
        assert!(!is_protected_path(&path));
    }

    /// Integration test — requires actual Windows filesystem access.
    #[test]
    #[cfg(windows)]
    fn get_acl_temp_dir() {
        let dir = env::temp_dir();
        let snapshot = get_acl(&dir).expect("get_acl should succeed for temp dir");
        assert!(!snapshot.owner.is_empty(), "owner should not be empty");
        assert!(
            !snapshot.entries.is_empty(),
            "temp dir should have at least one ACE"
        );
    }

    #[test]
    #[cfg(windows)]
    fn list_children_temp_dir() {
        let dir = env::temp_dir();
        // Should not panic; may return empty list on a clean system
        let _ = list_children(&dir, false).expect("list_children should not fail");
    }
}
