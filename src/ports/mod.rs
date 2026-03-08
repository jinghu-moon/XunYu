mod collect;
mod filter;
mod model;
mod process_map;

pub use model::{PortInfo, Protocol};

pub fn list_tcp_listeners() -> Vec<PortInfo> {
    collect::list_tcp_listeners()
}

pub fn list_udp_endpoints() -> Vec<PortInfo> {
    collect::list_udp_endpoints()
}

pub fn terminate_pid(pid: u32) -> Result<(), String> {
    filter::terminate_pid(pid)
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
        assert_eq!(collect::port_from_be(0x0000_901F), 8080);
    }

    #[test]
    fn process_name_from_path_extracts_basename_or_falls_back_to_pid() {
        assert_eq!(process_map::process_name_from_path("", 123), "pid 123");
        assert_eq!(
            process_map::process_name_from_path("C:\\Windows\\System32\\cmd.exe", 1),
            "cmd.exe"
        );
    }

    #[test]
    fn terminate_pid_error_message_maps_known_win32_codes() {
        assert_eq!(filter::terminate_pid_error_message(5), "access denied");
        assert_eq!(filter::terminate_pid_error_message(87), "not found");
        assert_eq!(filter::terminate_pid_error_message(12345), "error 12345");
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
