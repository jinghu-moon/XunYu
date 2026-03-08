#![cfg(all(windows, feature = "dashboard"))]

mod common;

use common::*;
use serde_json::Value;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct ServeGuard {
    child: Child,
}

impl Drop for ServeGuard {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn find_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
}

fn wait_port(port: u16, timeout: Duration) {
    let addr = format!("127.0.0.1:{port}");
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpStream::connect(&addr).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("server did not open port {port} in time");
}

fn http_request(port: u16, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();

    let body_str = body.unwrap_or("");
    let req = if body.is_some() {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body_str.as_bytes().len(),
            body_str
        )
    } else {
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
    };
    stream.write_all(req.as_bytes()).expect("write");

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap_or(0);
    let resp = String::from_utf8_lossy(&buf).to_string();

    let mut lines = resp.lines();
    let status_line = lines.next().unwrap_or("");
    let code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    let body = resp.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (code, body)
}

#[test]
fn dashboard_api_config_returns_json() {
    let env = TestEnv::new();
    let port = find_free_port();
    let mut cmd = env.cmd();
    cmd.args(["serve", "-p", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let guard = ServeGuard {
        child: cmd.spawn().expect("spawn serve"),
    };
    wait_port(port, Duration::from_secs(3));

    let (code, body) = http_request(port, "GET", "/api/config", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert!(v.get("proxy").is_some(), "expected proxy config key");
    assert!(v.get("tree").is_some(), "expected tree config key");

    drop(guard);
}

#[test]
fn dashboard_api_config_patch_updates_and_clears_fields() {
    let env = TestEnv::new();
    let port = find_free_port();
    let mut cmd = env.cmd();
    cmd.args(["serve", "-p", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let guard = ServeGuard {
        child: cmd.spawn().expect("spawn serve"),
    };
    wait_port(port, Duration::from_secs(3));

    let patch = r#"{
  "tree": { "defaultDepth": 3, "excludeNames": ["node_modules", "target"] },
  "proxy": { "defaultUrl": "http://127.0.0.1:7890", "noproxy": "localhost" }
}"#;
    let (code, body) = http_request(port, "POST", "/api/config", Some(patch));
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");

    let (code, body) = http_request(port, "GET", "/api/config", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["tree"]["defaultDepth"].as_u64(), Some(3));
    assert_eq!(
        v["tree"]["excludeNames"].as_array().map(|a| a.len()),
        Some(2)
    );
    assert_eq!(
        v["proxy"]["defaultUrl"].as_str(),
        Some("http://127.0.0.1:7890")
    );
    assert_eq!(v["proxy"]["noproxy"].as_str(), Some("localhost"));

    let patch_clear = r#"{
  "tree": { "defaultDepth": null, "excludeNames": [] },
  "proxy": { "defaultUrl": null, "noproxy": null }
}"#;
    let (code, body) = http_request(port, "POST", "/api/config", Some(patch_clear));
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");

    let (code, body) = http_request(port, "GET", "/api/config", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert!(
        v.get("tree")
            .and_then(|tree| tree.get("defaultDepth"))
            .is_none(),
        "expected defaultDepth cleared"
    );
    assert!(
        v.get("tree")
            .and_then(|tree| tree.get("excludeNames"))
            .is_none(),
        "expected excludeNames cleared"
    );
    assert!(
        v.get("proxy")
            .and_then(|proxy| proxy.get("defaultUrl"))
            .is_none(),
        "expected defaultUrl cleared"
    );
    assert!(
        v.get("proxy")
            .and_then(|proxy| proxy.get("noproxy"))
            .is_none(),
        "expected noproxy cleared"
    );

    drop(guard);
}

#[test]
fn dashboard_api_config_put_replaces_config() {
    let env = TestEnv::new();
    let port = find_free_port();
    let mut cmd = env.cmd();
    cmd.args(["serve", "-p", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let guard = ServeGuard {
        child: cmd.spawn().expect("spawn serve"),
    };
    wait_port(port, Duration::from_secs(3));

    let put_body = r#"{
  "tree": { "defaultDepth": 5, "excludeNames": ["src"] },
  "proxy": { "defaultUrl": null, "noproxy": "localhost" }
}"#;
    let (code, body) = http_request(port, "PUT", "/api/config", Some(put_body));
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");

    let (code, body) = http_request(port, "GET", "/api/config", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["tree"]["defaultDepth"].as_u64(), Some(5));
    assert_eq!(
        v["tree"]["excludeNames"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str()),
        Some("src")
    );
    assert!(
        v.get("proxy")
            .and_then(|proxy| proxy.get("defaultUrl"))
            .is_none(),
        "expected defaultUrl omitted when null"
    );
    assert_eq!(v["proxy"]["noproxy"].as_str(), Some("localhost"));

    drop(guard);
}

#[test]
fn dashboard_api_bookmark_rename_roundtrip() {
    let env = TestEnv::new();
    let port = find_free_port();
    let mut cmd = env.cmd();
    cmd.args(["serve", "-p", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let guard = ServeGuard {
        child: cmd.spawn().expect("spawn serve"),
    };
    wait_port(port, Duration::from_secs(3));

    let create_old = r#"{"path":"C:\\tmp","tags":["one"]}"#;
    let (code, body) = http_request(port, "POST", "/api/bookmarks/old", Some(create_old));
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");

    let create_taken = r#"{"path":"C:\\taken","tags":["two"]}"#;
    let (code, body) = http_request(port, "POST", "/api/bookmarks/taken", Some(create_taken));
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");

    let rename_body = r#"{"newName":"new"}"#;
    let (code, body) = http_request(port, "POST", "/api/bookmarks/old/rename", Some(rename_body));
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["name"].as_str(), Some("new"));

    let (code, body) = http_request(port, "GET", "/api/bookmarks", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    let names: Vec<&str> = match v.as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|item| item.get("name").and_then(|n| n.as_str()))
            .collect(),
        None => Vec::new(),
    };
    assert!(names.contains(&"new"), "expected renamed bookmark");
    assert!(!names.contains(&"old"), "expected old name removed");

    let rename_conflict = r#"{"newName":"taken"}"#;
    let (code, _) = http_request(
        port,
        "POST",
        "/api/bookmarks/new/rename",
        Some(rename_conflict),
    );
    assert_eq!(code, 409, "expected conflict");

    let rename_missing = r#"{"newName":"ghost"}"#;
    let (code, _) = http_request(
        port,
        "POST",
        "/api/bookmarks/missing/rename",
        Some(rename_missing),
    );
    assert_eq!(code, 404, "expected not found");

    drop(guard);
}

#[test]
fn dashboard_api_bookmark_export_supports_json_and_tsv() {
    let env = TestEnv::new();
    let port = find_free_port();
    let mut cmd = env.cmd();
    cmd.args(["serve", "-p", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let guard = ServeGuard {
        child: cmd.spawn().expect("spawn serve"),
    };
    wait_port(port, Duration::from_secs(3));

    let create_b = r#"{"path":"C:\\b","tags":["t1"]}"#;
    let (code, body) = http_request(port, "POST", "/api/bookmarks/b", Some(create_b));
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let create_a = r#"{"path":"C:\\a","tags":["t2"]}"#;
    let (code, body) = http_request(port, "POST", "/api/bookmarks/a", Some(create_a));
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");

    let (code, body) = http_request(port, "GET", "/api/bookmarks/export?format=json", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    let arr = v.as_array().expect("json array");
    assert_eq!(
        arr.first()
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str()),
        Some("a")
    );
    assert_eq!(
        arr.get(1)
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str()),
        Some("b")
    );

    let (code, body) = http_request(port, "GET", "/api/bookmarks/export?format=tsv", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 2, "expected at least 2 lines");
    assert!(
        lines[0].starts_with("a\t"),
        "expected first line to start with a"
    );
    assert!(
        lines[1].starts_with("b\t"),
        "expected second line to start with b"
    );

    drop(guard);
}

#[test]
fn dashboard_api_bookmark_import_supports_json_and_tsv() {
    let env = TestEnv::new();
    let port = find_free_port();
    let mut cmd = env.cmd();
    cmd.args(["serve", "-p", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let guard = ServeGuard {
        child: cmd.spawn().expect("spawn serve"),
    };
    wait_port(port, Duration::from_secs(3));

    let payload = r#"[
  { "name": "one", "path": "C:\\one", "tags": ["t1"], "visits": 1, "last_visited": 10 },
  { "name": "two", "path": "C:\\two", "tags": [], "visits": 0, "last_visited": 0 }
]"#;
    let (code, body) = http_request(
        port,
        "POST",
        "/api/bookmarks/import?format=json&mode=merge",
        Some(payload),
    );
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["added"].as_u64(), Some(2));
    assert_eq!(v["updated"].as_u64(), Some(0));
    assert_eq!(v["total"].as_u64(), Some(2));

    let tsv_body = "one\tC:\\new\tfoo,bar\t5\t20\n";
    let (code, body) = http_request(
        port,
        "POST",
        "/api/bookmarks/import?format=tsv&mode=overwrite",
        Some(tsv_body),
    );
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["added"].as_u64(), Some(0));
    assert_eq!(v["updated"].as_u64(), Some(1));
    assert_eq!(v["total"].as_u64(), Some(1));

    let (code, body) = http_request(port, "GET", "/api/bookmarks", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    let item = v.as_array().and_then(|arr| {
        arr.iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("one"))
    });
    let item = item.expect("expected one");
    assert_eq!(item["path"].as_str(), Some("C:\\new"));
    let tags = item["tags"].as_array().expect("tags");
    let mut tag_values: Vec<&str> = tags.iter().filter_map(|v| v.as_str()).collect();
    tag_values.sort_unstable();
    assert_eq!(tag_values, vec!["bar", "foo"]);

    drop(guard);
}

#[test]
#[cfg(feature = "redirect")]
fn dashboard_api_redirect_profiles_roundtrip() {
    let env = TestEnv::new();
    let port = find_free_port();
    let mut cmd = env.cmd();
    cmd.args(["serve", "-p", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let guard = ServeGuard {
        child: cmd.spawn().expect("spawn serve"),
    };
    wait_port(port, Duration::from_secs(3));

    let (code, body) = http_request(port, "GET", "/api/redirect/profiles", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");

    let profile = r#"{
  "rules": [
    { "name": "Imgs", "match": { "ext": ["jpg"], "regex": null, "glob": null, "size": null, "age": null }, "dest": "./Images/{created.year}/{created.month}" }
  ],
  "unmatched": "archive:>=1d:./Others",
  "on_conflict": "trash",
  "recursive": true,
  "max_depth": 2
}"#;
    let (code, _) = http_request(port, "POST", "/api/redirect/profiles/demo", Some(profile));
    assert_eq!(code, 200, "expected ok from upsert");

    let (code, body) = http_request(port, "GET", "/api/redirect/profiles", None);
    assert_eq!(code, 200);
    let v: Value = serde_json::from_str(&body).expect("json");
    assert!(v["profiles"]["demo"].is_object(), "expected demo profile");
    assert_eq!(
        v["profiles"]["demo"]["on_conflict"].as_str().unwrap_or(""),
        "trash"
    );
    assert_eq!(
        v["profiles"]["demo"]["unmatched"].as_str().unwrap_or(""),
        "archive:>=1d:./Others"
    );
    assert_eq!(v["profiles"]["demo"]["recursive"].as_bool(), Some(true));

    drop(guard);
}

#[test]
fn dashboard_api_ports_supports_kill_pid_and_details() {
    let env = TestEnv::new();
    let port = find_free_port();
    let mut cmd = env.cmd();
    cmd.args(["serve", "-p", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let guard = ServeGuard {
        child: cmd.spawn().expect("spawn serve"),
    };
    wait_port(port, Duration::from_secs(3));

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let listen_port = listener.local_addr().expect("addr").port();

    let (code, body) = http_request(port, "GET", "/api/ports", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    let tcp = v.get("tcp").and_then(|v| v.as_array()).expect("tcp array");
    let pid = std::process::id() as u64;
    let entry = tcp.iter().find(|p| {
        p.get("port").and_then(|v| v.as_u64()) == Some(listen_port as u64)
            && p.get("pid").and_then(|v| v.as_u64()) == Some(pid)
    });
    assert!(entry.is_some(), "expected listener entry");
    let entry = entry.unwrap();
    assert!(entry.get("cmdline").and_then(|v| v.as_str()).is_some());
    assert!(entry.get("cwd").and_then(|v| v.as_str()).is_some());
    drop(listener);

    let mut child = Command::new("cmd")
        .args(["/c", "ping", "-n", "60", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn child");
    let child_pid = child.id();

    let (code, body) = http_request(
        port,
        "POST",
        &format!("/api/ports/kill-pid/{child_pid}"),
        None,
    );
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");

    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if child.try_wait().ok().flatten().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }

    drop(guard);
}

#[test]
#[cfg(feature = "redirect")]
fn dashboard_api_redirect_dry_run_returns_results() {
    let env = TestEnv::new();
    let port = find_free_port();
    let mut cmd = env.cmd();
    cmd.args(["serve", "-p", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let guard = ServeGuard {
        child: cmd.spawn().expect("spawn serve"),
    };
    wait_port(port, Duration::from_secs(3));

    let source_dir = env.root.join("redirect-src");
    std::fs::create_dir_all(&source_dir).unwrap();
    std::fs::write(source_dir.join("a.txt"), "hello").unwrap();

    let source = source_dir.to_string_lossy().replace("\\", "\\\\");
    let profile = r#"{
  "rules": [
    { "name": "Docs", "match": { "ext": ["txt"], "regex": null, "glob": null, "size": null, "age": null }, "dest": "./Out" }
  ],
  "unmatched": "skip",
  "on_conflict": "rename_new",
  "recursive": false,
  "max_depth": 1
}"#;
    let body = format!(
        "{{\"source\":\"{}\",\"profile\":{},\"copy\":true}}",
        source, profile
    );

    let (code, body) = http_request(port, "POST", "/api/redirect/dry-run", Some(&body));
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    let results = v
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results");
    assert!(!results.is_empty(), "expected dry-run results");
    assert!(
        results
            .iter()
            .any(|r| r.get("result").and_then(|v| v.as_str()) == Some("dry_run"))
    );
    let stats = v.get("stats").expect("stats");
    assert!(stats.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);

    drop(guard);
}

#[test]
fn dashboard_api_audit_supports_range_cursor_and_csv() {
    let env = TestEnv::new();
    let mut f = File::create(env.audit_path()).expect("create audit");
    for (ts, action) in [(100u64, "a1"), (200, "a2"), (300, "a3"), (400, "a4")] {
        let entry = serde_json::json!({
            "timestamp": ts,
            "action": action,
            "target": "target",
            "user": "user",
            "params": "params",
            "result": "success",
            "reason": ""
        });
        writeln!(f, "{}", entry).unwrap();
    }

    let port = find_free_port();
    let mut cmd = env.cmd();
    cmd.args(["serve", "-p", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let guard = ServeGuard {
        child: cmd.spawn().expect("spawn serve"),
    };
    wait_port(port, Duration::from_secs(3));

    let (code, body) = http_request(port, "GET", "/api/audit?from=150&to=350&limit=1", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    let entries = v
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].get("timestamp").and_then(|v| v.as_u64()),
        Some(300)
    );
    assert_eq!(v.get("next_cursor").and_then(|v| v.as_str()), Some("1"));

    let (code, body) = http_request(
        port,
        "GET",
        "/api/audit?from=150&to=350&limit=1&cursor=1",
        None,
    );
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    let entries = v
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].get("timestamp").and_then(|v| v.as_u64()),
        Some(200)
    );

    let (code, body) = http_request(port, "GET", "/api/audit?from=150&to=350&format=csv", None);
    assert_eq!(code, 200, "unexpected status: {code}, body={body}");
    let mut lines = body.lines();
    let header = lines.next().unwrap_or("");
    assert_eq!(header, "timestamp,action,target,user,params,result,reason");
    let first = lines.next().unwrap_or("");
    assert!(first.starts_with("300,"), "expected newest entry first");

    drop(guard);
}
