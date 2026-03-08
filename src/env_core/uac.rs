use super::types::EnvScope;

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::mem;
    use std::ptr;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elev = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut needed = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elev as *mut _ as *mut _,
            mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut needed,
        ) != 0;
        let _ = CloseHandle(token);
        ok && elev.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

pub fn requires_elevation(scope: EnvScope) -> bool {
    matches!(scope, EnvScope::System | EnvScope::All) && !is_elevated()
}

pub fn elevation_hint(scope: EnvScope) -> String {
    let scope_text = match scope {
        EnvScope::System => "system",
        EnvScope::All => "all(system+user)",
        EnvScope::User => "user",
    };
    format!(
        "{} scope write requires Administrator token on Windows. Relaunch shell as Administrator and retry.",
        scope_text
    )
}
