use super::*;
use super::targets::{default_proxy_targets, ProxyTarget};

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

pub(super) fn run_proxy_tests_with(
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

pub(super) fn run_proxy_tests(proxy_url: &str) -> Vec<(String, Result<u64, String>)> {
    run_proxy_tests_with(
        proxy_url,
        default_proxy_targets(),
        Duration::from_secs(5),
        3,
    )
}

