use crate::runtime;

use windows_sys::Win32::Foundation::{GetLastError, HANDLE, LUID};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_PRIVILEGES,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;
use windows_sys::Win32::System::Threading::OpenProcessToken;

use super::core::OwnedHandle;

pub(super) trait DebugPrivilegeApi {
    unsafe fn open_process_token(&self, process: HANDLE, access: u32, token: *mut HANDLE) -> i32;
    unsafe fn lookup_privilege_value_w(
        &self,
        system_name: *const u16,
        name: *const u16,
        luid: *mut LUID,
    ) -> i32;
    unsafe fn adjust_token_privileges(&self, token: HANDLE, new_state: *const TOKEN_PRIVILEGES);
    unsafe fn get_last_error(&self) -> u32;
}

pub(super) struct RealDebugPrivilegeApi;

impl DebugPrivilegeApi for RealDebugPrivilegeApi {
    unsafe fn open_process_token(&self, process: HANDLE, access: u32, token: *mut HANDLE) -> i32 {
        unsafe { OpenProcessToken(process, access, token) }
    }

    unsafe fn lookup_privilege_value_w(
        &self,
        system_name: *const u16,
        name: *const u16,
        luid: *mut LUID,
    ) -> i32 {
        unsafe { LookupPrivilegeValueW(system_name, name, luid) }
    }

    unsafe fn adjust_token_privileges(&self, token: HANDLE, new_state: *const TOKEN_PRIVILEGES) {
        unsafe {
            AdjustTokenPrivileges(
                token,
                0,
                new_state,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }

    unsafe fn get_last_error(&self) -> u32 {
        unsafe { GetLastError() }
    }
}

pub(super) fn try_enable_debug_privilege_with_api(
    verbose: bool,
    api: &dyn DebugPrivilegeApi,
) -> bool {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if api.open_process_token(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &mut token) == 0 {
            if verbose {
                eprintln!("[DEBUG] SeDebugPrivilege: OpenProcessToken failed");
            }
            return false;
        }
        let _token_guard = OwnedHandle::new(token);

        // "SeDebugPrivilege" as null-terminated UTF-16
        let name: &[u16] = &[
            b'S' as u16,
            b'e' as u16,
            b'D' as u16,
            b'e' as u16,
            b'b' as u16,
            b'u' as u16,
            b'g' as u16,
            b'P' as u16,
            b'r' as u16,
            b'i' as u16,
            b'v' as u16,
            b'i' as u16,
            b'l' as u16,
            b'e' as u16,
            b'g' as u16,
            b'e' as u16,
            0,
        ];

        let mut tp = std::mem::zeroed::<TOKEN_PRIVILEGES>();
        if api.lookup_privilege_value_w(std::ptr::null(), name.as_ptr(), &mut tp.Privileges[0].Luid)
            == 0
        {
            if verbose {
                eprintln!("[DEBUG] SeDebugPrivilege: LookupPrivilegeValueW failed");
            }
            return false;
        }
        tp.PrivilegeCount = 1;
        tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

        api.adjust_token_privileges(token, &tp);
        let ok = api.get_last_error() == 0;
        if verbose {
            eprintln!(
                "[DEBUG] SeDebugPrivilege: {}",
                if ok { "enabled" } else { "not granted" }
            );
        }
        ok
    }
}

pub(super) fn try_enable_debug_privilege() -> bool {
    try_enable_debug_privilege_with_api(runtime::is_verbose(), &RealDebugPrivilegeApi)
}
