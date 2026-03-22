#![cfg(all(windows, feature = "lock", feature = "protect"))]

#[path = "../support/mod.rs"]
mod common;

use common::*;
use serde_json::Value;
use std::fs;

#[test]
fn protect_blocks_then_force_reason_allows_and_audit_persists() {
    let env = TestEnv::new();
    let file = env.root.join("protected.txt");
    fs::write(&file, "data").unwrap();

    run_ok(
        env.cmd()
            .args(["protect", "set", file.to_str().unwrap(), "--deny", "delete"]),
    );

    let status_out = run_ok(env.cmd().args([
        "protect",
        "status",
        file.to_str().unwrap(),
        "--format",
        "json",
    ]));
    let status_stderr = String::from_utf8_lossy(&status_out.stderr);
    let status_json: Value = serde_json::from_str(status_stderr.trim()).unwrap_or_else(|e| {
        panic!("protect status should output json to stderr: {e}; raw={status_stderr}")
    });
    let status_arr = status_json
        .as_array()
        .expect("protect status should return array");
    assert!(
        status_arr
            .iter()
            .any(|x| x.get("path").and_then(Value::as_str) == Some(file.to_str().unwrap())),
        "protect status should contain target path"
    );

    let blocked = run_err(env.cmd().args(["rm", file.to_str().unwrap()]));
    let blocked_err = String::from_utf8_lossy(&blocked.stderr);
    assert_eq!(
        blocked.status.code(),
        Some(3),
        "expected EXIT_ACCESS_DENIED(3), stderr={blocked_err}"
    );
    assert!(
        blocked_err.contains("Protection check failed"),
        "unexpected stderr: {blocked_err}"
    );
    assert!(
        file.exists(),
        "file should remain when protection blocks delete"
    );

    run_ok(env.cmd().args([
        "rm",
        file.to_str().unwrap(),
        "--force",
        "--reason",
        "e2e-force",
        "--yes",
    ]));
    assert!(
        !file.exists(),
        "file should be deleted after force+reason bypass"
    );

    let audit_path = env.audit_path();
    assert!(
        audit_path.exists(),
        "audit file should exist: {:?}",
        audit_path
    );
    let audit_content = fs::read_to_string(&audit_path).unwrap_or_else(|e| {
        panic!("failed to read audit log {:?}: {e}", audit_path);
    });
    let entries: Vec<Value> = audit_content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect();

    assert!(
        entries.iter().any(|e| {
            e.get("action").and_then(Value::as_str) == Some("protect_set")
                && e.get("target").and_then(Value::as_str) == Some(file.to_str().unwrap())
        }),
        "protect_set audit entry missing"
    );
    assert!(
        entries.iter().any(|e| {
            e.get("action").and_then(Value::as_str) == Some("protected_delete")
                && e.get("target").and_then(Value::as_str) == Some(file.to_str().unwrap())
                && e.get("result").and_then(Value::as_str) == Some("success")
                && e.get("reason").and_then(Value::as_str) == Some("e2e-force")
        }),
        "protected_delete audit entry missing"
    );
}
