use std::ffi::{OsString, c_void};
use std::mem::size_of;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ, QueryFullProcessImageNameW,
};

const PROCESS_BASIC_INFORMATION_CLASS: u32 = 0;
const MAX_UNICODE_BYTES: usize = 32 * 1024;

#[repr(C)]
struct PROCESS_BASIC_INFORMATION {
    reserved1: usize,
    peb_base_address: *mut PEB,
    reserved2: [usize; 2],
    unique_process_id: usize,
    reserved3: usize,
}

#[repr(C)]
struct UNICODE_STRING {
    length: u16,
    maximum_length: u16,
    buffer: *const u16,
}

#[repr(C)]
struct CURDIR {
    dos_path: UNICODE_STRING,
    handle: usize,
}

#[repr(C)]
struct RTL_USER_PROCESS_PARAMETERS {
    maximum_length: u32,
    length: u32,
    flags: u32,
    debug_flags: u32,
    console_handle: usize,
    console_flags: u32,
    standard_input: usize,
    standard_output: usize,
    standard_error: usize,
    current_directory: CURDIR,
    dll_path: UNICODE_STRING,
    image_path_name: UNICODE_STRING,
    command_line: UNICODE_STRING,
}

#[repr(C)]
struct PEB {
    reserved1: [u8; 2],
    being_debugged: u8,
    reserved2: [u8; 1],
    reserved3: [usize; 2],
    ldr: *mut c_void,
    process_parameters: *mut RTL_USER_PROCESS_PARAMETERS,
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQueryInformationProcess(
        process_handle: HANDLE,
        process_information_class: u32,
        process_information: *mut c_void,
        process_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

pub(super) fn query_process_path(pid: u32) -> String {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
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

fn read_process_struct<T>(handle: HANDLE, base: *const c_void, out: &mut T) -> bool {
    unsafe {
        let mut read = 0usize;
        let size = size_of::<T>();
        let ok = ReadProcessMemory(
            handle,
            base,
            out as *mut _ as *mut c_void,
            size,
            &mut read as *mut usize,
        );
        ok != 0 && read >= size
    }
}

fn read_unicode_string(handle: HANDLE, s: UNICODE_STRING) -> String {
    if s.length == 0 || s.buffer.is_null() {
        return String::new();
    }
    let mut len = s.length as usize;
    if len > MAX_UNICODE_BYTES {
        len = MAX_UNICODE_BYTES;
    }
    len -= len % 2;
    if len == 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len / 2];
    unsafe {
        let mut read = 0usize;
        let ok = ReadProcessMemory(
            handle,
            s.buffer as *const c_void,
            buf.as_mut_ptr() as *mut c_void,
            len,
            &mut read as *mut usize,
        );
        if ok == 0 || read == 0 {
            return String::new();
        }
        let read_u16 = (read / 2).min(buf.len());
        OsString::from_wide(&buf[..read_u16])
            .to_string_lossy()
            .to_string()
    }
}

pub(super) fn query_process_command_line_and_cwd(pid: u32) -> (String, String) {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, pid);
        if h.is_null() {
            return (String::new(), String::new());
        }
        let mut pbi: PROCESS_BASIC_INFORMATION = std::mem::zeroed();
        let mut ret_len: u32 = 0;
        let status = NtQueryInformationProcess(
            h,
            PROCESS_BASIC_INFORMATION_CLASS,
            &mut pbi as *mut _ as *mut c_void,
            size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            &mut ret_len as *mut u32,
        );
        if status != 0 || pbi.peb_base_address.is_null() {
            CloseHandle(h);
            return (String::new(), String::new());
        }

        let mut peb: PEB = std::mem::zeroed();
        if !read_process_struct(h, pbi.peb_base_address as *const c_void, &mut peb) {
            CloseHandle(h);
            return (String::new(), String::new());
        }
        if peb.process_parameters.is_null() {
            CloseHandle(h);
            return (String::new(), String::new());
        }

        let mut params: RTL_USER_PROCESS_PARAMETERS = std::mem::zeroed();
        if !read_process_struct(h, peb.process_parameters as *const c_void, &mut params) {
            CloseHandle(h);
            return (String::new(), String::new());
        }

        let cmdline = read_unicode_string(h, params.command_line);
        let cwd = read_unicode_string(h, params.current_directory.dos_path);
        CloseHandle(h);
        (cmdline, cwd)
    }
}

pub(super) fn process_name_from_path(path: &str, pid: u32) -> String {
    if path.is_empty() {
        return format!("pid {}", pid);
    }
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}
