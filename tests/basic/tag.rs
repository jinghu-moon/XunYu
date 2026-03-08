use crate::common::*;
use serde_json::Value;
use std::fs;

#[test]
fn tag_add_remove_rename_list() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["set", "home", work.to_str().unwrap()]));
    run_ok(env.cmd().args(["tag", "add", "home", "dev,cli"]));

    let output = run_ok(env.cmd().args(["list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "home")
        .unwrap();
    let tags = item["tags"].as_array().unwrap();
    let tag_vals: Vec<String> = tags
        .iter()
        .map(|t| t.as_str().unwrap().to_string())
        .collect();
    assert!(tag_vals.contains(&"dev".to_string()));
    assert!(tag_vals.contains(&"cli".to_string()));

    run_ok(env.cmd().args(["tag", "remove", "home", "cli"]));
    let output = run_ok(env.cmd().args(["list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "home")
        .unwrap();
    let tags = item["tags"].as_array().unwrap();
    let tag_vals: Vec<String> = tags
        .iter()
        .map(|t| t.as_str().unwrap().to_string())
        .collect();
    assert!(tag_vals.contains(&"dev".to_string()));
    assert!(!tag_vals.contains(&"cli".to_string()));

    run_ok(env.cmd().args(["tag", "rename", "dev", "prod"]));
    let output = run_ok(env.cmd().args(["list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "home")
        .unwrap();
    let tags = item["tags"].as_array().unwrap();
    let tag_vals: Vec<String> = tags
        .iter()
        .map(|t| t.as_str().unwrap().to_string())
        .collect();
    assert!(tag_vals.contains(&"prod".to_string()));
    assert!(!tag_vals.contains(&"dev".to_string()));

    let list_out = run_ok(env.cmd().args(["tag", "list"]));
    let s = String::from_utf8_lossy(&list_out.stdout);
    let first = s.lines().next().unwrap_or("");
    assert!(first.starts_with("prod\t1"));
}

#[test]
fn tag_add_does_not_duplicate_existing_tags() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(
        env.cmd()
            .args(["set", "home", work.to_str().unwrap(), "-t", "work"]),
    );
    let out = run_ok(env.cmd().args(["tag", "add", "home", "work,WORK"]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("No new tags added."));

    let output = run_ok(env.cmd().args(["list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "home")
        .unwrap();
    let tags = item["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].as_str(), Some("work"));
}
