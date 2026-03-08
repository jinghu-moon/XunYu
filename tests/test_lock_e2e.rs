#![cfg(all(windows, feature = "lock"))]

mod common;

use common::*;
use serde_json::Value;
use std::fs;
use std::thread;
use std::time::Duration;

#[test]
fn lock_who_detects_holder_and_rm_unlock_force_kill_deletes() {
    let env = TestEnv::new();
    let file = env.root.join("locked.txt");
    fs::write(&file, "locked").unwrap();

    let holder = start_lock_holder(&file);
    let holder_pid = holder.pid() as u64;
    assert!(
        wait_until_locked(&file, Duration::from_secs(3)),
        "holder did not acquire lock in time"
    );

    let mut seen = false;
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

        let v: Value = serde_json::from_str(&last_err)
            .unwrap_or_else(|e| panic!("invalid json from lock who: {e}; raw={last_err}"));
        let arr = v
            .as_array()
            .unwrap_or_else(|| panic!("expected array from lock who; raw={last_err}"));
        if arr
            .iter()
            .any(|item| item.get("pid").and_then(Value::as_u64) == Some(holder_pid))
        {
            seen = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    assert!(
        seen,
        "lock who did not report holder pid={holder_pid}; last stderr={last_err}"
    );

    run_ok(
        env.cmd()
            .args(["rm", file.to_str().unwrap(), "--unlock", "--force-kill"]),
    );
    assert!(!file.exists(), "file should be deleted after unlock flow");
}

#[test]
fn rm_unlock_without_force_kill_in_non_interactive_fails() {
    let env = TestEnv::new();
    let file = env.root.join("locked-no-force.txt");
    fs::write(&file, "locked").unwrap();

    let _holder = start_lock_holder(&file);
    assert!(
        wait_until_locked(&file, Duration::from_secs(3)),
        "holder did not acquire lock in time"
    );

    let probe =
        run_raw(
            env.cmd()
                .args(["lock", "who", file.to_str().unwrap(), "--format", "json"]),
        );
    if is_lock_query_env_unavailable(&probe) {
        return;
    }

    let out = run_err(env.cmd().args(["rm", file.to_str().unwrap(), "--unlock"]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(10),
        "expected EXIT_LOCKED_UNAUTHORIZED(10), stderr={err}"
    );
    assert!(
        err.contains("No --force-kill provided and non-interactive"),
        "unexpected stderr: {err}"
    );
    assert!(file.exists(), "file should still exist on aborted unlock");
}

#[test]
fn rm_on_reboot_non_admin_fails() {
    let env = TestEnv::new();
    let file = env.root.join("on_reboot_test.txt");
    fs::write(&file, "data").unwrap();

    let out = run_raw(
        env.cmd()
            .args(["rm", file.to_str().unwrap(), "--on-reboot", "--yes"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    match out.status.code() {
        Some(3) => {
            // Typical non-elevated environment (access denied).
            assert!(
                err.contains("Failed to schedule reboot delete: OS Error"),
                "unexpected stderr: {err}"
            );
        }
        Some(20) => {
            // Elevated environment: scheduling succeeds and returns EXIT_REBOOT_SCHEDULED(20).
            assert!(
                err.contains("Successfully scheduled deletion on next reboot"),
                "unexpected stderr: {err}"
            );
        }
        other => panic!("unexpected exit code {other:?}, stderr={err}"),
    }
}

#[test]
fn mv_unlock_force_kill_moves_locked_file() {
    let env = TestEnv::new();
    let src = env.root.join("mv-locked-src.txt");
    let dst = env.root.join("mv-locked-dst.txt");
    fs::write(&src, "data").unwrap();

    let _holder = start_lock_holder(&src);
    assert!(
        wait_until_locked(&src, Duration::from_secs(3)),
        "holder did not acquire lock in time"
    );

    let out = run_raw(env.cmd().args([
        "mv",
        src.to_str().unwrap(),
        dst.to_str().unwrap(),
        "--unlock",
        "--force-kill",
    ]));
    if !out.status.success() {
        if is_lock_query_env_unavailable(&out) {
            return;
        }
        panic!(
            "mv --unlock failed: {}\nstderr: {}\nstdout: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
    }

    assert!(!src.exists(), "source should be moved");
    assert!(dst.exists(), "destination should exist");
    assert_eq!(fs::read_to_string(&dst).unwrap_or_default(), "data");
}

#[test]
fn lock_who_verbose_emits_debug_privilege_log_line() {
    let env = TestEnv::new();
    let file = env.root.join("verbose-lock-who.txt");
    fs::write(&file, "data").unwrap();

    let out =
        run_raw(
            env.cmd()
                .env("XUN_VERBOSE", "1")
                .args(["lock", "who", file.to_str().unwrap()]),
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

    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("[DEBUG] SeDebugPrivilege:"),
        "expected verbose debug line, got stderr:\n{err}"
    );
}
