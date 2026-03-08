use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use comfy_table::{Attribute, Cell, Color, Table};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

use crate::cli::{
    ProxyCmd, ProxyExecCmd, ProxyOffCmd, ProxyOnCmd, ProxyStatusCmd, ProxySubCommand,
};
use crate::output::{apply_pretty_table_style, emit_warning};
use crate::util::has_cmd;

#[allow(dead_code)]
fn legacy_exit(code: i32) -> ! {
    panic!("legacy_exit:{code}");
}

fn legacy_error(code: i32, message: impl AsRef<str>, details: &[&str]) -> ! {
    let msg = message.as_ref();
    ui_println!("Error: {}", msg);
    if details.is_empty() {
        ui_println!("Hint: Run `xun --help` for usage.");
    } else {
        for d in details {
            ui_println!("{d}");
        }
    }
    legacy_exit(code);
}

fn out_env_set(key: &str, value: &str) {
    out_println!("__ENV_SET__:{}={}", key, value);
}

fn out_env_unset(key: &str) {
    out_println!("__ENV_UNSET__:{}", key);
}

fn get_system_proxy_url(fallback: &str) -> (String, bool) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    let key = hkcu.open_subkey(path);

    if let Ok(k) = key {
        let enabled: u32 = k.get_value("ProxyEnable").unwrap_or(0);
        let server: String = k.get_value("ProxyServer").unwrap_or_default();
        if enabled == 1 && !server.trim().is_empty() {
            let raw = server.trim();
            let candidate = if raw.contains("http=") {
                raw.split(';')
                    .find_map(|part| part.strip_prefix("http="))
                    .unwrap_or(raw)
                    .to_string()
            } else {
                raw.split(';').next().unwrap_or(raw).to_string()
            };
            let mut url = candidate;
            if !url.starts_with("http://") && !url.starts_with("https://") {
                url = format!("http://{}", url);
            }
            return (url, false);
        }
    }

    (fallback.to_string(), true)
}

fn parse_proxy_addr(url: &str) -> Option<SocketAddr> {
    let stripped = url
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/');
    stripped.to_socket_addrs().ok()?.next()
}

fn probe_proxy_alive(addr: SocketAddr, timeout: Duration) -> Result<u64, String> {
    let start = Instant::now();
    TcpStream::connect_timeout(&addr, timeout).map_err(|e| e.to_string())?;
    Ok(start.elapsed().as_millis() as u64)
}

fn probe_through_proxy(proxy: SocketAddr, target: &str, timeout: Duration) -> Result<u64, String> {
    let start = Instant::now();
    let mut s = TcpStream::connect_timeout(&proxy, timeout).map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(timeout)).ok();
    s.set_write_timeout(Some(timeout)).ok();

    let req = format!("CONNECT {} HTTP/1.1\r\nHost: {}\r\n\r\n", target, target);
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;

    let mut buf = [0u8; 128];
    let n = s.read(&mut buf).map_err(|e| e.to_string())?;
    let resp = std::str::from_utf8(&buf[..n]).unwrap_or("");

    if resp.contains("200") {
        Ok(start.elapsed().as_millis() as u64)
    } else {
        let first = resp.lines().next().unwrap_or("no response").trim();
        Err(first.to_string())
    }
}

#[derive(Clone)]
struct ProxyTarget {
    label: String,
    target: Option<String>,
}

fn default_proxy_targets() -> Vec<ProxyTarget> {
    vec![
        ProxyTarget {
            label: "proxy".to_string(),
            target: None,
        },
        ProxyTarget {
            label: "8.8.8.8".to_string(),
            target: Some("8.8.8.8:80".to_string()),
        },
        ProxyTarget {
            label: "1.1.1.1".to_string(),
            target: Some("1.1.1.1:80".to_string()),
        },
    ]
}

fn parse_proxy_targets(raw: Option<&str>) -> Vec<ProxyTarget> {
    let raw = raw.unwrap_or("");
    let mut out = Vec::new();
    for part in raw.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        if t.eq_ignore_ascii_case("proxy") {
            out.push(ProxyTarget {
                label: "proxy".to_string(),
                target: None,
            });
            continue;
        }
        let target = if t.contains(':') {
            t.to_string()
        } else {
            format!("{}:80", t)
        };
        let label = t.split(':').next().unwrap_or(t).to_string();
        out.push(ProxyTarget {
            label,
            target: Some(target),
        });
    }
    if out.is_empty() {
        default_proxy_targets()
    } else {
        out
    }
}

fn run_proxy_tests_with(
    proxy_url: &str,
    targets: Vec<ProxyTarget>,
    timeout: Duration,
    jobs: usize,
) -> Vec<(String, Result<u64, String>)> {
    let Some(addr) = parse_proxy_addr(proxy_url) else {
        return vec![("proxy".to_string(), Err("invalid proxy url".into()))];
    };

    let max_jobs = jobs.max(1);
    let mut results: Vec<Option<(String, Result<u64, String>)>> = vec![None; targets.len()];
    let mut idx = 0usize;

    while idx < targets.len() {
        let end = (idx + max_jobs).min(targets.len());
        let mut handles = Vec::new();
        for (i, t) in targets[idx..end].iter().cloned().enumerate() {
            let index = idx + i;
            let timeout = timeout;
            let addr = addr;
            handles.push((
                index,
                thread::spawn(move || {
                    let result = match t.target {
                        None => probe_proxy_alive(addr, timeout),
                        Some(target) => probe_through_proxy(addr, &target, timeout),
                    };
                    (t.label, result)
                }),
            ));
        }

        for (i, h) in handles {
            let res = h.join().unwrap_or(("unknown".into(), Err("thread panic".into())));
            results[i] = Some(res);
        }
        idx = end;
    }

    results
        .into_iter()
        .map(|r| r.unwrap_or(("unknown".into(), Err("thread panic".into()))))
        .collect()
}

fn run_proxy_tests(proxy_url: &str) -> Vec<(String, Result<u64, String>)> {
    run_proxy_tests_with(
        proxy_url,
        default_proxy_targets(),
        Duration::from_secs(5),
        3,
    )
}
fn parse_proxy_only(raw: Option<&str>) -> Result<Option<HashSet<String>>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let mut set = HashSet::new();
    for part in raw.split(',') {
        let t = part.trim().to_lowercase();
        if t.is_empty() {
            continue;
        }
        if t == "all" {
            return Ok(None);
        }
        match t.as_str() {
            "cargo" | "git" | "npm" | "msys2" => {
                set.insert(t);
            }
            _ => return Err(format!("Invalid --only value: {}", t)),
        }
    }
    if set.is_empty() {
        Ok(None)
    } else {
        Ok(Some(set))
    }
}

fn want_only(only: Option<&HashSet<String>>, key: &str) -> bool {
    only.map(|s| s.contains(key)).unwrap_or(true)
}

fn cargo_config_path() -> PathBuf {
    let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    Path::new(&home).join(".cargo/config.toml")
}

fn read_cargo_proxy() -> Option<String> {
    let path = cargo_config_path();
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    let mut in_http = false;
    for line in content.lines() {
        let tr = line.trim();
        if tr == "[http]" {
            in_http = true;
            continue;
        }
        if tr.starts_with('[') {
            in_http = false;
        }
        if in_http && tr.starts_with("proxy") {
            let parts: Vec<&str> = tr.split('=').collect();
            if parts.len() == 2 {
                return Some(parts[1].trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn msys2_proxy_path(root_override: Option<&str>) -> Option<PathBuf> {
    let userprofile = env::var("USERPROFILE").unwrap_or_default();
    let roots = vec![
        root_override.map(String::from),
        env::var("MSYS2_ROOT").ok(),
        Some(r"C:\msys64".to_string()),
        Some(r"C:\msys32".to_string()),
        Some(format!(r"{}\AppData\Local\msys64", userprofile)),
    ];

    roots
        .into_iter()
        .flatten()
        .map(|s| Path::new(&s).join(r"etc\profile.d\proxy.sh"))
        .find(|p| p.parent().map(|pa| pa.exists()).unwrap_or(false))
}

fn set_proxy(url: &str, noproxy: &str, msys2: Option<&str>, only: Option<&HashSet<String>>) {
    if want_only(only, "cargo") {
        let cargo_path = cargo_config_path();
        if let Some(p) = cargo_path.parent() {
            fs::create_dir_all(p).ok();
        }

        let content = fs::read_to_string(&cargo_path).unwrap_or_default();
        let new_line = format!("proxy = \"{}\"", url);
        let updated = if content.contains("[http]") {
            let mut lines: Vec<String> = content.lines().map(String::from).collect();
            let mut in_http = false;
            let mut found = false;
            let mut insert_at = None;
            for (i, line) in lines.iter_mut().enumerate() {
                let tr = line.trim();
                if tr == "[http]" {
                    in_http = true;
                    insert_at = Some(i + 1);
                    continue;
                }
                if tr.starts_with('[') {
                    in_http = false;
                }
                if in_http && tr.starts_with("proxy") {
                    *line = new_line.clone();
                    found = true;
                    break;
                }
            }
            if !found {
                if let Some(pos) = insert_at {
                    lines.insert(pos, new_line);
                } else {
                    lines.push("[http]".into());
                    lines.push(new_line);
                }
            }
            lines.join("\n")
        } else {
            format!("{}\n[http]\n{}\n", content, new_line)
        };
        fs::write(&cargo_path, updated).ok();
        ui_println!("ok:cargo");
    } else {
        ui_println!("skip:cargo (only)");
    }

    if want_only(only, "msys2") {
        if let Some(dest) = msys2_proxy_path(msys2) {
            let script = format!(
                "# managed by xun\nexport http_proxy=\"{url}\"\nexport https_proxy=\"{url}\"\nexport ftp_proxy=\"{url}\"\nexport no_proxy=\"{noproxy}\"\nexport HTTP_PROXY=\"{url}\"\nexport HTTPS_PROXY=\"{url}\"\nexport NO_PROXY=\"{noproxy}\"\n"
            );
            fs::write(&dest, script.replace("\r\n", "\n")).ok();
            ui_println!("ok:msys2");
        } else {
            ui_println!("skip:msys2 (not found)");
        }
    } else {
        ui_println!("skip:msys2 (only)");
    }

    if want_only(only, "git") {
        if has_cmd("git") {
            Command::new("git")
                .args(["config", "--global", "http.proxy", url])
                .output()
                .ok();
            Command::new("git")
                .args(["config", "--global", "https.proxy", url])
                .output()
                .ok();
            ui_println!("ok:git");
        } else {
            ui_println!("skip:git (not found)");
        }
    } else {
        ui_println!("skip:git (only)");
    }

    if want_only(only, "npm") {
        if has_cmd("npm") {
            Command::new("npm")
                .args(["config", "set", "proxy", url])
                .output()
                .ok();
            Command::new("npm")
                .args(["config", "set", "https-proxy", url])
                .output()
                .ok();
            ui_println!("ok:npm");
        } else {
            ui_println!("skip:npm (not found)");
        }
    } else {
        ui_println!("skip:npm (only)");
    }
}

fn del_proxy(msys2: Option<&str>, only: Option<&HashSet<String>>) {
    if want_only(only, "cargo") {
        let cargo_path = cargo_config_path();
        if cargo_path.exists() {
            let content = fs::read_to_string(&cargo_path).unwrap_or_default();
            let updated: String = content
                .lines()
                .filter(|l| !l.trim().starts_with("proxy"))
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(&cargo_path, updated).ok();
            ui_println!("ok:cargo");
        }
    } else {
        ui_println!("skip:cargo (only)");
    }

    if want_only(only, "msys2") {
        if let Some(dest) = msys2_proxy_path(msys2) {
            fs::remove_file(dest).ok();
            ui_println!("ok:msys2");
        }
    } else {
        ui_println!("skip:msys2 (only)");
    }

    if want_only(only, "git") {
        if has_cmd("git") {
            Command::new("git")
                .args(["config", "--global", "--unset", "http.proxy"])
                .output()
                .ok();
            Command::new("git")
                .args(["config", "--global", "--unset", "https.proxy"])
                .output()
                .ok();
            ui_println!("ok:git");
        } else {
            ui_println!("skip:git (not found)");
        }
    } else {
        ui_println!("skip:git (only)");
    }

    if want_only(only, "npm") {
        if has_cmd("npm") {
            Command::new("npm")
                .args(["config", "delete", "proxy"])
                .output()
                .ok();
            Command::new("npm")
                .args(["config", "delete", "https-proxy"])
                .output()
                .ok();
            ui_println!("ok:npm");
        } else {
            ui_println!("skip:npm (not found)");
        }
    } else {
        ui_println!("skip:npm (only)");
    }
}

pub(crate) fn cmd_proxy_on(args: ProxyOnCmd) {
    let fallback = "http://127.0.0.1:7897";
    let (auto_url, used_fallback) = get_system_proxy_url(fallback);
    let user_url = args.url;
    let user_provided = user_url.is_some();
    let mut proxy_url = user_url.unwrap_or(auto_url);
    if !proxy_url.starts_with("http://") && !proxy_url.starts_with("https://") {
        proxy_url = format!("http://{}", proxy_url);
    }

    if used_fallback && !user_provided {
        ui_println!("~ system proxy disabled, using fallback {}", proxy_url);
    } else if !user_provided {
        ui_println!("~ detected system proxy {}", proxy_url);
    }

    let noproxy = args.noproxy;
    out_env_set("HTTP_PROXY", &proxy_url);
    out_env_set("HTTPS_PROXY", &proxy_url);
    out_env_set("ALL_PROXY", &proxy_url);
    out_env_set("NO_PROXY", &noproxy);
    out_env_set("http_proxy", &proxy_url);
    out_env_set("https_proxy", &proxy_url);
    out_env_set("all_proxy", &proxy_url);
    out_env_set("no_proxy", &noproxy);

    set_proxy(&proxy_url, &noproxy, args.msys2.as_deref(), None);
}

pub(crate) fn cmd_proxy_off(args: ProxyOffCmd) {
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
    ] {
        out_env_unset(key);
    }
    del_proxy(args.msys2.as_deref(), None);
}

pub(crate) fn cmd_proxy_detect() {
    let fallback = "http://127.0.0.1:7897";
    let (url, used_fallback) = get_system_proxy_url(fallback);
    if used_fallback {
        out_println!("disabled\t");
    } else {
        out_println!("enabled\t{}", url);
    }
}

pub(crate) fn cmd_proxy_status(_args: ProxyStatusCmd) {
    let env_proxy = env::var("HTTP_PROXY")
        .or_else(|_| env::var("http_proxy"))
        .ok();
    let env_noproxy = env::var("NO_PROXY")
        .or_else(|_| env::var("no_proxy"))
        .ok();

    let git_proxy = if has_cmd("git") {
        Command::new("git")
            .args(["config", "--global", "--get", "http.proxy"])
            .output()
            .ok()
            .and_then(|o| {
                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if v.is_empty() || v == "null" { None } else { Some(v) }
            })
    } else {
        None
    };

    let npm_proxy = if has_cmd("npm") {
        Command::new("npm")
            .args(["config", "get", "proxy"])
            .output()
            .ok()
            .and_then(|o| {
                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if v.is_empty() || v == "null" { None } else { Some(v) }
            })
    } else {
        None
    };

    let cargo_proxy = read_cargo_proxy();

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Tool").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Status").add_attribute(Attribute::Bold).fg(Color::Green),
        Cell::new("Address").add_attribute(Attribute::Bold).fg(Color::Magenta),
        Cell::new("Note").add_attribute(Attribute::Bold).fg(Color::Yellow),
    ]);

    let env_state = env_proxy.is_some();
    table.add_row(vec![
        Cell::new("Env"),
        Cell::new(if env_state { "ON" } else { "OFF" })
            .fg(if env_state { Color::Green } else { Color::DarkGrey }),
        Cell::new(env_proxy.clone().unwrap_or_else(|| "-".into()))
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(env_noproxy.unwrap_or_else(|| "-".into()))
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);

    let git_state = git_proxy.is_some();
    table.add_row(vec![
        Cell::new("Git"),
        Cell::new(if git_state { "ON" } else { "OFF" })
            .fg(if git_state { Color::Green } else { Color::DarkGrey }),
        Cell::new(git_proxy.clone().unwrap_or_else(|| "-".into()))
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(if has_cmd("git") { "" } else { "not found" })
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);

    let npm_state = npm_proxy.is_some();
    table.add_row(vec![
        Cell::new("npm"),
        Cell::new(if npm_state { "ON" } else { "OFF" })
            .fg(if npm_state { Color::Green } else { Color::DarkGrey }),
        Cell::new(npm_proxy.clone().unwrap_or_else(|| "-".into()))
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(if has_cmd("npm") { "" } else { "not found" })
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);

    let cargo_state = cargo_proxy.is_some();
    table.add_row(vec![
        Cell::new("Cargo"),
        Cell::new(if cargo_state { "ON" } else { "OFF" })
            .fg(if cargo_state { Color::Green } else { Color::DarkGrey }),
        Cell::new(cargo_proxy.clone().unwrap_or_else(|| "-".into()))
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(if cargo_state { "config.toml" } else { "" })
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);

    ui_println!("{}", table);

    if let Some(proxy_url) = env_proxy {
        let results = run_proxy_tests(&proxy_url);
        let mut t = Table::new();
        apply_pretty_table_style(&mut t);
        t.set_header(vec![
            Cell::new("Target").add_attribute(Attribute::Bold).fg(Color::Cyan),
            Cell::new("Latency").add_attribute(Attribute::Bold).fg(Color::Green),
            Cell::new("Detail").add_attribute(Attribute::Bold).fg(Color::Yellow),
        ]);
        for (label, result) in results {
            match result {
                Ok(ms) => {
                    t.add_row(vec![
                        Cell::new(label),
                        Cell::new(format!("{}ms", ms)).fg(Color::Green),
                        Cell::new("ok").fg(Color::DarkGrey).add_attribute(Attribute::Dim),
                    ]);
                }
                Err(e) => {
                    t.add_row(vec![
                        Cell::new(label).fg(Color::Red),
                        Cell::new("-").fg(Color::Red),
                        Cell::new(e).fg(Color::Red),
                    ]);
                }
            }
        }
        ui_println!("{}", t);
    }
}

pub(crate) fn cmd_proxy_exec(args: ProxyExecCmd) {
    if args.cmd.is_empty() {
        legacy_error(
            2,
            "Missing command for px.",
            &["Fix: Usage: px <command> [args]."],
        );
    }

    let fallback = "http://127.0.0.1:7897";
    let (auto_url, _) = get_system_proxy_url(fallback);
    let mut proxy_url = args.url.unwrap_or(auto_url);
    if !proxy_url.starts_with("http://") && !proxy_url.starts_with("https://") {
        proxy_url = format!("http://{}", proxy_url);
    }

    let mut cmd = Command::new(&args.cmd[0]);
    if args.cmd.len() > 1 {
        cmd.args(&args.cmd[1..]);
    }
    cmd.env("HTTP_PROXY", &proxy_url)
        .env("HTTPS_PROXY", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .env("NO_PROXY", &args.noproxy)
        .env("http_proxy", &proxy_url)
        .env("https_proxy", &proxy_url)
        .env("all_proxy", &proxy_url)
        .env("no_proxy", &args.noproxy);

    match cmd.status() {
        Ok(status) => {
            legacy_exit(status.code().unwrap_or(1));
        }
        Err(e) => {
            legacy_error(1, format!("px failed: {e}"), &["Hint: Check the command path and arguments."]);
        }
    }
}

pub(crate) fn cmd_proxy(args: ProxyCmd) {
    match args.cmd {
        ProxySubCommand::Set(a) => {
            let only = match parse_proxy_only(a.only.as_deref()) {
                Ok(v) => v,
                Err(e) => {
                    legacy_error(2, e, &["Fix: Use cargo|git|npm|msys2|all."]);
                }
            };
            set_proxy(&a.url, &a.noproxy, a.msys2.as_deref(), only.as_ref());
        }
        ProxySubCommand::Del(a) => {
            let only = match parse_proxy_only(a.only.as_deref()) {
                Ok(v) => v,
                Err(e) => {
                    legacy_error(2, e, &["Fix: Use cargo|git|npm|msys2|all."]);
                }
            };
            del_proxy(a.msys2.as_deref(), only.as_ref());
        }
        ProxySubCommand::Get(_a) => {
            if has_cmd("git") {
                if let Ok(o) = Command::new("git")
                    .args(["config", "--global", "--get", "http.proxy"])
                    .output()
                {
                    out_println!("{}", String::from_utf8_lossy(&o.stdout).trim());
                }
            } else {
                emit_warning(
                    "git not found; skipping proxy get.",
                    &["Hint: Install Git to enable `proxy get`."],
                );
            }
        }
        ProxySubCommand::Detect(_a) => cmd_proxy_detect(),
        ProxySubCommand::Test(a) => {
            let targets = parse_proxy_targets(a.targets.as_deref());
            let timeout = Duration::from_secs(a.timeout.max(1));
            let jobs = a.jobs.max(1);
            for (label, result) in run_proxy_tests_with(&a.url, targets, timeout, jobs) {
                match result {
                    Ok(ms) => out_println!("{}\t{}\tok", label, ms),
                    Err(e) => out_println!("{}\t-1\t{}", label, e),
                }
            }
        }
    }
}
