use std::mem;
use std::ptr;

use super::guards::HandleGuard;
use super::utils::to_wide;
use super::{
    GetCurrentProcess, GetTokenInformation, OpenProcessToken, ShellExecuteW, TOKEN_ELEVATION_CLASS,
    TOKEN_QUERY, TokenElevation,
};

pub(crate) fn is_elevated() -> bool {
    unsafe {
        let mut token: super::HANDLE = 0;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let _guard = HandleGuard(token);
        let mut elev = TokenElevation { is_elevated: 0 };
        let mut needed: super::DWORD = 0;
        GetTokenInformation(
            token,
            TOKEN_ELEVATION_CLASS,
            &mut elev as *mut _ as _,
            mem::size_of::<TokenElevation>() as _,
            &mut needed,
        );
        elev.is_elevated != 0
    }
}

pub(crate) fn relaunch_elevated(exe: &str, args: &str) {
    let op = to_wide("runas");
    let file = to_wide(exe);
    let params = to_wide(args);
    unsafe {
        let _ = ShellExecuteW(
            0,
            op.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            ptr::null(),
            1,
        );
    }
}
