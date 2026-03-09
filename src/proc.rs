use std::collections::HashMap;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{
    BOOL, CloseHandle, FALSE, GetLastError, HANDLE, HWND, INVALID_HANDLE_VALUE, LPARAM,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, QueryFullProcessImageNameW,
    TerminateProcess,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub exe_path: String,
    pub thread_cnt: u32,
    pub window_title: String,
}

type WindowMap = Vec<(u32, String)>;
static WINDOW_MAP: Mutex<Option<WindowMap>> = Mutex::new(None);

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, _: LPARAM) -> BOOL {
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }

    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return 1;
    }

    let title = OsString::from_wide(&buf[..len as usize])
        .to_string_lossy()
        .to_string();
    if title.trim().is_empty() {
        return 1;
    }

    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
    }
    if pid == 0 {
        return 1;
    }

    if let Ok(mut guard) = WINDOW_MAP.lock()
        && let Some(map) = guard.as_mut()
    {
        // A process may own multiple windows; keep the first non-empty title.
        if !map.iter().any(|(p, _)| *p == pid) {
            map.push((pid, title));
        }
    }

    1
}

fn build_window_map() -> HashMap<u32, String> {
    if let Ok(mut guard) = WINDOW_MAP.lock() {
        *guard = Some(Vec::new());
    }

    unsafe {
        EnumWindows(Some(enum_windows_callback), 0);
    }

    WINDOW_MAP
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn snapshot_raw() -> Vec<(u32, u32, String, u32)> {
    let mut out = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap.is_null() {
            return out;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snap, &mut entry) == 0 {
            CloseHandle(snap);
            return out;
        }

        loop {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(260);
            let name = OsString::from_wide(&entry.szExeFile[..len])
                .to_string_lossy()
                .to_string();
            out.push((
                entry.th32ProcessID,
                entry.th32ParentProcessID,
                name,
                entry.cntThreads,
            ));
            if Process32NextW(snap, &mut entry) == 0 {
                break;
            }
        }
        CloseHandle(snap);
    }
    out
}

fn query_exe_path(pid: u32) -> String {
    unsafe {
        let h: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
        if h.is_null() {
            return String::new();
        }
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        if ok == 0 {
            return String::new();
        }
        OsString::from_wide(&buf[..len as usize])
            .to_string_lossy()
            .to_string()
    }
}

pub fn list_all(with_paths: bool) -> Vec<ProcInfo> {
    let windows = build_window_map();
    snapshot_raw()
        .into_iter()
        .filter(|(pid, _, _, _)| *pid > 4)
        .map(|(pid, ppid, name, thread_cnt)| ProcInfo {
            pid,
            ppid,
            name,
            exe_path: if with_paths {
                query_exe_path(pid)
            } else {
                String::new()
            },
            thread_cnt,
            window_title: windows.get(&pid).cloned().unwrap_or_default(),
        })
        .collect()
}

pub fn find_by_name(pattern: &str) -> Vec<ProcInfo> {
    let needle = pattern.to_lowercase();
    let mut list: Vec<ProcInfo> = list_all(true)
        .into_iter()
        .filter(|p| p.name.to_lowercase().contains(&needle))
        .collect();
    list.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.pid.cmp(&b.pid))
    });
    list
}

pub fn find_by_pid(pid: u32) -> Option<ProcInfo> {
    list_all(true).into_iter().find(|p| p.pid == pid)
}

pub fn find_by_window_title(pattern: &str) -> Vec<ProcInfo> {
    let needle = pattern.to_lowercase();
    let mut list: Vec<ProcInfo> = list_all(true)
        .into_iter()
        .filter(|p| p.window_title.to_lowercase().contains(&needle))
        .collect();
    list.sort_by(|a, b| {
        a.window_title
            .to_lowercase()
            .cmp(&b.window_title.to_lowercase())
            .then(a.pid.cmp(&b.pid))
    });
    list
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillResult {
    Ok,
    AccessDenied,
    NotFound,
    Error(u32),
}

pub fn kill_pid(pid: u32) -> KillResult {
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
        if h.is_null() {
            return match GetLastError() {
                5 => KillResult::AccessDenied,
                87 => KillResult::NotFound,
                code => KillResult::Error(code),
            };
        }
        let ok = TerminateProcess(h, 1);
        CloseHandle(h);
        if ok != 0 {
            KillResult::Ok
        } else {
            match GetLastError() {
                5 => KillResult::AccessDenied,
                87 => KillResult::NotFound,
                code => KillResult::Error(code),
            }
        }
    }
}
