#![cfg(all(windows, feature = "lock"))]

#[path = "../support/mod.rs"]
mod common;

use common::*;
use serde_json::Value;
use std::fs;
use std::thread;
use std::time::Duration;

#[test]
fn rm_dry_run_keeps_target_intact() {
    let env = TestEnv::new();
    let file = env.root.join("dry-rm.txt");
    fs::write(&file, "data").unwrap();

    run_ok(env.cmd().args(["rm", file.to_str().unwrap(), "--dry-run"]));
    assert!(file.exists(), "rm --dry-run should not delete file");
}

#[test]
fn mv_dry_run_does_not_move() {
    let env = TestEnv::new();
    let src = env.root.join("dry-mv-src.txt");
    let dst = env.root.join("dry-mv-dst.txt");
    fs::write(&src, "data").unwrap();

    run_ok(env.cmd().args([
        "mv",
        src.to_str().unwrap(),
        dst.to_str().unwrap(),
        "--dry-run",
    ]));
    assert!(src.exists(), "mv --dry-run should keep source");
    assert!(!dst.exists(), "mv --dry-run should not create destination");
}

#[test]
fn ren_dry_run_does_not_rename() {
    let env = TestEnv::new();
    let src = env.root.join("dry-ren-src.txt");
    let dst = env.root.join("dry-ren-dst.txt");
    fs::write(&src, "data").unwrap();

    run_ok(env.cmd().args([
        "ren",
        src.to_str().unwrap(),
        dst.to_str().unwrap(),
        "--dry-run",
    ]));
    assert!(src.exists(), "ren --dry-run should keep source");
    assert!(!dst.exists(), "ren --dry-run should not create destination");
}

#[test]
fn lock_who_json_has_stable_fields() {
    let env = TestEnv::new();
    let file = env.root.join("format-locked.txt");
    fs::write(&file, "data").unwrap();
    let _holder = start_lock_holder(&file);
    assert!(
        wait_until_locked(&file, Duration::from_secs(3)),
        "holder did not acquire lock in time"
    );

    let mut parsed = None;
    let mut last_err = String::new();
    for _ in 0..20 {
        let out =
            run_raw(
                env.cmd()
                    .args(["lock", "who", file.to_str().unwrap(), "--format", "json"]),
            );
        if !out.status.success() {
            if is_lock_query_env_unavailable(&out) {
                return;
            }
            panic!(
                "lock who failed: {}\nstderr: {}\nstdout: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr),
                String::from_utf8_lossy(&out.stdout)
            );
        }

        last_err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if last_err.is_empty() {
            thread::sleep(Duration::from_millis(100));
            continue;
        }
        let value: Value = serde_json::from_str(&last_err).unwrap();
        if let Some(first) = value.as_array().and_then(|arr| arr.first()) {
            parsed = Some(first.clone());
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    let first = parsed.unwrap_or_else(|| panic!("lock who json is empty, raw={last_err}"));
    assert!(first.get("pid").and_then(Value::as_u64).is_some());
    assert!(first.get("name").and_then(Value::as_str).is_some());
    assert!(first.get("type").is_some());
}

#[test]
fn lock_who_missing_path_exits_2() {
    let env = TestEnv::new();
    let missing = env.root.join("missing.txt");

    let out = run_err(env.cmd().args(["lock", "who", missing.to_str().unwrap()]));
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("File or directory not found"));
}

#[test]
fn lock_who_tsv_outputs_three_columns() {
    let env = TestEnv::new();
    let file = env.root.join("format-locked-tsv.txt");
    fs::write(&file, "data").unwrap();
    let _holder = start_lock_holder(&file);
    assert!(
        wait_until_locked(&file, Duration::from_secs(3)),
        "holder did not acquire lock in time"
    );

    let out = run_raw(
        env.cmd()
            .args(["lock", "who", file.to_str().unwrap(), "--format", "tsv"]),
    );
    if !out.status.success() {
        if is_lock_query_env_unavailable(&out) {
            return;
        }
        panic!(
            "lock who --format tsv failed: {}\nstderr: {}\nstdout: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
    }

    let stderr = String::from_utf8_lossy(&out.stderr);
    let first = stderr.lines().next().unwrap_or("");
    let cols: Vec<&str> = first.split('\t').collect();
    assert_eq!(cols.len(), 3, "unexpected tsv: {first}");
    assert!(
        cols[0].parse::<u64>().is_ok(),
        "pid should be numeric: {first}"
    );
    assert!(!cols[1].is_empty(), "name should not be empty: {first}");
    assert!(
        cols[2].parse::<u64>().is_ok(),
        "type should be numeric: {first}"
    );
}

#[test]
fn lock_who_no_lockers_outputs_message_in_auto_format() {
    let env = TestEnv::new();
    let file = env.root.join("format-unlocked.txt");
    fs::write(&file, "data").unwrap();

    let out = run_raw(env.cmd().args(["lock", "who", file.to_str().unwrap()]));
    if !out.status.success() {
        if is_lock_query_env_unavailable(&out) {
            return;
        }
        panic!(
            "lock who failed: {}\nstderr: {}\nstdout: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
    }

    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("No locking processes found."),
        "unexpected stderr: {err}"
    );
}

#[cfg(feature = "protect")]
#[test]
fn protect_status_json_is_array_and_fields_exist() {
    let env = TestEnv::new();
    let file = env.root.join("format-protect.txt");
    fs::write(&file, "data").unwrap();

    run_ok(
        env.cmd()
            .args(["protect", "set", file.to_str().unwrap(), "--deny", "delete"]),
    );

    let out = run_ok(env.cmd().args([
        "protect",
        "status",
        file.to_str().unwrap(),
        "--format",
        "json",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: Value = serde_json::from_str(stderr.trim()).unwrap();
    let arr = v.as_array().expect("protect status json should be array");
    assert!(!arr.is_empty(), "protect status array should not be empty");
    let first = &arr[0];
    assert!(first.get("path").and_then(Value::as_str).is_some());
    assert!(first.get("deny").is_some());
    assert!(first.get("require").is_some());
}
