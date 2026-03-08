use crate::common::*;
use serde_json::Value;
use std::fs;

#[test]
fn list_tsv_has_fields() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["set", "home", work.to_str().unwrap()]));

    let output = run_ok(env.cmd().args(["list", "--format", "tsv"]));
    let line = String::from_utf8_lossy(&output.stdout);
    let first = line.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split('\t').collect();
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0], "home");
    assert_eq!(parts[1], work.to_str().unwrap());
}

#[test]
fn list_auto_outputs_tsv() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["set", "home", work.to_str().unwrap()]));

    let output = run_ok(env.cmd().args(["list"]));
    let line = String::from_utf8_lossy(&output.stdout);
    let first = line.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split('\t').collect();
    assert_eq!(parts.len(), 5);
}

#[test]
fn list_json_contains_tags() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(
        env.cmd()
            .args(["set", "home", work.to_str().unwrap(), "-t", "dev,cli"]),
    );

    let output = run_ok(env.cmd().args(["list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = v.as_array().unwrap();
    let item = arr.iter().find(|x| x["name"] == "home").unwrap();
    let tags = item["tags"].as_array().unwrap();
    let tag_vals: Vec<String> = tags
        .iter()
        .map(|t| t.as_str().unwrap().to_string())
        .collect();
    assert!(tag_vals.contains(&"dev".to_string()));
    assert!(tag_vals.contains(&"cli".to_string()));
}

#[test]
fn all_and_fuzzy_work() {
    let env = TestEnv::new();
    let a = env.root.join("alpha");
    let b = env.root.join("beta");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    run_ok(
        env.cmd()
            .args(["set", "alpha", a.to_str().unwrap(), "-t", "t1"]),
    );
    run_ok(
        env.cmd()
            .args(["set", "beta", b.to_str().unwrap(), "-t", "t2"]),
    );

    let out_all = run_ok(env.cmd().args(["all", "t1"]));
    let s_all = String::from_utf8_lossy(&out_all.stdout);
    assert!(s_all.lines().any(|l| l.starts_with("alpha\t")));
    assert!(!s_all.lines().any(|l| l.starts_with("beta\t")));

    let out_fuzzy = run_ok(env.cmd().args(["fuzzy", "alp"]));
    let s_fuzzy = String::from_utf8_lossy(&out_fuzzy.stdout);
    assert!(s_fuzzy.lines().next().unwrap_or("").starts_with("alpha\t"));
}

#[test]
fn list_tag_filters_results() {
    let env = TestEnv::new();
    let a = env.root.join("a");
    let b = env.root.join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    run_ok(
        env.cmd()
            .args(["set", "a", a.to_str().unwrap(), "-t", "t1"]),
    );
    run_ok(
        env.cmd()
            .args(["set", "b", b.to_str().unwrap(), "-t", "t2"]),
    );

    let output = run_ok(env.cmd().args(["list", "-t", "t1", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(arr.iter().any(|x| x["name"] == "a"));
    assert!(!arr.iter().any(|x| x["name"] == "b"));
}

#[test]
fn list_sort_visits_descending() {
    let env = TestEnv::new();
    let a = env.root.join("a");
    let b = env.root.join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    run_ok(env.cmd().args(["set", "a", a.to_str().unwrap()]));
    run_ok(env.cmd().args(["set", "b", b.to_str().unwrap()]));

    run_ok(env.cmd().args(["touch", "a"]));
    run_ok(env.cmd().args(["touch", "a"]));
    run_ok(env.cmd().args(["touch", "a"]));
    run_ok(env.cmd().args(["touch", "b"]));

    let output = run_ok(env.cmd().args(["list", "-s", "visits", "--format", "tsv"]));
    let s = String::from_utf8_lossy(&output.stdout);
    let first = s.lines().next().unwrap_or("");
    assert!(first.starts_with("a\t"));
}

#[test]
fn keys_outputs_all_bookmark_names() {
    let env = TestEnv::new();
    let a = env.root.join("a");
    let b = env.root.join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    run_ok(env.cmd().args(["set", "b", b.to_str().unwrap()]));
    run_ok(env.cmd().args(["set", "a", a.to_str().unwrap()]));

    let output = run_ok(env.cmd().args(["keys"]));
    let s = String::from_utf8_lossy(&output.stdout);
    let keys: Vec<&str> = s.lines().collect();
    assert_eq!(keys, vec!["a", "b"]);
}

#[test]
fn invalid_format_fails() {
    let env = TestEnv::new();
    let out = env
        .cmd()
        .args(["list", "--format", "nope"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Invalid format"));
}
