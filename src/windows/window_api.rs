use windows_sys::Win32::Foundation::{BOOL, GetLastError, HWND, LPARAM, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, GetWindowLongW, GetWindowRect,
    IsWindowVisible, MoveWindow, SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongW,
    SetWindowPos, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, LWA_ALPHA, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
};

#[derive(Debug)]
pub(crate) enum WindowApiError {
    NotFound,
    OsError { action: &'static str, code: u32 },
}

impl WindowApiError {
    fn os_error(action: &'static str) -> Self {
        Self::OsError {
            action,
            code: unsafe { GetLastError() },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowRect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

impl WindowRect {
    pub(crate) fn width(&self) -> i32 {
        self.right - self.left
    }

    pub(crate) fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

struct WindowSearch {
    pid: u32,
    hwnd: isize,
}

unsafe extern "system" fn enum_windows_by_pid(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }

    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid); }
    if pid == 0 {
        return 1;
    }

    let search = unsafe { &mut *(lparam as *mut WindowSearch) };
    if pid != search.pid {
        return 1;
    }

    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return 1;
    }
    let title = String::from_utf16_lossy(&buf[..len as usize]);
    if title.trim().is_empty() {
        return 1;
    }

    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
    if ex_style & WS_EX_TOOLWINDOW != 0 {
        return 1;
    }

    search.hwnd = hwnd as isize;
    0
}

pub(crate) fn find_hwnd_by_pid(pid: u32) -> Result<isize, WindowApiError> {
    let mut search = WindowSearch { pid, hwnd: 0 };
    unsafe { EnumWindows(Some(enum_windows_by_pid), &mut search as *mut _ as LPARAM); }
    if search.hwnd == 0 {
        return Err(WindowApiError::NotFound);
    }
    Ok(search.hwnd)
}

fn get_window_rect_raw(hwnd: HWND) -> Result<RECT, WindowApiError> {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
    if ok == 0 {
        return Err(WindowApiError::os_error("GetWindowRect"));
    }
    Ok(rect)
}

pub(crate) fn get_window_rect(hwnd: isize) -> Result<WindowRect, WindowApiError> {
    let rect = get_window_rect_raw(hwnd as HWND)?;
    Ok(WindowRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

pub(crate) fn focus_window(hwnd: isize) -> Result<(), WindowApiError> {
    let ok = unsafe { SetForegroundWindow(hwnd as HWND) };
    if ok == 0 {
        return Err(WindowApiError::os_error("SetForegroundWindow"));
    }
    Ok(())
}

pub(crate) fn move_window(hwnd: isize, x: i32, y: i32) -> Result<(), WindowApiError> {
    let rect = get_window_rect_raw(hwnd as HWND)?;
    let ok = unsafe { MoveWindow(hwnd as HWND, x, y, rect.right - rect.left, rect.bottom - rect.top, 1) };
    if ok == 0 {
        return Err(WindowApiError::os_error("MoveWindow"));
    }
    Ok(())
}

pub(crate) fn resize_window(hwnd: isize, width: i32, height: i32) -> Result<(), WindowApiError> {
    let rect = get_window_rect_raw(hwnd as HWND)?;
    let ok = unsafe { MoveWindow(hwnd as HWND, rect.left, rect.top, width, height, 1) };
    if ok == 0 {
        return Err(WindowApiError::os_error("MoveWindow"));
    }
    Ok(())
}


pub(crate) fn set_topmost(hwnd: isize, enable: bool) -> Result<(), WindowApiError> {
    let insert_after = if enable { HWND_TOPMOST } else { HWND_NOTOPMOST };
    let ok = unsafe {
        SetWindowPos(
            hwnd as HWND,
            insert_after,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE,
        )
    };
    if ok == 0 {
        return Err(WindowApiError::os_error("SetWindowPos"));
    }
    Ok(())
}

pub(crate) fn set_transparency(hwnd: isize, alpha: u8) -> Result<(), WindowApiError> {
    let style = unsafe { GetWindowLongW(hwnd as HWND, GWL_EXSTYLE) };
    unsafe {
        SetWindowLongW(hwnd as HWND, GWL_EXSTYLE, style | WS_EX_LAYERED as i32);
    }
    let ok = unsafe { SetLayeredWindowAttributes(hwnd as HWND, 0, alpha, LWA_ALPHA) };
    if ok == 0 {
        return Err(WindowApiError::os_error("SetLayeredWindowAttributes"));
    }
    Ok(())
}

pub(crate) fn apply_window_rect(hwnd: isize, rect: WindowRect) -> Result<(), WindowApiError> {
    let ok = unsafe {
        SetWindowPos(
            hwnd as HWND,
            std::ptr::null_mut(),
            rect.left,
            rect.top,
            rect.width(),
            rect.height(),
            SWP_NOZORDER,
        )
    };
    if ok == 0 {
        return Err(WindowApiError::os_error("SetWindowPos"));
    }
    Ok(())
}
