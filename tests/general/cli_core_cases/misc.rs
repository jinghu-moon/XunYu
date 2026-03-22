use crate::common::*;
use serde_json::{Value, json};
use std::fs;

#[test]
fn init_powershell_contains_wrappers() {
    let env = TestEnv::new();
    let output = run_ok(env.cmd().args(["init", "powershell"]));
    let s = String::from_utf8_lossy(&output.stdout);
    assert!(s.contains("function xun"));
    assert!(s.contains("_xun_apply_magic"));
    assert!(s.contains("__ENV_SET__"));
    for name in [
        "function list",
        "function gc",
        "function delete",
        "function rename",
        "function tag",
        "function recent",
        "function stats",
        "function dedup",
        "function bak",
        "function xtree",
    ] {
        assert!(s.contains(name), "missing wrapper: {}", name);
    }
    let has_tree = s
        .lines()
        .any(|l| l.trim_start().starts_with("function tree "));
    assert!(!has_tree);
}

#[test]
fn init_bash_uses_xtree_only() {
    let env = TestEnv::new();
    let output = run_ok(env.cmd().args(["init", "bash"]));
    let s = String::from_utf8_lossy(&output.stdout);
    let has_xtree = s.lines().any(|l| l.trim_start().starts_with("xtree() {"));
    let has_tree = s.lines().any(|l| l.trim_start().starts_with("tree() {"));
    assert!(has_xtree);
    assert!(!has_tree);
}

#[test]
fn recent_outputs_tsv() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["set", "home", work.to_str().unwrap()]));
    run_ok(env.cmd().args(["touch", "home"]));

    let output = run_ok(env.cmd().args(["recent", "-n", "1"]));
    let line = String::from_utf8_lossy(&output.stdout);
    let first = line.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split('\t').collect();
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[0], "home");
}

#[test]
fn stats_outputs_tsv() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["set", "home", work.to_str().unwrap()]));
    let output = run_ok(env.cmd().args(["stats"]));
    let s = String::from_utf8_lossy(&output.stdout);
    let mut found = false;
    for line in s.lines() {
        if line == "bookmarks\t1" {
            found = true;
            break;
        }
    }
    assert!(found);
}

#[test]
fn del_deletes_existing_bookmark() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["set", "home", work.to_str().unwrap()]));
    run_ok(env.cmd().args(["del", "-bm", "home"]));

    let output = run_ok(env.cmd().args(["list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!v.as_array().unwrap().iter().any(|x| x["name"] == "home"));
}

#[test]
fn del_missing_reports_not_found_and_does_not_error() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args(["del", "-bm", "nope"]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("not found"));
}

#[test]
fn check_outputs_missing_stale_and_duplicate_in_json() {
    let env = TestEnv::new();
    let dup_dir = env.root.join("dup");
    let stale_dir = env.root.join("stale");
    fs::create_dir_all(&dup_dir).unwrap();
    fs::create_dir_all(&stale_dir).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let stale_ts = now.saturating_sub(100 * 24 * 3600);

    let db = json!({
        "missing": { "path": env.root.join("missing").to_str().unwrap(), "tags": [], "visit_count": 0, "last_visited": 0 },
        "stale": { "path": stale_dir.to_str().unwrap(), "tags": [], "visit_count": 1, "last_visited": stale_ts },
        "dup1": { "path": dup_dir.to_str().unwrap(), "tags": [], "visit_count": 1, "last_visited": now },
        "dup2": { "path": dup_dir.to_str().unwrap(), "tags": [], "visit_count": 2, "last_visited": now }
    });
    fs::write(
        env.root.join(".xun.json"),
        serde_json::to_string(&db).unwrap(),
    )
    .unwrap();

    let out = run_ok(
        env.cmd()
            .args(["check", "--format", "json", "--days", "30"]),
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(arr.iter().any(|x| x["kind"] == "missing"));
    assert!(arr.iter().any(|x| x["kind"] == "stale"));
    assert!(arr.iter().any(|x| x["kind"] == "duplicate"));
}

#[test]
fn config_set_and_get_roundtrip() {
    let env = TestEnv::new();
    run_ok(
        env.cmd()
            .args(["config", "set", "proxy.defaultUrl", "http://127.0.0.1:7890"]),
    );
    let out = run_ok(env.cmd().args(["config", "get", "proxy.defaultUrl"]));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("http://127.0.0.1:7890"));
}
