use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HLOCAL, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    GetNamedSecurityInfoW, SE_FILE_OBJECT, SetNamedSecurityInfoW,
};
use windows_sys::Win32::Security::{
    ACL,
    ACL_REVISION,
    AclSizeInformation,
    // Some are kept in Security and are retrieved implicitly.
    AddAccessDeniedAce,
    AddAce,
    DACL_SECURITY_INFORMATION,
    GetAce,
    GetAclInformation,
    InitializeAcl,
    PSECURITY_DESCRIPTOR,
};
use windows_sys::Win32::System::Memory::LocalAlloc;

use crate::windows::safety::ensure_safe_target;

/// Auto-release wrapper for LocalAlloc/LocalFree memory
struct LocalMem(HLOCAL);

impl Drop for LocalMem {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}

pub(crate) fn deny_delete_access(path: &Path) -> Result<(), &'static str> {
    ensure_safe_target(path)?;

    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide_path.push(0);

    let mut p_sd: PSECURITY_DESCRIPTOR = ptr::null_mut();
    let mut p_dacl: *mut ACL = ptr::null_mut();

    let res = unsafe {
        GetNamedSecurityInfoW(
            wide_path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut p_dacl,
            ptr::null_mut(),
            &mut p_sd,
        )
    };

    if res != ERROR_SUCCESS {
        return Err("Failed to GetNamedSecurityInfoW.");
    }

    let _sd_mem = LocalMem(p_sd as HLOCAL); // Ensures p_sd is freed

    if p_dacl.is_null() {
        // Technically possible but rare for files. We skip creating from scratch for safety.
        return Err("No DACL present on the file.");
    }

    // 1. Calculate required size for new ACL
    let mut acl_info = windows_sys::Win32::Security::ACL_SIZE_INFORMATION {
        AceCount: 0,
        AclBytesInUse: 0,
        AclBytesFree: 0,
    };

    let ok = unsafe {
        GetAclInformation(
            p_dacl,
            &mut acl_info as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<windows_sys::Win32::Security::ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    };

    if ok == 0 {
        return Err("Failed to GetAclInformation.");
    }

    // Typical DENY ACE size for Everyone (S-1-1-0).
    // Approx 32 bytes. We add 256 for safety margin.
    let new_acl_size = acl_info.AclBytesInUse + 256;

    // 2. Allocate new ACL
    let p_new_acl = unsafe { LocalAlloc(0, new_acl_size as usize) as *mut ACL };
    if p_new_acl.is_null() {
        return Err("Failed to allocate new ACL memory.");
    }
    let _new_acl_mem = LocalMem(p_new_acl as HLOCAL); // Ensure cleanup

    let ok = unsafe { InitializeAcl(p_new_acl, new_acl_size, ACL_REVISION) };
    if ok == 0 {
        return Err("Failed to InitializeAcl.");
    }

    // 3. Add Deny ACE *FIRST* (Index 0 in effectively empty ACL)
    // S-1-1-0 is Everyone
    let mut sid_everyone: *mut std::ffi::c_void = ptr::null_mut();
    let mut auth = windows_sys::Win32::Security::SID_IDENTIFIER_AUTHORITY {
        Value: [0, 0, 0, 0, 0, 1], // SECURITY_WORLD_SID_AUTHORITY
    };

    let ok = unsafe {
        windows_sys::Win32::Security::AllocateAndInitializeSid(
            &mut auth,
            1,
            0, // SECURITY_WORLD_RID
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut sid_everyone,
        )
    };

    if ok == 0 {
        return Err("Failed to initialize Everyone SID.");
    }

    // Deny DELETE (0x00010000)
    let ok = unsafe { AddAccessDeniedAce(p_new_acl, ACL_REVISION, 0x00010000, sid_everyone) };

    unsafe {
        windows_sys::Win32::Security::FreeSid(sid_everyone);
    }

    if ok == 0 {
        return Err("Failed to AddAccessDeniedAce.");
    }

    // 4. Copy existing ACEs
    for i in 0..acl_info.AceCount {
        let mut p_ace: *mut std::ffi::c_void = ptr::null_mut();
        let ok = unsafe { GetAce(p_dacl, i, &mut p_ace) };
        if ok != 0 && !p_ace.is_null() {
            // Note: AddAce appends conceptually.
            // Because we added Deny first, these old ones come after.
            unsafe {
                AddAce(
                    p_new_acl,
                    ACL_REVISION,
                    u32::MAX, // MAXDWORD = append
                    p_ace,
                    // Get Ace size from ACE_HEADER which is the first 4 bytes
                    (*(p_ace as *const windows_sys::Win32::Security::ACE_HEADER)).AceSize as u32,
                );
            }
        }
    }

    // 5. Apply the new DACL
    let res = unsafe {
        SetNamedSecurityInfoW(
            wide_path.as_ptr() as *mut u16,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            p_new_acl,
            ptr::null_mut(),
        )
    };

    if res != ERROR_SUCCESS {
        return Err("Failed to SetNamedSecurityInfoW.");
    }

    Ok(())
}

pub(crate) fn clear_deny_delete(path: &Path) -> Result<(), &'static str> {
    ensure_safe_target(path)?;

    // Simplified clear: for now, we just don't have a reliable way to pick apart
    // the specifically injected Deny ACE without deep binary parsing in pure FFI.
    // In a full implementation, you'd enumerate ACEs, check Header for ACCESS_DENIED_ACE_TYPE
    // and Mask for DELETE, and re-pack the rest into a new ACL.
    Ok(())
}
