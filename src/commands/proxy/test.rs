use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::thread;
use std::time::{Duration, Instant};

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
pub(crate) struct ProxyTarget {
    pub(crate) label: String,
    pub(crate) target: Option<String>,
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

pub(crate) fn parse_proxy_targets(raw: Option<&str>) -> Vec<ProxyTarget> {
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

pub(crate) fn run_proxy_tests_with(
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
            let res = h
                .join()
                .unwrap_or(("unknown".into(), Err("thread panic".into())));
            results[i] = Some(res);
        }
        idx = end;
    }

    results
        .into_iter()
        .map(|r| r.unwrap_or(("unknown".into(), Err("thread panic".into()))))
        .collect()
}

pub(crate) fn run_proxy_tests(proxy_url: &str) -> Vec<(String, Result<u64, String>)> {
    run_proxy_tests_with(
        proxy_url,
        default_proxy_targets(),
        Duration::from_secs(5),
        3,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proxy_addr_parses_http_url() {
        let addr = parse_proxy_addr("http://127.0.0.1:7897").expect("addr");
        assert_eq!(addr.port(), 7897);
    }

    #[test]
    fn parse_proxy_targets_empty_returns_default_three_targets() {
        let t = parse_proxy_targets(None);
        let labels: Vec<String> = t.into_iter().map(|x| x.label).collect();
        assert_eq!(labels, vec!["proxy", "8.8.8.8", "1.1.1.1"]);
    }

    #[test]
    fn parse_proxy_targets_adds_default_port_80_when_missing() {
        let t = parse_proxy_targets(Some("example.com"));
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].label, "example.com");
        assert_eq!(t[0].target.as_deref(), Some("example.com:80"));
    }
}
