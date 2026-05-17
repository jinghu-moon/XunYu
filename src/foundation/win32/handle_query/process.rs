use std::path::Path;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};

pub(super) fn process_name_from_pid(pid: u32) -> String {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return format!("pid-{pid}");
    }

    let mut buf = vec![0u16; 32768];
    let mut size = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 || size == 0 {
        return format!("pid-{pid}");
    }

    let full = String::from_utf16_lossy(&buf[..size as usize]);
    Path::new(&full)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or(full)
}

pub(super) fn infer_app_type_from_session(process_name: &str, session_id: Option<u32>) -> u32 {
    // Align with Restart Manager's RM_APP_TYPE:
    // 1=UnknownApp, 4=Service, 5=Explorer.
    const RM_UNKNOWN_APP: u32 = 1;
    const RM_SERVICE: u32 = 4;
    const RM_EXPLORER: u32 = 5;

    if process_name.eq_ignore_ascii_case("explorer.exe") {
        return RM_EXPLORER;
    }

    match session_id {
        Some(0) => RM_SERVICE,
        Some(_) => RM_UNKNOWN_APP,
        None => RM_UNKNOWN_APP,
    }
}

pub(super) fn infer_app_type(pid: u32, process_name: &str) -> u32 {
    let mut session_id: u32 = 0;
    let ok = unsafe { ProcessIdToSessionId(pid, &mut session_id) };
    let session = if ok != 0 { Some(session_id) } else { None };
    infer_app_type_from_session(process_name, session)
}
