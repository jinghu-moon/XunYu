use std::path::Path;

use anyhow::{Context, Result};
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW, SDDL_REVISION_1,
    SE_FILE_OBJECT, SetNamedSecurityInfoW,
};
use windows::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, LookupAccountNameW, OWNER_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, PSID,
};
use windows::core::{PCWSTR, PWSTR};

use crate::acl::error::AclError;
use crate::acl::reader::sid_to_string;
use crate::acl::types::{AceEntry, AceType, InheritanceFlags, PropagationFlags};

use super::error_map::check_win32;

/// Allocate and return the SID bytes for `principal` (e.g. `"BUILTIN\Administrators"`).
///
/// The returned `Vec<u8>` owns the SID memory. The caller must not free it via
/// Win32 — Rust's allocator owns it.
pub(super) fn lookup_account_sid(principal: &str) -> Result<Vec<u8>> {
    let name_wide: Vec<u16> = principal.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut sid_len: u32 = 0;
        let mut domain_len: u32 = 0;
        let mut sid_use = windows::Win32::Security::SID_NAME_USE(0);

        // First call: size probe
        let _ = LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(name_wide.as_ptr()),
            PSID::default(),
            &mut sid_len,
            PWSTR::null(),
            &mut domain_len,
            &mut sid_use,
        );

        if sid_len == 0 {
            return Err(AclError::InvalidPrincipal(principal.to_string()).into());
        }

        let mut sid_buf = vec![0u8; sid_len as usize];
        let mut domain_buf = vec![0u16; domain_len as usize];

        LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(name_wide.as_ptr()),
            PSID(sid_buf.as_mut_ptr() as *mut _),
            &mut sid_len,
            PWSTR(domain_buf.as_mut_ptr()),
            &mut domain_len,
            &mut sid_use,
        )
        .map_err(|_| AclError::InvalidPrincipal(principal.to_string()))?;

        Ok(sid_buf)
    }
}

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(super) fn path_wide(path: &Path) -> Vec<u16> {
    let canonical = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    to_wide_null(&canonical.to_string_lossy())
}

/// Add a new ACE to `path`'s DACL.
///
/// Uses `SetEntriesInAclW` + `SetNamedSecurityInfoW` so that the existing DACL
/// is preserved and the new rule is appended.
pub(super) fn add_rule(
    path: &Path,
    principal: &str,
    rights_mask: u32,
    ace_type: AceType,
    inheritance: InheritanceFlags,
    propagation: PropagationFlags,
) -> Result<()> {
    use windows::Win32::Security::ACE_FLAGS;
    use windows::Win32::Security::Authorization::{
        DENY_ACCESS, EXPLICIT_ACCESS_W, SET_ACCESS, SetEntriesInAclW, TRUSTEE_IS_SID, TRUSTEE_W,
    };

    let sid_bytes = lookup_account_sid(principal)
        .with_context(|| format!("failed to resolve principal '{principal}'"))?;
    let sid = PSID(sid_bytes.as_ptr() as *mut _);

    // Build ACE flags from InheritanceFlags + PropagationFlags
    let ace_flags = ACE_FLAGS((inheritance.0 & 0x3) | ((propagation.0 & 0x3) << 2));

    let access_mode = match ace_type {
        AceType::Allow => SET_ACCESS,
        AceType::Deny => DENY_ACCESS,
    };

    unsafe {
        // Build EXPLICIT_ACCESS_W with SID trustee
        let mut trustee = TRUSTEE_W::default();
        trustee.TrusteeForm = TRUSTEE_IS_SID;
        trustee.ptstrName = PWSTR(sid.0 as *mut u16);

        let ea = EXPLICIT_ACCESS_W {
            grfAccessPermissions: rights_mask,
            grfAccessMode: access_mode,
            grfInheritance: ace_flags,
            Trustee: trustee,
        };

        // Get existing DACL
        let pw = path_wide(path);
        let mut p_old_dacl: *mut ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();

        let status = GetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut p_old_dacl),
            None,
            &mut p_sd,
        );
        check_win32(status, "add_rule: GetNamedSecurityInfoW failed")?;

        // Merge new entry with existing DACL
        let mut p_new_dacl: *mut ACL = std::ptr::null_mut();
        let status = SetEntriesInAclW(
            Some(std::slice::from_ref(&ea)),
            Some(p_old_dacl),
            &mut p_new_dacl,
        );
        check_win32(status, "add_rule: SetEntriesInAclW failed")?;

        // Write back
        let status = SetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            PSID::default(),
            PSID::default(),
            Some(p_new_dacl),
            None,
        );
        check_win32(status, "add_rule: SetNamedSecurityInfoW failed")?;

        LocalFree(HLOCAL(p_sd.0 as *mut _));
        LocalFree(HLOCAL(p_new_dacl as *mut _));
    }
    Ok(())
}

/// Remove specific explicit ACEs from `path`'s DACL.
///
/// Matches by `(principal, ace_type, rights_mask)`.  Inherited ACEs are
/// silently skipped.
pub(super) fn remove_rules(path: &Path, to_remove: &[AceEntry]) -> Result<usize> {
    use windows::Win32::Security::{
        ACCESS_ALLOWED_ACE, ACCESS_DENIED_ACE, ACL_SIZE_INFORMATION, AclSizeInformation, DeleteAce,
        GetAce, GetAclInformation,
    };

    if to_remove.is_empty() {
        return Ok(0);
    }

    let pw = path_wide(path);
    let mut removed = 0usize;

    unsafe {
        let mut p_dacl: *mut ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();

        let status = GetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut p_dacl),
            None,
            &mut p_sd,
        );
        check_win32(status, "remove_rules: GetNamedSecurityInfoW failed")?;

        if p_dacl.is_null() {
            LocalFree(HLOCAL(p_sd.0 as *mut _));
            return Ok(0);
        }

        let mut info = ACL_SIZE_INFORMATION::default();
        GetAclInformation(
            p_dacl,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
        .map_err(|_| AclError::last_win32())
        .context("remove_rules: GetAclInformation failed")?;

        // Walk backwards so indices stay valid after deletion
        let count = info.AceCount as i32;
        for i in (0..count).rev() {
            let mut ace_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            if GetAce(p_dacl, i as u32, &mut ace_ptr).is_err() || ace_ptr.is_null() {
                continue;
            }

            let header = &*(ace_ptr as *const super::super::reader::AceHeaderPublic);
            // Skip inherited ACEs — they cannot be removed here
            if header.ace_flags & 0x10 != 0 {
                continue;
            }

            let (entry_ace_type, mask, sid_ptr) = match header.ace_type {
                0 => {
                    let ace = &*(ace_ptr as *const ACCESS_ALLOWED_ACE);
                    (
                        AceType::Allow,
                        ace.Mask,
                        &ace.SidStart as *const u32 as *const _,
                    )
                }
                1 => {
                    let ace = &*(ace_ptr as *const ACCESS_DENIED_ACE);
                    (
                        AceType::Deny,
                        ace.Mask,
                        &ace.SidStart as *const u32 as *const _,
                    )
                }
                _ => continue,
            };

            let sid = PSID(sid_ptr as *mut _);
            let raw_sid = sid_to_string(sid).unwrap_or_default();

            // Check if this ACE is in our removal list
            let should_remove = to_remove.iter().any(|r| {
                r.raw_sid == raw_sid
                    && r.ace_type == entry_ace_type
                    && r.rights_mask == mask
                    && !r.is_inherited
            });

            if should_remove {
                DeleteAce(p_dacl, i as u32)
                    .map_err(|_| AclError::last_win32())
                    .context("remove_rules: DeleteAce failed")?;
                removed += 1;
            }
        }

        // Write modified DACL back
        if removed > 0 {
            let status = SetNamedSecurityInfoW(
                PCWSTR(pw.as_ptr()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                PSID::default(),
                PSID::default(),
                Some(p_dacl),
                None,
            );
            check_win32(status, "remove_rules: SetNamedSecurityInfoW failed")?;
        }

        LocalFree(HLOCAL(p_sd.0 as *mut _));
    }
    Ok(removed)
}

/// Remove **all** explicit ACEs whose SID matches `principal`.
///
/// Returns the count of removed entries.
pub(super) fn purge_principal(path: &Path, principal: &str) -> Result<u32> {
    use windows::Win32::Security::{
        ACCESS_ALLOWED_ACE, ACCESS_DENIED_ACE, ACL_SIZE_INFORMATION, AclSizeInformation, DeleteAce,
        GetAce, GetAclInformation,
    };

    let target_sid = lookup_account_sid(principal)
        .with_context(|| format!("purge_principal: cannot resolve '{principal}'"))?;
    let target = PSID(target_sid.as_ptr() as *mut _);
    let target_str = sid_to_string(target).unwrap_or_default();

    let pw = path_wide(path);
    let mut removed = 0u32;

    unsafe {
        let mut p_dacl: *mut ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();

        let status = GetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut p_dacl),
            None,
            &mut p_sd,
        );
        check_win32(status, "purge_principal: GetNamedSecurityInfoW failed")?;

        if p_dacl.is_null() {
            LocalFree(HLOCAL(p_sd.0 as *mut _));
            return Ok(0);
        }

        let mut info = ACL_SIZE_INFORMATION::default();
        GetAclInformation(
            p_dacl,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
        .map_err(|_| AclError::last_win32())
        .context("purge_principal: GetAclInformation failed")?;

        for i in (0..info.AceCount as i32).rev() {
            let mut ace_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            if GetAce(p_dacl, i as u32, &mut ace_ptr).is_err() || ace_ptr.is_null() {
                continue;
            }
            let header = &*(ace_ptr as *const super::super::reader::AceHeaderPublic);
            if header.ace_flags & 0x10 != 0 {
                continue;
            } // skip inherited

            let sid_ptr: *const _ = match header.ace_type {
                0 => {
                    let a = &*(ace_ptr as *const ACCESS_ALLOWED_ACE);
                    &a.SidStart as *const u32 as *const _
                }
                1 => {
                    let a = &*(ace_ptr as *const ACCESS_DENIED_ACE);
                    &a.SidStart as *const u32 as *const _
                }
                _ => continue,
            };

            let sid = PSID(sid_ptr as *mut _);
            if sid_to_string(sid).ok().as_deref() == Some(&target_str) {
                if DeleteAce(p_dacl, i as u32).is_ok() {
                    removed += 1;
                }
            }
        }

        if removed > 0 {
            let status = SetNamedSecurityInfoW(
                PCWSTR(pw.as_ptr()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                PSID::default(),
                PSID::default(),
                Some(p_dacl),
                None,
            );
            check_win32(
                status,
                "purge_principal: SetNamedSecurityInfoW write-back failed",
            )?;
        }

        LocalFree(HLOCAL(p_sd.0 as *mut _));
    }
    Ok(removed)
}

/// Change the owner of `path`.
///
/// Requires `SeRestorePrivilege` to be active on the caller's token.
pub(super) fn set_owner(path: &Path, owner: &str) -> Result<()> {
    crate::acl::privilege::enable_privilege("SeRestorePrivilege")
        .context("set_owner: failed to enable SeRestorePrivilege")?;

    let sid_bytes = lookup_account_sid(owner)
        .with_context(|| format!("set_owner: cannot resolve '{owner}'"))?;
    let sid = PSID(sid_bytes.as_ptr() as *mut _);
    let pw = path_wide(path);

    unsafe {
        let status = SetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            sid,
            PSID::default(),
            None,
            None,
        );
        check_win32(
            status,
            format!(
                "set_owner: SetNamedSecurityInfoW failed for {}",
                path.display()
            ),
        )?;
    }
    Ok(())
}

/// Copy the entire ACL (owner + DACL) from `src` to `dst` using SDDL strings.
pub(super) fn copy_acl(src: &Path, dst: &Path) -> Result<()> {
    let src_wide = path_wide(src);
    let dst_wide = path_wide(dst);

    unsafe {
        // Read source SD as SDDL
        let mut p_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();
        let status = GetNamedSecurityInfoW(
            PCWSTR(src_wide.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION,
            None,
            None,
            None,
            None,
            &mut p_sd,
        );
        check_win32(status, "copy_acl: read source failed")?;

        let mut sddl_ptr = PWSTR::null();
        let mut sddl_len: u32 = 0;
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            p_sd,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION,
            &mut sddl_ptr,
            Some(&mut sddl_len),
        )
        .map_err(|_| AclError::last_win32())
        .context("copy_acl: ConvertSecurityDescriptorToStringSecurityDescriptorW failed")?;

        LocalFree(HLOCAL(p_sd.0 as *mut _));

        // Convert SDDL back to a new SD
        let mut p_new_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();
        let mut sd_size: u32 = 0;
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_ptr,
            SDDL_REVISION_1,
            &mut p_new_sd,
            Some(&mut sd_size),
        )
        .map_err(|_| AclError::last_win32())
        .context("copy_acl: ConvertStringSecurityDescriptorToSecurityDescriptorW failed")?;

        LocalFree(HLOCAL(sddl_ptr.0 as *mut _));

        // Extract owner + DACL from new SD
        let mut p_owner2: PSID = PSID::default();
        let mut owner_def = windows::Win32::Foundation::BOOL(0);
        let mut p_dacl2: *mut ACL = std::ptr::null_mut();
        let mut dacl_present = windows::Win32::Foundation::BOOL(0);
        let mut dacl_def = windows::Win32::Foundation::BOOL(0);

        windows::Win32::Security::GetSecurityDescriptorOwner(
            p_new_sd,
            &mut p_owner2,
            &mut owner_def,
        )
        .map_err(|_| AclError::last_win32())
        .context("copy_acl: GetSecurityDescriptorOwner failed")?;

        windows::Win32::Security::GetSecurityDescriptorDacl(
            p_new_sd,
            &mut dacl_present,
            &mut p_dacl2,
            &mut dacl_def,
        )
        .map_err(|_| AclError::last_win32())
        .context("copy_acl: GetSecurityDescriptorDacl failed")?;

        // Apply to destination
        let status = SetNamedSecurityInfoW(
            PCWSTR(dst_wide.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION,
            p_owner2,
            PSID::default(),
            Some(p_dacl2),
            None,
        );
        check_win32(status, "copy_acl: SetNamedSecurityInfoW failed")?;

        LocalFree(HLOCAL(p_new_sd.0 as *mut _));
    }
    Ok(())
}
