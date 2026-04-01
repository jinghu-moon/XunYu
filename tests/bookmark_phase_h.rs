#![cfg(windows)]

#[path = "support/mod.rs"]
mod common;

use common::{run_ok, TestEnv};
use std::fs;

fn stdout_text(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
}

fn write_store(env: &TestEnv, body: &str) {
    fs::write(env.root.join(".xun.bookmark.json"), body).unwrap();
}

#[test]
fn zi_non_interactive_falls_back_to_top1_jump() {
    let env = TestEnv::new();
    let api = env.root.join("client-api");
    let web = env.root.join("client-web");
    fs::create_dir_all(&api).unwrap();
    fs::create_dir_all(&web).unwrap();
    write_store(
        &env,
        &format!(
            r#"{{
  "schema_version": 1,
  "bookmarks": [
    {{"name":"client-api","path":"{}","tags":[],"visit_count":10,"last_visited":1700000000}},
    {{"name":"client-web","path":"{}","tags":[],"visit_count":5,"last_visited":1700000000}}
  ]
}}"#,
            api.to_string_lossy().replace('\\', "/"),
            web.to_string_lossy().replace('\\', "/")
        ),
    );

    let out = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("zi")
            .arg("client"),
    );
    let stdout = stdout_text(&out);
    assert!(stdout.contains(&format!(
        "__BM_CD__ {}",
        api.to_string_lossy().replace('\\', "/")
    )));
}

#[test]
fn oi_non_interactive_falls_back_to_top1_without_crashing() {
    let env = TestEnv::new();
    let dir = env.root.join("open-target");
    fs::create_dir_all(&dir).unwrap();
    write_store(
        &env,
        &format!(
            r#"{{
  "schema_version": 1,
  "bookmarks": [
    {{"name":"open-target","path":"{}","tags":[],"visit_count":10,"last_visited":1700000000}}
  ]
}}"#,
            dir.to_string_lossy().replace('\\', "/")
        ),
    );

    let _out = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("oi")
            .arg("open-target"),
    );
}

#[test]
fn complete_bookmark_z_uses_query_core_order() {
    let env = TestEnv::new();
    let api = env.root.join("client-api");
    let web = env.root.join("client-web");
    fs::create_dir_all(&api).unwrap();
    fs::create_dir_all(&web).unwrap();
    write_store(
        &env,
        &format!(
            r#"{{
  "schema_version": 1,
  "bookmarks": [
    {{"name":"client-api","path":"{}","tags":[],"visit_count":10,"last_visited":1700000000}},
    {{"name":"client-web","path":"{}","tags":[],"visit_count":5,"last_visited":1700000000}}
  ]
}}"#,
            api.to_string_lossy().replace('\\', "/"),
            web.to_string_lossy().replace('\\', "/")
        ),
    );

    let out = run_ok(
        env.cmd()
            .arg("__complete")
            .arg("bookmark")
            .arg("z")
            .arg("client"),
    );
    let stdout = stdout_text(&out);
    assert!(stdout.contains("client-api"));
    assert!(stdout.contains("client-web"));
    let pos_api = stdout.find("client-api").unwrap();
    let pos_web = stdout.find("client-web").unwrap();
    assert!(pos_api < pos_web);
}

#[test]
fn complete_bookmark_delete_and_unpin_suggest_bookmark_names() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();
    write_store(
        &env,
        &format!(
            r#"{{
  "schema_version": 1,
  "bookmarks": [
    {{"name":"home","path":"{}","tags":[],"visit_count":10,"last_visited":1700000000,"pinned":true}}
  ]
}}"#,
            work.to_string_lossy().replace('\\', "/")
        ),
    );

    let delete_out = run_ok(
        env.cmd()
            .arg("__complete")
            .arg("bookmark")
            .arg("delete")
            .arg("ho"),
    );
    let delete_stdout = stdout_text(&delete_out);
    assert!(delete_stdout.contains("home"));

    let unpin_out = run_ok(
        env.cmd()
            .arg("__complete")
            .arg("bookmark")
            .arg("unpin")
            .arg("ho"),
    );
    let unpin_stdout = stdout_text(&unpin_out);
    assert!(unpin_stdout.contains("home"));
}

#[test]
fn completion_same_order_with_cache_hit() {
    let env = TestEnv::new();
    let api = env.root.join("client-api");
    let web = env.root.join("client-web");
    fs::create_dir_all(&api).unwrap();
    fs::create_dir_all(&web).unwrap();
    write_store(
        &env,
        &format!(
            r#"{{
  "schema_version": 1,
  "bookmarks": [
    {{"name":"client-api","path":"{}","tags":[],"visit_count":10,"last_visited":1700000000}},
    {{"name":"client-web","path":"{}","tags":[],"visit_count":5,"last_visited":1700000000}}
  ]
}}"#,
            api.to_string_lossy().replace('\\', "/"),
            web.to_string_lossy().replace('\\', "/")
        ),
    );

    run_ok(
        env.cmd()
            .env("XUN_BM_ENABLE_BINARY_CACHE", "1")
            .arg("bookmark")
            .arg("list")
            .arg("--format")
            .arg("json"),
    );

    let out = run_ok(
        env.cmd()
            .env("XUN_BM_ENABLE_BINARY_CACHE", "1")
            .arg("__complete")
            .arg("bookmark")
            .arg("z")
            .arg("client"),
    );
    let stdout = stdout_text(&out);
    let pos_api = stdout.find("client-api").unwrap();
    let pos_web = stdout.find("client-web").unwrap();
    assert!(pos_api < pos_web);
}
