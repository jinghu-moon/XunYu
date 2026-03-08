use std::path::Path;

use anyhow::{Context, Result};
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW, SDDL_REVISION_1,
    SE_FILE_OBJECT, SetNamedSecurityInfoW,
};
use windows::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
};
use windows::core::{PCWSTR, PWSTR};

use crate::acl::error::AclError;

use super::super::error_map::check_win32;
use super::common::path_wide;

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
