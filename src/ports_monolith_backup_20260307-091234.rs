use std::collections::{HashMap, HashSet};
use std::ffi::{OsString, c_void};
use std::mem::size_of;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, GetLastError, HANDLE,
};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6TABLE_OWNER_PID, MIB_TCPTABLE_OWNER_PID,
    MIB_UDP6TABLE_OWNER_PID, MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    UDP_TABLE_OWNER_PID,
};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, PROCESS_VM_READ,
    QueryFullProcessImageNameW, TerminateProcess,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub pid: u32,
    pub name: String,
    pub exe_path: String,
    pub cmdline: String,
    pub cwd: String,
    pub protocol: Protocol,
}

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

fn port_from_be(raw: u32) -> u16 {
    u16::from_be(raw as u16)
}

fn query_process_path(pid: u32) -> String {
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

fn query_process_command_line_and_cwd(pid: u32) -> (String, String) {
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

fn process_name_from_path(path: &str, pid: u32) -> String {
    if path.is_empty() {
        return format!("pid {}", pid);
    }
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

fn enrich_ports(raw: Vec<(u16, u32, Protocol)>) -> Vec<PortInfo> {
    let mut pids: HashSet<u32> = HashSet::new();
    for (_, pid, _) in &raw {
        if *pid > 0 {
            pids.insert(*pid);
        }
    }

    let mut cache: HashMap<u32, (String, String, String, String)> = HashMap::new();
    for pid in pids {
        let path = query_process_path(pid);
        let name = process_name_from_path(&path, pid);
        let (cmdline, cwd) = query_process_command_line_and_cwd(pid);
        cache.insert(pid, (name, path, cmdline, cwd));
    }

    raw.into_iter()
        .map(|(port, pid, protocol)| {
            let (name, exe_path, cmdline, cwd) = cache.get(&pid).cloned().unwrap_or_else(|| {
                (
                    format!("pid {}", pid),
                    String::new(),
                    String::new(),
                    String::new(),
                )
            });
            PortInfo {
                port,
                pid,
                name,
                exe_path,
                cmdline,
                cwd,
                protocol,
            }
        })
        .collect()
}

fn read_tcp_v4() -> Vec<(u16, u32, Protocol)> {
    unsafe {
        let mut size: u32 = 0;
        let mut ret = GetExtendedTcpTable(
            std::ptr::null_mut(),
            &mut size,
            0,
            AF_INET as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
        if ret != ERROR_INSUFFICIENT_BUFFER {
            return Vec::new();
        }
        let mut buf = vec![0u8; size as usize];
        ret = GetExtendedTcpTable(
            buf.as_mut_ptr() as *mut _,
            &mut size,
            0,
            AF_INET as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
        if ret != 0 {
            return Vec::new();
        }
        let table = buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID;
        let entries = (*table).dwNumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).table.as_ptr(), entries);
        rows.iter()
            .map(|r| (port_from_be(r.dwLocalPort), r.dwOwningPid, Protocol::Tcp))
            .collect()
    }
}

fn read_tcp_v6() -> Vec<(u16, u32, Protocol)> {
    unsafe {
        let mut size: u32 = 0;
        let mut ret = GetExtendedTcpTable(
            std::ptr::null_mut(),
            &mut size,
            0,
            AF_INET6 as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
        if ret != ERROR_INSUFFICIENT_BUFFER {
            return Vec::new();
        }
        let mut buf = vec![0u8; size as usize];
        ret = GetExtendedTcpTable(
            buf.as_mut_ptr() as *mut _,
            &mut size,
            0,
            AF_INET6 as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
        if ret != 0 {
            return Vec::new();
        }
        let table = buf.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID;
        let entries = (*table).dwNumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).table.as_ptr(), entries);
        rows.iter()
            .map(|r| (port_from_be(r.dwLocalPort), r.dwOwningPid, Protocol::Tcp))
            .collect()
    }
}

fn read_udp_v4() -> Vec<(u16, u32, Protocol)> {
    unsafe {
        let mut size: u32 = 0;
        let mut ret = GetExtendedUdpTable(
            std::ptr::null_mut(),
            &mut size,
            0,
            AF_INET as u32,
            UDP_TABLE_OWNER_PID,
            0,
        );
        if ret != ERROR_INSUFFICIENT_BUFFER {
            return Vec::new();
        }
        let mut buf = vec![0u8; size as usize];
        ret = GetExtendedUdpTable(
            buf.as_mut_ptr() as *mut _,
            &mut size,
            0,
            AF_INET as u32,
            UDP_TABLE_OWNER_PID,
            0,
        );
        if ret != 0 {
            return Vec::new();
        }
        let table = buf.as_ptr() as *const MIB_UDPTABLE_OWNER_PID;
        let entries = (*table).dwNumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).table.as_ptr(), entries);
        rows.iter()
            .map(|r| (port_from_be(r.dwLocalPort), r.dwOwningPid, Protocol::Udp))
            .collect()
    }
}

fn read_udp_v6() -> Vec<(u16, u32, Protocol)> {
    unsafe {
        let mut size: u32 = 0;
        let mut ret = GetExtendedUdpTable(
            std::ptr::null_mut(),
            &mut size,
            0,
            AF_INET6 as u32,
            UDP_TABLE_OWNER_PID,
            0,
        );
        if ret != ERROR_INSUFFICIENT_BUFFER {
            return Vec::new();
        }
        let mut buf = vec![0u8; size as usize];
        ret = GetExtendedUdpTable(
            buf.as_mut_ptr() as *mut _,
            &mut size,
            0,
            AF_INET6 as u32,
            UDP_TABLE_OWNER_PID,
            0,
        );
        if ret != 0 {
            return Vec::new();
        }
        let table = buf.as_ptr() as *const MIB_UDP6TABLE_OWNER_PID;
        let entries = (*table).dwNumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).table.as_ptr(), entries);
        rows.iter()
            .map(|r| (port_from_be(r.dwLocalPort), r.dwOwningPid, Protocol::Udp))
            .collect()
    }
}

pub fn list_tcp_listeners() -> Vec<PortInfo> {
    let mut raw = Vec::new();
    raw.extend(read_tcp_v4());
    raw.extend(read_tcp_v6());
    let mut seen: HashSet<(u16, u32, Protocol)> = HashSet::new();
    raw.retain(|r| seen.insert(*r));
    enrich_ports(raw)
}

pub fn list_udp_endpoints() -> Vec<PortInfo> {
    let mut raw = Vec::new();
    raw.extend(read_udp_v4());
    raw.extend(read_udp_v6());
    let mut seen: HashSet<(u16, u32, Protocol)> = HashSet::new();
    raw.retain(|r| seen.insert(*r));
    enrich_ports(raw)
}

fn terminate_pid_error_message(code: u32) -> String {
    match code {
        5 => "access denied".to_string(),
        87 => "not found".to_string(),
        e => format!("error {}", e),
    }
}

pub fn terminate_pid(pid: u32) -> Result<(), String> {
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if h.is_null() {
            return Err(terminate_pid_error_message(GetLastError()));
        }
        let ok = TerminateProcess(h, 1);
        CloseHandle(h);
        if ok != 0 {
            Ok(())
        } else {
            Err(terminate_pid_error_message(GetLastError()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::net::{TcpListener, UdpSocket};
    use std::time::Duration;

    #[test]
    fn port_from_be_swaps_endianness() {
        // 0x901F -> 0x1F90 (8080)
        assert_eq!(port_from_be(0x0000_901F), 8080);
    }

    #[test]
    fn process_name_from_path_extracts_basename_or_falls_back_to_pid() {
        assert_eq!(process_name_from_path("", 123), "pid 123");
        assert_eq!(
            process_name_from_path("C:\\Windows\\System32\\cmd.exe", 1),
            "cmd.exe"
        );
    }

    #[test]
    fn terminate_pid_error_message_maps_known_win32_codes() {
        assert_eq!(terminate_pid_error_message(5), "access denied");
        assert_eq!(terminate_pid_error_message(87), "not found");
        assert_eq!(terminate_pid_error_message(12345), "error 12345");
    }

    #[test]
    fn list_tcp_listeners_contains_self_bound_port_and_is_deduped() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::sleep(Duration::from_millis(100));

        let ports = list_tcp_listeners();
        assert!(
            ports.iter().any(|p| p.port == port
                && p.pid == std::process::id()
                && p.protocol == Protocol::Tcp),
            "expected to find listener port {port}"
        );

        let uniq: HashSet<(u16, u32, Protocol)> =
            ports.iter().map(|p| (p.port, p.pid, p.protocol)).collect();
        assert_eq!(uniq.len(), ports.len(), "expected deduped tuples");
        drop(l);
    }

    #[test]
    fn list_udp_endpoints_contains_self_bound_port_and_is_deduped() {
        let s = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = s.local_addr().unwrap().port();
        std::thread::sleep(Duration::from_millis(100));

        let ports = list_udp_endpoints();
        assert!(
            ports.iter().any(|p| p.port == port
                && p.pid == std::process::id()
                && p.protocol == Protocol::Udp),
            "expected to find udp port {port}"
        );

        let uniq: HashSet<(u16, u32, Protocol)> =
            ports.iter().map(|p| (p.port, p.pid, p.protocol)).collect();
        assert_eq!(uniq.len(), ports.len(), "expected deduped tuples");
        drop(s);
    }

    #[test]
    fn terminate_pid_invalid_pid_returns_error() {
        let err = terminate_pid(u32::MAX).expect_err("expected invalid pid error");
        assert!(!err.trim().is_empty());
    }
}
