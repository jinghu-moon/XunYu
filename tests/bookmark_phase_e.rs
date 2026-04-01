#![cfg(windows)]

#[path = "support/mod.rs"]
mod common;

use common::{run_err, run_ok, TestEnv};
use std::fs;

fn stdout_text(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
}

fn stderr_text(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n")
}

fn write_store(env: &TestEnv, body: &str) {
    let path = env.root.join(".xun.bookmark.json");
    fs::write(path, body).unwrap();
}

#[test]
fn z_and_o_list_have_same_order() {
    let env = TestEnv::new();
    write_store(
        &env,
        r#"{
  "schema_version": 1,
  "bookmarks": [
    {"name":"client-api","path":"C:/work/client-api","tags":["work"],"visit_count":10,"last_visited":1700000000},
    {"name":"client-web","path":"C:/work/client-web","tags":["work"],"visit_count":5,"last_visited":1700000000}
  ]
}"#,
    );

    let z = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("z")
            .arg("client")
            .arg("--list")
            .arg("--tsv"),
    );
    let o = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("o")
            .arg("client")
            .arg("--list")
            .arg("--tsv"),
    );

    assert_eq!(stdout_text(&z), stdout_text(&o));
}

#[test]
fn preview_does_not_emit_bm_cd() {
    let env = TestEnv::new();
    write_store(
        &env,
        r#"{
  "schema_version": 1,
  "bookmarks": [
    {"name":"client-api","path":"C:/work/client-api","tags":[],"visit_count":10,"last_visited":1700000000}
  ]
}"#,
    );

    let out = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("z")
            .arg("client")
            .arg("--preview"),
    );
    let stdout = stdout_text(&out);
    let stderr = stderr_text(&out);
    assert!(!stdout.contains("__BM_CD__"));
    assert!(stderr.contains("Preview mode"));
}

#[test]
fn why_outputs_factor_lines() {
    let env = TestEnv::new();
    write_store(
        &env,
        r#"{
  "schema_version": 1,
  "bookmarks": [
    {"name":"client-api","path":"C:/work/client-api","tags":["work"],"visit_count":10,"last_visited":1700000000}
  ]
}"#,
    );

    let out = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("z")
            .arg("client")
            .arg("--why"),
    );
    let stdout = stdout_text(&out);
    assert!(stdout.contains("MatchScore"));
    assert!(stdout.contains("FrecencyMult"));
    assert!(stdout.contains("ScopeMult"));
    assert!(stdout.contains("FinalScore"));
}

#[test]
fn dead_link_detected_for_local_path() {
    let env = TestEnv::new();
    write_store(
        &env,
        r#"{
  "schema_version": 1,
  "bookmarks": [
    {"name":"missing","path":"C:/definitely/missing/path","tags":[],"visit_count":10,"last_visited":1700000000}
  ]
}"#,
    );

    let out = run_err(env.cmd().arg("bookmark").arg("z").arg("missing"));
    let stderr = stderr_text(&out);
    assert!(stderr.contains("missing path"));
}
