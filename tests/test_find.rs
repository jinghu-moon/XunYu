#![cfg(windows)]

mod common;

use common::*;
use serde_json::Value;
use std::fs;

fn tsv_paths(out: &std::process::Output) -> Vec<String> {
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines()
        .filter_map(|line| line.split('\t').next())
        .map(|v| v.to_string())
        .collect()
}

fn tsv_entries(out: &std::process::Output) -> Vec<(String, bool)> {
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let path = parts.next()?.to_string();
            let is_dir = parts.next().map(|v| v == "1").unwrap_or(false);
            Some((path, is_dir))
        })
        .collect()
}

#[test]
fn find_count_with_include_exclude() {
    let env = TestEnv::new();
    let root = env.root.join("work");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("keep.txt"), b"ok").unwrap();
    fs::write(root.join("skip.txt"), b"no").unwrap();
    fs::write(root.join("note.log"), b"log").unwrap();

    let out = run_ok(env.cmd().args([
        "find",
        root.to_str().unwrap(),
        "--include",
        "*.txt",
        "--exclude",
        "skip.txt",
        "--count",
    ]));
    let count = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert_eq!(count, "1");
}

#[test]
fn find_tsv_paths_use_forward_slash() {
    let env = TestEnv::new();
    let root = env.root.join("work");
    let dir = root.join("dir");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("a.txt"), b"ok").unwrap();

    let out = run_ok(
        env.cmd()
            .args(["find", root.to_str().unwrap(), "--format", "tsv"]),
    );
    let paths = tsv_paths(&out);
    assert!(!paths.is_empty());
    for p in paths {
        assert!(p.contains('/'));
        assert!(!p.contains('\\'));
    }
}

#[test]
fn find_json_rule_idx_is_1_based() {
    let env = TestEnv::new();
    let root = env.root.join("work");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), b"ok").unwrap();

    let out = run_ok(env.cmd().args([
        "find",
        root.to_str().unwrap(),
        "--include",
        "a.txt",
        "--format",
        "json",
    ]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let results = v["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    let item = &results[0];
    assert_eq!(item["rule_idx"].as_i64().unwrap(), 1);
    let path = item["path"].as_str().unwrap();
    assert!(path.contains('/'));
    assert!(!path.contains('\\'));
}

#[test]
fn find_regex_full_match_and_path() {
    let env = TestEnv::new();
    let root = env.root.join("work");
    let dir = root.join("dir");
    fs::create_dir_all(&dir).unwrap();
    fs::write(root.join("a.txt"), b"ok").unwrap();
    fs::write(root.join("ab.txt"), b"ok").unwrap();
    fs::write(dir.join("b.txt"), b"ok").unwrap();

    let out = run_ok(env.cmd().args([
        "find",
        root.to_str().unwrap(),
        "--regex-include",
        r"a\.txt",
        "--format",
        "tsv",
    ]));
    let paths = tsv_paths(&out);
    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("/a.txt"));

    let out = run_ok(env.cmd().args([
        "find",
        root.to_str().unwrap(),
        "--regex-include",
        r"dir/.+\.txt",
        "--format",
        "tsv",
    ]));
    let paths = tsv_paths(&out);
    assert_eq!(paths.len(), 1);
    assert!(paths[0].contains("/dir/"));
}

#[test]
fn find_anchored_glob_and_empty_dirs() {
    let env = TestEnv::new();
    let root = env.root.join("work");
    let dir = root.join("dir");
    let empty_dir = root.join("empty_dir");
    let non_empty_dir = root.join("non_empty_dir");
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&empty_dir).unwrap();
    fs::create_dir_all(&non_empty_dir).unwrap();
    fs::write(dir.join("b.txt"), b"ok").unwrap();
    fs::write(non_empty_dir.join("x.txt"), b"ok").unwrap();

    let out = run_ok(env.cmd().args([
        "find",
        root.to_str().unwrap(),
        "--include",
        "dir/b.txt",
        "--format",
        "tsv",
    ]));
    let paths = tsv_paths(&out);
    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("/dir/b.txt"));

    let out = run_ok(env.cmd().args([
        "find",
        root.to_str().unwrap(),
        "--include",
        "empty_dir/",
        "--include",
        "non_empty_dir/",
        "--empty-dirs",
        "--format",
        "tsv",
    ]));
    let entries = tsv_entries(&out);
    let dir_paths: Vec<String> = entries
        .into_iter()
        .filter(|(_, is_dir)| *is_dir)
        .map(|(p, _)| p)
        .collect();
    assert_eq!(dir_paths.len(), 1);
    assert!(dir_paths[0].ends_with("/empty_dir"));
}
