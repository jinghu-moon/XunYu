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
    fs::write(env.root.join(".xun.bookmark.json"), body).unwrap();
}

#[test]
fn set_with_desc_is_visible_in_list_json() {
    let env = TestEnv::new();
    let target = env.root.join("desc-target");
    fs::create_dir_all(&target).unwrap();

    run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("set")
            .arg("main-proj")
            .arg(target.to_string_lossy().to_string())
            .arg("--desc")
            .arg("主项目"),
    );

    let out = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("list")
            .arg("-f")
            .arg("json"),
    );
    let stdout = stdout_text(&out);
    assert!(stdout.contains("主项目"));
    assert!(stdout.contains("main-proj"));
}

#[test]
fn recent_filters_by_workspace_and_since() {
    let env = TestEnv::new();
    let now = 1_700_100_000u64;
    write_store(
        &env,
        &format!(
            r#"{{
  "schema_version": 1,
  "bookmarks": [
    {{
      "id":"1","name":"foo","name_norm":"foo","path":"C:/work/foo","path_norm":"c:/work/foo",
      "source":"Explicit","pinned":false,"tags":["work"],"desc":"","workspace":"xunyu","created_at":1,
      "last_visited":{now},"visit_count":2,"frecency_score":10.0
    }},
    {{
      "id":"2","name":"bar","name_norm":"bar","path":"C:/work/bar","path_norm":"c:/work/bar",
      "source":"Explicit","pinned":false,"tags":["work"],"desc":"","workspace":"other","created_at":1,
      "last_visited":{old},"visit_count":2,"frecency_score":10.0
    }}
  ]
}}"#,
            now = now,
            old = now - 10 * 86_400
        ),
    );

    let out = run_ok(
        env.cmd()
            .env("XUN_TEST_NOW_SECS", now.to_string())
            .arg("bookmark")
            .arg("recent")
            .arg("--workspace")
            .arg("xunyu")
            .arg("--since")
            .arg("7d")
            .arg("-f")
            .arg("json"),
    );
    let stdout = stdout_text(&out);
    assert!(stdout.contains("foo"));
    assert!(!stdout.contains("bar"));
}

#[test]
fn gc_dry_run_lists_but_does_not_delete() {
    let env = TestEnv::new();
    write_store(
        &env,
        r#"{
  "schema_version": 1,
  "bookmarks": [
    {"id":"1","name":"dead","name_norm":"dead","path":"C:/missing/path","path_norm":"c:/missing/path","source":"Explicit","pinned":false,"tags":[],"desc":"","workspace":null,"created_at":1,"last_visited":1,"visit_count":1,"frecency_score":1.0}
  ]
}"#,
    );

    let out = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("gc")
            .arg("--dry-run")
            .arg("-f")
            .arg("json"),
    );
    let stdout = stdout_text(&out);
    assert!(stdout.contains("dead"));

    let raw = fs::read_to_string(env.root.join(".xun.bookmark.json")).unwrap();
    assert!(raw.contains("\"dead\""));
}

#[test]
fn fuzzy_subcommand_returns_error() {
    let env = TestEnv::new();
    let out = run_err(env.cmd().arg("bookmark").arg("fuzzy").arg("foo"));
    let stderr = stderr_text(&out);
    assert!(stderr.contains("Run xun --help") || stderr.contains("unrecognized"));
}
