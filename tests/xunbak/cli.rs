#[path = "../support/mod.rs"]
mod common;

use common::{TestEnv, run_ok};
use serde_json::Value;
use std::fs;

#[test]
fn cli_backup_container_creates_xunbak_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    assert!(root.join("backup.xunbak").exists());
}

#[test]
fn cli_backup_container_second_run_updates_incrementally() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));
    let first_size = fs::metadata(root.join("backup.xunbak")).unwrap().len();

    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(root.join("b.txt"), "bbb").unwrap();
    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t2",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Updated xunbak:"));
    let second_size = fs::metadata(root.join("backup.xunbak")).unwrap().len();
    assert!(second_size > first_size);
}

#[test]
fn cli_restore_container_restores_to_target_dir() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let out_dir = root.join("restore");
    run_ok(env.cmd().args([
        "restore",
        root.join("backup.xunbak").to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(fs::read_to_string(out_dir.join("a.txt")).unwrap(), "aaa");
}

#[test]
fn cli_restore_container_restores_single_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("nested").join("b.txt"), "bbb").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let out_dir = root.join("restore");
    run_ok(env.cmd().args([
        "restore",
        root.join("backup.xunbak").to_str().unwrap(),
        "--file",
        "nested/b.txt",
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert!(!out_dir.join("a.txt").exists());
    assert_eq!(
        fs::read_to_string(out_dir.join("nested").join("b.txt")).unwrap(),
        "bbb"
    );
}

#[test]
fn cli_restore_container_restores_glob_selection() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.rs"), "aaa").unwrap();
    fs::write(root.join("nested").join("b.rs"), "bbb").unwrap();
    fs::write(root.join("c.txt"), "ccc").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let out_dir = root.join("restore");
    run_ok(env.cmd().args([
        "restore",
        root.join("backup.xunbak").to_str().unwrap(),
        "--glob",
        "nested/**/*.rs",
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert!(!out_dir.join("a.rs").exists());
    assert_eq!(
        fs::read_to_string(out_dir.join("nested").join("b.rs")).unwrap(),
        "bbb"
    );
    assert!(!out_dir.join("c.txt").exists());
}

#[test]
fn cli_verify_container_outputs_json() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let out = run_ok(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value.get("passed").and_then(Value::as_bool), Some(true));
    assert_eq!(value.get("level").and_then(Value::as_str), Some("quick"));
}

#[test]
fn cli_verify_full_and_paranoid_levels_succeed() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let full = run_ok(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--level",
        "full",
        "--json",
    ]));
    let full_value: Value = serde_json::from_slice(&full.stdout).unwrap();
    assert_eq!(
        full_value.get("passed").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        full_value.get("level").and_then(Value::as_str),
        Some("full")
    );

    let paranoid = run_ok(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--level",
        "paranoid",
        "--json",
    ]));
    let paranoid_value: Value = serde_json::from_slice(&paranoid.stdout).unwrap();
    assert_eq!(
        paranoid_value.get("passed").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        paranoid_value.get("level").and_then(Value::as_str),
        Some("paranoid")
    );
}

#[test]
fn cli_backup_container_prints_progress_for_long_runs() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    for i in 0..120 {
        fs::write(root.join(format!("f{i:03}.txt")), "x").unwrap();
    }

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("xunbak progress:"));
    assert!(stderr.contains("files="));
    assert!(stderr.contains("bytes="));
}

#[test]
fn cli_backup_restore_verify_split_container() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(80)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(80)).unwrap();
    fs::write(root.join("c.txt"), "c".repeat(80)).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "--split-size",
        "1900",
        "-m",
        "t",
    ]));

    assert!(root.join("backup.xunbak.001").exists());
    assert!(root.join("backup.xunbak.002").exists());

    let restore_dir = root.join("restore");
    run_ok(env.cmd().args([
        "restore",
        root.join("backup.xunbak").to_str().unwrap(),
        "--to",
        restore_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));
    assert_eq!(
        fs::read_to_string(restore_dir.join("a.txt")).unwrap(),
        "a".repeat(80)
    );

    let verify = run_ok(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--level",
        "paranoid",
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(value.get("passed").and_then(Value::as_bool), Some(true));
}
