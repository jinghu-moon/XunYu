use std::collections::{HashMap, HashSet};

use windows_sys::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6TABLE_OWNER_PID, MIB_TCPTABLE_OWNER_PID,
    MIB_UDP6TABLE_OWNER_PID, MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    UDP_TABLE_OWNER_PID,
};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};

use super::filter::dedupe_raw;
use super::model::{PortInfo, Protocol, RawPortEntry};
use super::process_map::{
    process_name_from_path, query_process_command_line_and_cwd, query_process_path,
};

pub(super) fn port_from_be(raw: u32) -> u16 {
    u16::from_be(raw as u16)
}

fn enrich_ports(raw: Vec<RawPortEntry>) -> Vec<PortInfo> {
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

fn read_tcp_v4() -> Vec<RawPortEntry> {
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

fn read_tcp_v6() -> Vec<RawPortEntry> {
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

fn read_udp_v4() -> Vec<RawPortEntry> {
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

fn read_udp_v6() -> Vec<RawPortEntry> {
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

pub(super) fn list_tcp_listeners() -> Vec<PortInfo> {
    let mut raw = Vec::new();
    raw.extend(read_tcp_v4());
    raw.extend(read_tcp_v6());
    dedupe_raw(&mut raw);
    enrich_ports(raw)
}

pub(super) fn list_udp_endpoints() -> Vec<PortInfo> {
    let mut raw = Vec::new();
    raw.extend(read_udp_v4());
    raw.extend(read_udp_v6());
    dedupe_raw(&mut raw);
    enrich_ports(raw)
}
