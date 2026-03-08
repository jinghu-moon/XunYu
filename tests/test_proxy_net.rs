#![cfg(windows)]

mod common;

use common::*;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::net::{TcpListener, UdpSocket};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn is_dev_port(port: u16) -> bool {
    (3000..=3999).contains(&port)
        || (5000..=5999).contains(&port)
        || (8000..=8999).contains(&port)
        || port == 4173
        || port == 5173
}

fn bind_free_dev_port() -> TcpListener {
    for port in 5500..=5599 {
        if let Ok(l) = TcpListener::bind(("127.0.0.1", port)) {
            return l;
        }
    }
    panic!("no free dev port in 5500-5599");
}

fn bind_free_non_dev_port() -> TcpListener {
    for _ in 0..32 {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        if !is_dev_port(port) {
            return l;
        }
    }
    panic!("failed to bind a non-dev ephemeral port");
}

#[test]
fn proxy_test_invalid_url() {
    let env = TestEnv::new();
    let output = run_ok(env.cmd().args(["proxy", "test", "not-a-url"]));
    let s = String::from_utf8_lossy(&output.stdout);
    let first = s.lines().next().unwrap_or("");
    assert!(first.starts_with("proxy\t-1\t"));
    assert!(first.contains("invalid proxy url"));
}

#[test]
fn proxy_only_invalid_fails() {
    let env = TestEnv::new();
    let out = run_err(
        env.cmd()
            .args(["proxy", "set", "http://127.0.0.1:7897", "--only", "nope"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Invalid --only value"));
}

#[test]
fn proxy_test_fake_server_ok() {
    let env = TestEnv::new();
    let (url, handle) = start_fake_proxy(3);

    let output = run_ok(env.cmd().args([
        "proxy",
        "test",
        url.as_str(),
        "--timeout",
        "2",
        "--jobs",
        "3",
    ]));
    let s = String::from_utf8_lossy(&output.stdout);
    let mut labels = HashSet::new();
    for line in s.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[2], "ok");
        labels.insert(parts[0].to_string());
    }
    assert!(labels.contains("proxy"));
    assert!(labels.contains("8.8.8.8"));
    assert!(labels.contains("1.1.1.1"));

    let _ = handle.join();
}

#[test]
fn proxy_detect_outputs_status() {
    let env = TestEnv::new();
    let output = run_ok(env.cmd().args(["proxy", "detect"]));
    let s = String::from_utf8_lossy(&output.stdout);
    assert!(
        s.starts_with("enabled\t") || s.starts_with("disabled\t"),
        "unexpected output: {}",
        s
    );
}

#[test]
fn proxy_test_custom_targets() {
    let env = TestEnv::new();
    let (url, handle) = start_fake_proxy(1);

    let output = run_ok(env.cmd().args([
        "proxy",
        "test",
        url.as_str(),
        "--targets",
        "proxy",
        "--timeout",
        "1",
        "--jobs",
        "1",
    ]));
    let s = String::from_utf8_lossy(&output.stdout);
    let first = s.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split('\t').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "proxy");
    assert_eq!(parts[2], "ok");

    let _ = handle.join();
}

#[test]
fn proxy_set_only_cargo() {
    let env = TestEnv::new();
    run_ok(
        env.cmd()
            .args(["proxy", "set", "http://127.0.0.1:7897", "--only", "cargo"]),
    );

    let cargo_cfg = env.root.join(".cargo/config.toml");
    let content = fs::read_to_string(&cargo_cfg).unwrap_or_default();
    assert!(content.contains("proxy = \"http://127.0.0.1:7897\""));

    // Ensure `--only cargo` doesn't touch other configs.
    assert!(!env.root.join(".gitconfig").exists());
    assert!(!env.root.join(".npmrc").exists());

    run_ok(env.cmd().args(["proxy", "del", "--only", "cargo"]));
    let content = fs::read_to_string(&cargo_cfg).unwrap_or_default();
    assert!(!content.contains("proxy = \"http://127.0.0.1:7897\""));
}

#[test]
fn proxy_set_persists_state() {
    let env = TestEnv::new();
    run_ok(env.cmd().args(["proxy", "set", "http://127.0.0.1:7897"]));

    let state = env.root.join(".xun.proxy.json");
    let content = fs::read_to_string(&state).unwrap_or_default();
    assert!(content.contains("127.0.0.1:7897"));
}

#[test]
fn ports_all_includes_tcp_listener() {
    let env = TestEnv::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::sleep(Duration::from_millis(50));

    let output = run_ok(env.cmd().args(["ports", "--all"]));
    let s = String::from_utf8_lossy(&output.stdout);
    let needle = format!("{}\t", port);
    assert!(s.lines().any(|l| l.starts_with(&needle)));
}

#[test]
fn ports_udp_includes_socket() {
    let env = TestEnv::new();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    thread::sleep(Duration::from_millis(50));

    let output = run_ok(env.cmd().args(["ports", "--udp"]));
    let s = String::from_utf8_lossy(&output.stdout);
    let needle = format!("{}\t", port);
    assert!(s.lines().any(|l| l.starts_with(&needle)));
}

#[test]
fn ports_range_filters_tcp() {
    let env = TestEnv::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::sleep(Duration::from_millis(50));

    let range = format!("{}-{}", port, port);
    let output = run_ok(
        env.cmd()
            .args(["ports", "--all", "--range", range.as_str()]),
    );
    let s = String::from_utf8_lossy(&output.stdout);
    let needle = format!("{}\t", port);
    assert!(s.lines().any(|l| l.starts_with(&needle)));
}

#[test]
fn ports_pid_filters_tcp() {
    let env = TestEnv::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::sleep(Duration::from_millis(50));

    let pid = std::process::id().to_string();
    let output = run_ok(env.cmd().args(["ports", "--all", "--pid", pid.as_str()]));
    let s = String::from_utf8_lossy(&output.stdout);
    let needle = format!("{}\t", port);
    assert!(s.lines().any(|l| l.starts_with(&needle)));
}

#[test]
fn ports_default_filters_to_dev_ports() {
    let env = TestEnv::new();
    let dev = bind_free_dev_port();
    let dev_port = dev.local_addr().unwrap().port();
    let non_dev = bind_free_non_dev_port();
    let non_dev_port = non_dev.local_addr().unwrap().port();
    thread::sleep(Duration::from_millis(50));

    let out = run_ok(env.cmd().args(["ports", "--format", "json"]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let ports: Vec<u64> = v
        .as_array()
        .unwrap_or_else(|| panic!("expected array, got: {v}"))
        .iter()
        .filter_map(|row| row.get("port").and_then(Value::as_u64))
        .collect();

    assert!(
        ports.contains(&(dev_port as u64)),
        "expected dev port in list"
    );
    assert!(
        !ports.contains(&(non_dev_port as u64)),
        "expected non-dev port to be filtered out"
    );
}

#[test]
fn ports_name_filters_case_insensitive() {
    let env = TestEnv::new();
    let dev = bind_free_dev_port();
    let dev_port = dev.local_addr().unwrap().port();
    thread::sleep(Duration::from_millis(50));

    let out = run_ok(env.cmd().args([
        "ports",
        "--all",
        "--name",
        "TEST_PROXY_NET",
        "--format",
        "json",
    ]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v
        .as_array()
        .unwrap_or_else(|| panic!("expected array, got: {v}"));
    assert!(!arr.is_empty(), "expected at least one matching port");
    assert!(
        arr.iter()
            .any(|row| row.get("port").and_then(Value::as_u64) == Some(dev_port as u64)),
        "expected our dev port to be present"
    );
    assert!(arr.iter().all(|row| {
        row.get("name")
            .and_then(Value::as_str)
            .map(|s| s.to_lowercase().contains("test_proxy_net"))
            .unwrap_or(false)
    }));
}

#[test]
fn ports_invalid_range_fails() {
    let env = TestEnv::new();
    let out = run_err(env.cmd().args(["ports", "--range", "1-"]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Invalid range"));
}

#[test]
fn proxy_detect_invalid_format_fails() {
    let env = TestEnv::new();
    let out = run_err(env.cmd().args(["proxy", "detect", "--format", "nope"]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Invalid format"));
}

#[test]
fn kill_invalid_port_fails() {
    let env = TestEnv::new();
    let out = run_err(env.cmd().args(["kill", "abc"]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Invalid port"));
}

#[test]
fn kill_force_parses_comma_list_and_terminates_process() {
    let env = TestEnv::new();
    let ports_file = env.root.join("kill-ports.txt");
    let out_path = ports_file.to_string_lossy().replace('\'', "''");

    // Bind two TCP listeners in a child process and write the chosen ports to a file for the test.
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         $out='{out_path}'; \
         $l1=[System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback,0); $l1.Start(); \
         $l2=[System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback,0); $l2.Start(); \
         $p1=$l1.LocalEndpoint.Port; $p2=$l2.LocalEndpoint.Port; \
         Set-Content -LiteralPath $out -Value \"$($p1),$($p2)\" -Encoding ASCII; \
         Start-Sleep -Seconds 300"
    );
    let child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    struct ChildGuard {
        child: std::process::Child,
    }
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if self.child.try_wait().ok().flatten().is_none() {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }
    let mut guard = ChildGuard { child };

    let mut ports: Vec<u16> = Vec::new();
    for _ in 0..60 {
        if let Ok(s) = fs::read_to_string(&ports_file) {
            ports = s
                .trim()
                .split(',')
                .filter_map(|p| p.trim().parse::<u16>().ok())
                .collect();
            if ports.len() == 2 {
                break;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        ports.len(),
        2,
        "expected ports file to contain two ports: {:?}",
        ports_file
    );

    let arg = format!("{}, {}", ports[0], ports[1]);
    run_ok(env.cmd().args(["kill", arg.as_str(), "--force"]));

    // Wait for termination and verify ports are released.
    for _ in 0..60 {
        if guard.child.try_wait().ok().flatten().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(
        guard.child.try_wait().ok().flatten().is_some(),
        "expected killed process to exit"
    );
    for p in ports {
        let rebind = TcpListener::bind(("127.0.0.1", p));
        assert!(rebind.is_ok(), "expected port {p} to be released");
    }
}

#[test]
fn pon_outputs_env_set_magic_lines() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args(["pon", "http://127.0.0.1:7897"]));
    let s = String::from_utf8_lossy(&out.stdout);
    let sets: Vec<&str> = s
        .lines()
        .filter(|l| l.starts_with("__ENV_SET__:"))
        .collect();
    assert_eq!(sets.len(), 8, "unexpected stdout:\n{s}");
    assert!(s.contains("__ENV_SET__:HTTP_PROXY="));
    assert!(s.contains("__ENV_SET__:NO_PROXY="));
}

#[test]
fn poff_outputs_env_unset_magic_lines() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args(["poff"]));
    let s = String::from_utf8_lossy(&out.stdout);
    let unsets: Vec<&str> = s
        .lines()
        .filter(|l| l.starts_with("__ENV_UNSET__:"))
        .collect();
    assert_eq!(unsets.len(), 8, "unexpected stdout:\n{s}");
    assert!(s.contains("__ENV_UNSET__:HTTP_PROXY"));
    assert!(s.contains("__ENV_UNSET__:NO_PROXY"));
}

#[test]
fn pon_without_scheme_adds_http_prefix() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args(["pon", "127.0.0.1:7897"]));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("__ENV_SET__:HTTP_PROXY=http://127.0.0.1:7897"));
}

#[test]
fn pon_without_url_auto_detects_or_falls_back() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args(["pon"]));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("__ENV_SET__:HTTP_PROXY=http"));

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("system proxy"),
        "unexpected stderr:\n{stderr}"
    );
}

#[test]
fn proxy_detect_json_outputs_object() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args(["proxy", "detect", "--format", "json"]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v.get("enabled").is_some());
    assert!(v.get("url").is_some());
}

#[test]
fn pst_json_has_four_rows_and_cargo_off_when_no_config() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args(["pst", "--format", "json"]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 4);

    let tools: HashSet<&str> = arr
        .iter()
        .filter_map(|o| o.get("tool").and_then(|v| v.as_str()))
        .collect();
    assert!(tools.contains("Env"));
    assert!(tools.contains("Git"));
    assert!(tools.contains("npm"));
    assert!(tools.contains("Cargo"));

    let cargo = arr
        .iter()
        .find(|o| o.get("tool").and_then(|v| v.as_str()) == Some("Cargo"))
        .expect("cargo row");
    assert_eq!(cargo.get("status").and_then(|v| v.as_str()), Some("OFF"));
}

#[test]
fn pst_reads_cargo_proxy_after_proxy_set_only_cargo() {
    let env = TestEnv::new();
    run_ok(
        env.cmd()
            .args(["proxy", "set", "http://127.0.0.1:7897", "--only", "cargo"]),
    );

    let out = run_ok(env.cmd().args(["pst", "--format", "json"]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().expect("array");
    let cargo = arr
        .iter()
        .find(|o| o.get("tool").and_then(|v| v.as_str()) == Some("Cargo"))
        .expect("cargo row");
    assert_eq!(cargo.get("status").and_then(|v| v.as_str()), Some("ON"));
    assert_eq!(
        cargo.get("address").and_then(|v| v.as_str()),
        Some("http://127.0.0.1:7897")
    );
}

#[test]
fn proxy_state_is_used_by_pon_when_no_url() {
    let env = TestEnv::new();
    run_ok(
        env.cmd()
            .args(["proxy", "set", "http://127.0.0.1:7897", "--only", "cargo"]),
    );

    let out = run_ok(env.cmd().args(["pon"]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("using saved proxy"),
        "unexpected stderr:\n{stderr}"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("__ENV_SET__:HTTP_PROXY=http://127.0.0.1:7897"));
}

#[test]
fn px_inherits_proxy_env_vars() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args([
        "px",
        "-u",
        "http://127.0.0.1:7897",
        "cmd",
        "/c",
        "echo",
        "%HTTP_PROXY%",
    ]));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("http://127.0.0.1:7897"));
}

#[test]
fn px_without_command_exits_2() {
    let env = TestEnv::new();
    let out = run_err(env.cmd().args(["px"]));
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("usage: px"));
}
