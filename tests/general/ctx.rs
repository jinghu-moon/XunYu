#![cfg(windows)]

#[path = "../support/mod.rs"]
mod common;

use common::*;
use serde_json::Value;
use std::fs;

#[test]
fn ctx_set_list_show_roundtrip() {
    let env = TestEnv::new();
    let ctx_file = env.root.join(".xun.ctx.json");
    let proj = env.root.join("proj");
    fs::create_dir_all(&proj).unwrap();

    run_ok(env.cmd().env("XUN_CTX_FILE", &ctx_file).args([
        "ctx",
        "set",
        "work",
        "--path",
        proj.to_str().unwrap(),
        "--tag",
        "work,rust",
        "--env",
        "FOO=bar",
    ]));

    let out = run_ok(
        env.cmd()
            .env("XUN_CTX_FILE", &ctx_file)
            .args(["ctx", "list", "--format", "json"]),
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "work")
        .unwrap();
    assert_eq!(item["path"].as_str().unwrap(), proj.to_str().unwrap());

    let out = run_ok(
        env.cmd()
            .env("XUN_CTX_FILE", &ctx_file)
            .args(["ctx", "show", "work", "--format", "json"]),
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["name"], "work");
    assert_eq!(v["env"]["FOO"], "bar");
}

#[test]
fn ctx_set_merges_fields() {
    let env = TestEnv::new();
    let ctx_file = env.root.join(".xun.ctx.json");
    let proj = env.root.join("proj");
    fs::create_dir_all(&proj).unwrap();

    run_ok(env.cmd().env("XUN_CTX_FILE", &ctx_file).args([
        "ctx",
        "set",
        "work",
        "--path",
        proj.to_str().unwrap(),
        "--tag",
        "work",
        "--env",
        "FOO=bar",
    ]));

    run_ok(
        env.cmd()
            .env("XUN_CTX_FILE", &ctx_file)
            .args(["ctx", "set", "work", "--tag", "newtag"]),
    );

    let out = run_ok(
        env.cmd()
            .env("XUN_CTX_FILE", &ctx_file)
            .args(["ctx", "show", "work", "--format", "json"]),
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["path"].as_str().unwrap(), proj.to_str().unwrap());
    assert_eq!(v["env"]["FOO"], "bar");
    assert!(v["tags"].as_array().unwrap().iter().any(|t| t == "newtag"));
}

#[test]
fn ctx_use_off_outputs_magic_and_session() {
    let env = TestEnv::new();
    let ctx_file = env.root.join(".xun.ctx.json");
    let ctx_state = env.root.join("ctx-session.json");
    let proj = env.root.join("proj");
    fs::create_dir_all(&proj).unwrap();

    run_ok(env.cmd().env("XUN_CTX_FILE", &ctx_file).args([
        "ctx",
        "set",
        "work",
        "--path",
        proj.to_str().unwrap(),
        "--tag",
        "work",
    ]));

    let out = run_ok(
        env.cmd()
            .env("XUN_CTX_FILE", &ctx_file)
            .env("XUN_CTX_STATE", &ctx_state)
            .current_dir(&env.root)
            .args(["ctx", "use", "work"]),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let first = stdout.lines().next().unwrap_or("");
    assert!(first.starts_with("__CD__:"));
    assert!(stdout.contains("__ENV_SET__:XUN_CTX=work"));
    assert!(ctx_state.exists());

    let session = fs::read_to_string(&ctx_state).unwrap();
    let v: Value = serde_json::from_str(&session).unwrap();
    assert_eq!(v["active"], "work");

    let out = run_ok(
        env.cmd()
            .env("XUN_CTX_STATE", &ctx_state)
            .env("XUN_CTX_FILE", &ctx_file)
            .args(["ctx", "off"]),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("__ENV_UNSET__:XUN_CTX_STATE"));
    assert!(stdout.contains(proj.to_str().unwrap()) || stdout.contains(env.root.to_str().unwrap()));
    assert!(!ctx_state.exists());
}

#[test]
fn ctx_use_handles_paths_with_spaces() {
    let env = TestEnv::new();
    let ctx_file = env.root.join(".xun.ctx.json");
    let ctx_state = env.root.join("ctx-session.json");
    let proj = env.root.join("space dir");
    fs::create_dir_all(&proj).unwrap();

    run_ok(env.cmd().env("XUN_CTX_FILE", &ctx_file).args([
        "ctx",
        "set",
        "space",
        "--path",
        proj.to_str().unwrap(),
    ]));

    let out = run_ok(
        env.cmd()
            .env("XUN_CTX_FILE", &ctx_file)
            .env("XUN_CTX_STATE", &ctx_state)
            .args(["ctx", "use", "space"]),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(proj.to_str().unwrap()));
}

#[test]
fn ctx_off_without_session_is_ok() {
    let env = TestEnv::new();
    let ctx_state = env.root.join("missing-session.json");
    let out = run_ok(
        env.cmd()
            .env("XUN_CTX_STATE", &ctx_state)
            .args(["ctx", "off"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("No active profile"));
}

#[test]
fn ctx_set_rejects_reserved_name() {
    let env = TestEnv::new();
    let ctx_file = env.root.join(".xun.ctx.json");
    let proj = env.root.join("proj");
    fs::create_dir_all(&proj).unwrap();

    let out = run_err(env.cmd().env("XUN_CTX_FILE", &ctx_file).args([
        "ctx",
        "set",
        "list",
        "--path",
        proj.to_str().unwrap(),
    ]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Invalid profile name"));
}

#[test]
fn default_tag_filters_list_and_z() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    let home = env.root.join("home");
    fs::create_dir_all(&work).unwrap();
    fs::create_dir_all(&home).unwrap();

    run_ok(
        env.cmd()
            .args(["bookmark", "set", "proj-work", work.to_str().unwrap(), "-t", "work"]),
    );
    run_ok(
        env.cmd()
            .args(["bookmark", "set", "proj-home", home.to_str().unwrap(), "-t", "home"]),
    );

    let out = run_ok(
        env.cmd()
            .env("XUN_DEFAULT_TAG", "work")
            .args(["bookmark", "list", "--format", "json"]),
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(v[0]["name"], "proj-work");

    let out = run_ok(
        env.cmd()
            .env("XUN_DEFAULT_TAG", "work")
            .args(["bookmark", "z", "proj"]),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&work.to_string_lossy().replace('\\', "/")));
    assert!(!stdout.contains(&home.to_string_lossy().replace('\\', "/")));
}

#[test]
fn ctx_off_second_call_reports_no_active() {
    let env = TestEnv::new();
    let ctx_file = env.root.join(".xun.ctx.json");
    let ctx_state = env.root.join("ctx-session.json");
    let proj = env.root.join("proj");
    fs::create_dir_all(&proj).unwrap();

    run_ok(env.cmd().env("XUN_CTX_FILE", &ctx_file).args([
        "ctx",
        "set",
        "work",
        "--path",
        proj.to_str().unwrap(),
    ]));
    run_ok(
        env.cmd()
            .env("XUN_CTX_FILE", &ctx_file)
            .env("XUN_CTX_STATE", &ctx_state)
            .args(["ctx", "use", "work"]),
    );
    run_ok(
        env.cmd()
            .env("XUN_CTX_STATE", &ctx_state)
            .args(["ctx", "off"]),
    );

    let out = run_ok(
        env.cmd()
            .env("XUN_CTX_STATE", &ctx_state)
            .args(["ctx", "off"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("No active profile"));
}
