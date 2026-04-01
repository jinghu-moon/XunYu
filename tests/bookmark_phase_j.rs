#![cfg(windows)]

#[path = "support/mod.rs"]
mod common;

use common::{run_err, run_ok, TestEnv};
use serde_json::Value;
use std::fs;

fn list_json(env: &TestEnv) -> Value {
    let out = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn undo_after_set_removes_bookmark() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "undo"]));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap().clone();
    assert!(arr.is_empty());
}

#[test]
fn undo_after_rename_restores_old_name() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "old", dir.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "rename", "old", "new"]));
    run_ok(env.cmd().args(["bookmark", "undo"]));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    assert!(arr.iter().any(|item| item["name"] == "old"));
    assert!(!arr.iter().any(|item| item["name"] == "new"));
}

#[test]
fn undo_after_delete_restores_bookmark() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));
    run_ok(env.cmd().args(["del", "-bm", "home"]));
    run_ok(env.cmd().args(["bookmark", "undo"]));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    assert!(arr.iter().any(|item| item["name"] == "home"));
}

#[test]
fn undo_after_pin_restores_unpinned_state() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "pin", "home"]));
    run_ok(env.cmd().args(["bookmark", "undo"]));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    let item = arr.iter().find(|item| item["name"] == "home").unwrap();
    assert_eq!(item["pinned"].as_bool(), Some(false));
}

#[test]
fn undo_after_import_restores_previous_state() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();
    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));

    let import_path = env.root.join("import.json");
    fs::write(
        &import_path,
        r#"[
  { "name":"api", "path":"C:/work/api", "tags":[], "visits":0, "last_visited":0 }
]"#,
    )
    .unwrap();
    run_ok(
        env.cmd().args([
            "bookmark",
            "import",
            "--format",
            "json",
            "--input",
            import_path.to_str().unwrap(),
            "--mode",
            "merge",
        ]),
    );

    run_ok(env.cmd().args(["bookmark", "undo"]));
    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    assert!(arr.iter().any(|item| item["name"] == "home"));
    assert!(!arr.iter().any(|item| item["name"] == "api"));
}

#[test]
fn undo_empty_history_returns_error() {
    let env = TestEnv::new();
    let out = run_err(env.cmd().args(["bookmark", "undo"]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Nothing to undo"));
}

#[test]
fn undo_steps_two_restores_older_snapshot() {
    let env = TestEnv::new();
    let dir1 = env.root.join("one");
    let dir2 = env.root.join("two");
    fs::create_dir_all(&dir1).unwrap();
    fs::create_dir_all(&dir2).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "one", dir1.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "set", "two", dir2.to_str().unwrap()]));

    run_ok(env.cmd().args(["bookmark", "undo", "-n", "2"]));
    let binding = list_json(&env);
    let arr = binding.as_array().unwrap().clone();
    assert!(arr.is_empty());
}

#[test]
fn redo_after_undo_reapplies_change() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "rename", "home", "main"]));
    run_ok(env.cmd().args(["bookmark", "undo"]));
    run_ok(env.cmd().args(["bookmark", "redo"]));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    assert!(arr.iter().any(|item| item["name"] == "main"));
    assert!(!arr.iter().any(|item| item["name"] == "home"));
}

#[test]
fn redo_empty_history_returns_error() {
    let env = TestEnv::new();
    let out = run_err(env.cmd().args(["bookmark", "redo"]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Nothing to redo"));
}

#[test]
fn bookmark_delete_subcommand_removes_entry() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "delete", "home", "-y"]));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    assert!(!arr.iter().any(|item| item["name"] == "home"));
}

#[test]
fn unpin_command_clears_pinned_state() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "pin", "home"]));
    run_ok(env.cmd().args(["bookmark", "unpin", "home"]));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    let item = arr.iter().find(|item| item["name"] == "home").unwrap();
    assert_eq!(item["pinned"].as_bool(), Some(false));
}

#[test]
fn undo_after_unpin_restores_pinned_state() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "pin", "home"]));
    run_ok(env.cmd().args(["bookmark", "unpin", "home"]));
    run_ok(env.cmd().args(["bookmark", "undo"]));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    let item = arr.iter().find(|item| item["name"] == "home").unwrap();
    assert_eq!(item["pinned"].as_bool(), Some(true));
}

#[test]
fn cmd_z_same_result_with_cache_hit_and_json_fallback() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));

    let json_out = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("z")
            .arg("ho")
            .arg("--list")
            .arg("--json"),
    );
    let cache_out = run_ok(
        env.cmd()
            .env("XUN_BM_ENABLE_BINARY_CACHE", "1")
            .arg("bookmark")
            .arg("z")
            .arg("ho")
            .arg("--list")
            .arg("--json"),
    );
    assert_eq!(json_out.stdout, cache_out.stdout);
}

#[test]
fn cmd_zi_same_result_with_cache_hit_and_json_fallback() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));

    let json_out = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("zi")
            .arg("ho")
            .arg("--list")
            .arg("--json"),
    );
    let cache_out = run_ok(
        env.cmd()
            .env("XUN_BM_ENABLE_BINARY_CACHE", "1")
            .arg("bookmark")
            .arg("zi")
            .arg("ho")
            .arg("--list")
            .arg("--json"),
    );
    assert_eq!(json_out.stdout, cache_out.stdout);
}

#[test]
fn cmd_oi_same_result_with_cache_hit_and_json_fallback() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));

    let json_out = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("oi")
            .arg("ho")
            .arg("--list")
            .arg("--json"),
    );
    let cache_out = run_ok(
        env.cmd()
            .env("XUN_BM_ENABLE_BINARY_CACHE", "1")
            .arg("bookmark")
            .arg("oi")
            .arg("ho")
            .arg("--list")
            .arg("--json"),
    );
    assert_eq!(json_out.stdout, cache_out.stdout);
}

#[test]
fn bookmark_load_timing_emits_cache_hit_or_miss_reason() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();
    let debug_file = env.root.join("bookmark-debug.log");

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));

    let _miss = run_ok(
        env.cmd()
            .env("XUN_BM_DEBUG_FILE", &debug_file)
            .arg("bookmark")
            .arg("z")
            .arg("ho")
            .arg("--list")
            .arg("--json"),
    );
    let _hit = run_ok(
        env.cmd()
            .env("XUN_BM_ENABLE_BINARY_CACHE", "1")
            .env("XUN_BM_DEBUG_FILE", &debug_file)
            .arg("bookmark")
            .arg("z")
            .arg("ho")
            .arg("--list")
            .arg("--json"),
    );

    let text = fs::read_to_string(&debug_file).unwrap();
    assert!(text.contains("cache=miss") || text.contains("cache=hit"));
}

#[test]
fn undo_after_tag_add_restores_previous_tags() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "tag", "add", "home", "work,docs"]));
    run_ok(env.cmd().args(["bookmark", "undo"]));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    let item = arr.iter().find(|item| item["name"] == "home").unwrap();
    assert_eq!(item["tags"].as_array().unwrap().len(), 0);
}

#[test]
fn undo_after_gc_purge_restores_deleted_entries() {
    let env = TestEnv::new();
    let missing = env.root.join("missing-dir");

    run_ok(
        env.cmd()
            .args(["bookmark", "set", "ghost", missing.to_str().unwrap()]),
    );
    run_ok(env.cmd().args(["bookmark", "gc", "--purge"]));
    run_ok(env.cmd().args(["bookmark", "undo"]));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    assert!(arr.iter().any(|item| item["name"] == "ghost"));
}

#[test]
fn redo_is_cleared_after_new_mutation() {
    let env = TestEnv::new();
    let one = env.root.join("one");
    let two = env.root.join("two");
    fs::create_dir_all(&one).unwrap();
    fs::create_dir_all(&two).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "one", one.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "undo"]));
    run_ok(env.cmd().args(["bookmark", "set", "two", two.to_str().unwrap()]));

    let out = run_err(env.cmd().args(["bookmark", "redo"]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Nothing to redo"));

    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    assert!(arr.iter().any(|item| item["name"] == "two"));
    assert!(!arr.iter().any(|item| item["name"] == "one"));
}

#[test]
fn undo_and_redo_multiple_steps_preserve_operation_order() {
    let env = TestEnv::new();
    let dir = env.root.join("work");
    fs::create_dir_all(&dir).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", dir.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "rename", "home", "main"]));
    run_ok(env.cmd().args(["bookmark", "pin", "main"]));

    run_ok(env.cmd().args(["bookmark", "undo", "-n", "2"]));
    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    let item = arr.iter().find(|item| item["name"] == "home").unwrap();
    assert_eq!(item["pinned"].as_bool(), Some(false));

    run_ok(env.cmd().args(["bookmark", "redo", "-n", "2"]));
    let binding = list_json(&env);
    let arr = binding.as_array().unwrap();
    let item = arr.iter().find(|item| item["name"] == "main").unwrap();
    assert_eq!(item["pinned"].as_bool(), Some(true));
}
