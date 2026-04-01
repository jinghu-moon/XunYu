use crate::common::*;
use serde_json::Value;
use std::fs;

#[test]
fn gc_purge_removes_missing() {
    let env = TestEnv::new();
    let missing = env.root.join("missing");

    run_ok(env.cmd().args(["bookmark", "set", "dead", missing.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "gc", "--purge"]));

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let found = v.as_array().unwrap().iter().any(|x| x["name"] == "dead");
    assert!(!found);
}

#[test]
fn gc_without_purge_only_reports() {
    let env = TestEnv::new();
    let missing = env.root.join("missing");

    run_ok(env.cmd().args(["bookmark", "set", "dead", missing.to_str().unwrap()]));
    let out = run_ok(env.cmd().args(["bookmark", "gc", "--format", "json"]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v.as_array().unwrap().iter().any(|x| x["name"] == "dead"));

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v.as_array().unwrap().iter().any(|x| x["name"] == "dead"));
}

#[test]
fn dedup_reports_duplicates() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "a", work.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "set", "b", work.to_str().unwrap()]));

    let output = run_ok(env.cmd().args(["bookmark", "dedup"]));
    let s = String::from_utf8_lossy(&output.stdout);
    assert!(s.lines().any(|l| l.contains("\ta\t")));
    assert!(s.lines().any(|l| l.contains("\tb\t")));

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(arr.iter().any(|x| x["name"] == "a"));
    assert!(arr.iter().any(|x| x["name"] == "b"));
}

#[test]
fn explicit_name_is_case_insensitive_unique() {
    let env = TestEnv::new();
    let a = env.root.join("a");
    let b = env.root.join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "Home", a.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "set", "home", b.to_str().unwrap()]));

    let out = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "home");
    assert_eq!(
        arr[0]["path"].as_str().unwrap(),
        b.to_string_lossy().replace('\\', "/")
    );
}
