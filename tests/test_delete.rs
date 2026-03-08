#![cfg(windows)]

mod common;

use common::*;
use serde_json::Value;
use std::fs;
use std::time::Duration;

#[test]
fn del_without_bm_keeps_bookmark() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["set", "home", work.to_str().unwrap()]));
    run_ok(env.cmd().args(["del", "home"]));

    let output = run_ok(env.cmd().args(["list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v.as_array().unwrap().iter().any(|x| x["name"] == "home"));
}

#[test]
fn delete_default_skips_non_reserved_file() {
    let env = TestEnv::new();
    let file = env.root.join("normal.txt");
    fs::write(&file, "data").unwrap();

    let out = run_ok(env.cmd().args(["delete", file.to_str().unwrap()]));
    assert!(file.exists());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("No matching files found.") || err.contains("Skipped non-target file"),
        "unexpected stderr: {err}"
    );
}

#[test]
fn del_any_deletes_file() {
    let env = TestEnv::new();
    let file = env.root.join("any.txt");
    fs::write(&file, "data").unwrap();

    let out = run_ok(
        env.cmd()
            .args(["del", "--any", "--format", "json", file.to_str().unwrap()]),
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["success"].as_bool(), Some(true));
    assert!(!file.exists());
}

#[test]
fn delete_dry_run_keeps_file() {
    let env = TestEnv::new();
    let file = env.root.join("dry_run.txt");
    fs::write(&file, "data").unwrap();

    let out = run_ok(env.cmd().args([
        "delete",
        "--any",
        "--dry-run",
        "--format",
        "json",
        file.to_str().unwrap(),
    ]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let result = v
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("result"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(result, "WhatIf");
    assert!(file.exists());
}

#[test]
fn delete_what_if_keeps_file() {
    let env = TestEnv::new();
    let file = env.root.join("what_if.txt");
    fs::write(&file, "data").unwrap();

    let out = run_ok(env.cmd().args([
        "delete",
        "--any",
        "--what-if",
        "--format",
        "json",
        file.to_str().unwrap(),
    ]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let result = v
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("result"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(result, "WhatIf");
    assert!(file.exists());
}

#[test]
fn delete_on_reboot_fallback_for_locked_file() {
    let env = TestEnv::new();
    let file = env.root.join("locked.txt");
    fs::write(&file, "data").unwrap();

    let _holder = start_lock_holder(&file);
    assert!(
        wait_until_locked(&file, Duration::from_secs(3)),
        "holder did not acquire lock in time"
    );

    let out = run_ok(env.cmd().args([
        "delete",
        "--any",
        "--on-reboot",
        "--format",
        "json",
        file.to_str().unwrap(),
    ]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let result = v
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("result"))
        .and_then(Value::as_str)
        .unwrap_or("");

    assert!(
        result == "Scheduled (delete on reboot)" || result.starts_with("Failed:"),
        "unexpected result: {result}"
    );
    if result.starts_with("Failed:") {
        assert_ne!(
            result, "Failed: File is in use",
            "expected on-reboot fallback to change error"
        );
    }
    assert!(file.exists());
}
