use std::ptr;

use super::guards::HandleGuard;
use super::utils::to_wide;
use super::{
    AdjustTokenPrivileges, GetCurrentProcess, LookupPrivilegeValueW, Luid, LuidAndAttributes,
    OpenProcessToken, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_QUERY, TokenPrivileges,
};

pub(crate) fn enable_privilege(name: &str) -> bool {
    unsafe {
        let mut token: super::HANDLE = 0;
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        ) == 0
        {
            return false;
        }
        let _guard = HandleGuard(token);

        let mut luid = Luid {
            low_part: 0,
            high_part: 0,
        };
        let name_w = to_wide(name);
        if LookupPrivilegeValueW(ptr::null(), name_w.as_ptr(), &mut luid) == 0 {
            return false;
        }
        let tp = TokenPrivileges {
            count: 1,
            priv0: LuidAndAttributes {
                luid,
                attributes: SE_PRIVILEGE_ENABLED,
            },
        };
        AdjustTokenPrivileges(token, 0, &tp, 0, ptr::null_mut(), ptr::null_mut()) != 0
    }
}

pub(crate) fn enable_delete_privileges() {
    for priv_name in &[
        "SeTakeOwnershipPrivilege",
        "SeRestorePrivilege",
        "SeBackupPrivilege",
        "SeDebugPrivilege",
    ] {
        let _ = enable_privilege(priv_name);
    }
}
