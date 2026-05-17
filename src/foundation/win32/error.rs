#![allow(dead_code)]
use windows_sys::Win32::Foundation::GetLastError;

#[derive(Debug)]
pub struct Win32Error {
    pub code: u32,
    pub message: String,
}

impl Win32Error {
    pub fn last() -> Self {
        let code = unsafe { GetLastError() };
        Self {
            code,
            message: format!("Win32 error: {}", code),
        }
    }
}

impl std::fmt::Display for Win32Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Win32Error {}

pub fn check_bool(ret: i32) -> Result<(), Win32Error> {
    if ret != 0 {
        Ok(())
    } else {
        Err(Win32Error::last())
    }
}

pub fn check_handle(ret: isize) -> Result<isize, Win32Error> {
    // Usually 0 or -1 (INVALID_HANDLE_VALUE) are errors depending on API
    if ret != 0 && ret != -1 {
        Ok(ret)
    } else {
        Err(Win32Error::last())
    }
}
