#![cfg(all(windows, feature = "redirect"))]

mod common;

use common::*;
use serde_json::Value;
use std::fs;

fn write_undo_config(env: &TestEnv, on_conflict: &str) {
    let cfg = format!(
        r#"
{{
  "redirect": {{
    "profiles": {{
      "default": {{
        "rules": [
          {{ "name": "Images", "match": {{ "ext": ["jpg"] }}, "dest": "./Images" }}
        ],
        "unmatched": "skip",
        "on_conflict": "{on_conflict}"
      }}
    }}
  }}
}}
"#
    );
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();
}

fn parse_tx(params: &str) -> Option<String> {
    let idx = params.find("tx=")?;
    let rest = &params[(idx + 3)..];
    let end = rest.find(' ').unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn last_redirect_tx(env: &TestEnv) -> String {
    let log = fs::read_to_string(env.audit_path()).unwrap_or_default();
    for line in log.lines().rev() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let action = v.get("action").and_then(Value::as_str).unwrap_or("");
        if action != "redirect_move" && action != "redirect_copy" && action != "redirect_dedup" {
            continue;
        }
        let params = v.get("params").and_then(Value::as_str).unwrap_or("");
        if let Some(tx) = parse_tx(params) {
            return tx;
        }
    }
    panic!("missing redirect tx in audit log");
}

#[test]
fn redirect_undo_restores_moved_file() {
    let env = TestEnv::new();
    write_undo_config(&env, "rename_new");

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );
    assert!(!src.join("a.jpg").exists());
    assert!(src.join("Images").join("a.jpg").exists());

    let tx = last_redirect_tx(&env);
    run_ok(
        env.cmd()
            .args(["redirect", "--undo", &tx, "--format", "json"]),
    );

    assert!(
        src.join("a.jpg").exists(),
        "expected file restored to source"
    );
    assert!(
        !src.join("Images").join("a.jpg").exists(),
        "expected destination file moved back"
    );
}

#[test]
fn redirect_undo_removes_copied_file() {
    let env = TestEnv::new();
    write_undo_config(&env, "rename_new");

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--copy",
        "--format",
        "tsv",
    ]));
    assert!(src.join("a.jpg").exists());
    assert!(src.join("Images").join("a.jpg").exists());

    let tx = last_redirect_tx(&env);
    run_ok(
        env.cmd()
            .args(["redirect", "--undo", &tx, "--format", "tsv"]),
    );

    assert!(src.join("a.jpg").exists(), "source remains");
    assert!(
        !src.join("Images").join("a.jpg").exists(),
        "copied file should be removed"
    );
}

#[test]
fn redirect_undo_reports_dedup_as_unrestorable() {
    let env = TestEnv::new();
    write_undo_config(&env, "hash_dedup");

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "same").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "same").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );
    assert!(!src.join("a.jpg").exists(), "dedup should delete source");
    assert!(src.join("Images").join("a.jpg").exists());

    let tx = last_redirect_tx(&env);
    let out = run_ok(
        env.cmd()
            .args(["redirect", "--undo", &tx, "--format", "json"]),
    );

    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(
        arr.iter().any(|x| x["action"] == "undo_dedup"
            && x["result"] == "failed"
            && x["reason"] == "dedup_cannot_restore_deleted_source"),
        "expected a clear failure entry for dedup undo"
    );
}

#[test]
fn redirect_undo_parses_dst_even_if_path_contains_copy_param_marker() {
    let env = TestEnv::new();

    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images copy=folder" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );
    assert!(!src.join("a.jpg").exists());
    assert!(src.join("Images copy=folder").join("a.jpg").exists());

    let tx = last_redirect_tx(&env);
    run_ok(
        env.cmd()
            .args(["redirect", "--undo", &tx, "--format", "json"]),
    );

    assert!(
        src.join("a.jpg").exists(),
        "expected file restored to source"
    );
    assert!(
        !src.join("Images copy=folder").join("a.jpg").exists(),
        "expected destination file moved back"
    );
}
