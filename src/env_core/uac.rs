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

#[cfg(windows)]
#[allow(dead_code)]
pub fn relaunch_elevated(exe: &str, args: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let op = to_wide("runas");
    let file = to_wide(exe);
    let params = to_wide(args);
    let params_ptr = if args.trim().is_empty() {
        std::ptr::null()
    } else {
        params.as_ptr()
    };
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            file.as_ptr(),
            params_ptr,
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    let code = result as isize;
    if code <= 32 {
        return Err(format!("ShellExecuteW failed: {code}"));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(not(windows))]
pub fn relaunch_elevated(_exe: &str, _args: &str) -> Result<(), String> {
    Err("elevation is Windows-only".to_string())
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

#[cfg(windows)]
#[allow(dead_code)]
fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
