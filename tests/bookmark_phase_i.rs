#![cfg(windows)]

#[path = "support/mod.rs"]
mod common;

use common::{run_ok, TestEnv};
use serde_json::Value;
use std::fs;

fn stdout_text(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
}

fn write_store(env: &TestEnv, body: &str) {
    fs::write(env.root.join(".xun.bookmark.json"), body).unwrap();
}

#[test]
fn pinned_bookmark_wins_same_query_in_list_order() {
    let env = TestEnv::new();
    let pinned = env.root.join("client-pinned");
    let plain = env.root.join("client-plain");
    fs::create_dir_all(&pinned).unwrap();
    fs::create_dir_all(&plain).unwrap();

    run_ok(
        env.cmd()
            .args(["bookmark", "set", "client-main", pinned.to_str().unwrap()]),
    );
    run_ok(
        env.cmd()
            .args(["bookmark", "set", "client-side", plain.to_str().unwrap()]),
    );
    run_ok(env.cmd().args(["bookmark", "pin", "client-main"]));

    let out = run_ok(
        env.cmd()
            .args(["bookmark", "z", "client", "--list", "--tsv"]),
    );
    let stdout = stdout_text(&out);
    let first = stdout.lines().next().unwrap_or("");
    assert!(first.starts_with("client-main\t"), "unexpected first line: {first}");
}

#[test]
fn base_scope_filters_outside_matches() {
    let env = TestEnv::new();
    let base = env.root.join("base");
    let inside = base.join("client-api");
    let outside = env.root.join("other").join("client-web");
    fs::create_dir_all(&inside).unwrap();
    fs::create_dir_all(outside.parent().unwrap()).unwrap();
    fs::create_dir_all(&outside).unwrap();

    run_ok(
        env.cmd()
            .args(["bookmark", "set", "client-api", inside.to_str().unwrap()]),
    );
    run_ok(
        env.cmd()
            .args(["bookmark", "set", "client-web", outside.to_str().unwrap()]),
    );

    let out = run_ok(
        env.cmd()
            .args([
                "bookmark",
                "z",
                "client",
                "--list",
                "--tsv",
                "--base",
                base.to_str().unwrap(),
            ]),
    );
    let stdout = stdout_text(&out);
    assert!(stdout.contains("client-api"));
    assert!(!stdout.contains("client-web"));
}

#[test]
fn workspace_scope_filters_results() {
    let env = TestEnv::new();
    write_store(
        &env,
        r#"{
  "schema_version": 1,
  "bookmarks": [
    {
      "id":"1","name":"api","name_norm":"api","path":"C:/work/api","path_norm":"c:/work/api",
      "source":"Explicit","pinned":false,"tags":[],"desc":"","workspace":"xunyu","created_at":1,
      "last_visited":1700000100,"visit_count":2,"frecency_score":2.0
    },
    {
      "id":"2","name":"web","name_norm":"web","path":"C:/work/web","path_norm":"c:/work/web",
      "source":"Explicit","pinned":false,"tags":[],"desc":"","workspace":"other","created_at":1,
      "last_visited":1700000000,"visit_count":2,"frecency_score":2.0
    }
  ]
}"#,
    );

    let out = run_ok(
        env.cmd()
            .args(["bookmark", "z", "--list", "--json", "--workspace", "xunyu"]),
    );
    let stdout = stdout_text(&out);
    assert!(stdout.contains("\"api\""));
    assert!(!stdout.contains("\"web\""));
}

#[test]
fn native_json_import_merge_preserves_tags_and_visits() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(
        env.cmd()
            .args(["bookmark", "set", "home", work.to_str().unwrap(), "-t", "a"]),
    );
    run_ok(env.cmd().args(["bookmark", "touch", "home"]));
    run_ok(env.cmd().args(["bookmark", "touch", "home"]));

    let import_path = env.root.join("merge.json");
    fs::write(
        &import_path,
        r#"[
  {
    "name":"home",
    "path":"",
    "tags":["b"],
    "visits":1,
    "last_visited":100
  }
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

    let out = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "home")
        .unwrap();
    let tags: Vec<&str> = item["tags"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tag| tag.as_str())
        .collect();
    assert!(tags.contains(&"a"));
    assert!(tags.contains(&"b"));
    assert!(item["visits"].as_u64().unwrap_or(0) >= 2);
}

#[test]
fn native_json_import_overwrite_replaces_tags_and_visits() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    let other = env.root.join("other");
    fs::create_dir_all(&work).unwrap();
    fs::create_dir_all(&other).unwrap();

    run_ok(
        env.cmd()
            .args(["bookmark", "set", "home", work.to_str().unwrap(), "-t", "a"]),
    );
    run_ok(env.cmd().args(["bookmark", "touch", "home"]));

    let import_path = env.root.join("overwrite.json");
    fs::write(
        &import_path,
        format!(
            r#"[
  {{
    "name":"home",
    "path":"{}",
    "tags":["b"],
    "visits":1,
    "last_visited":99
  }}
]"#,
            other.to_string_lossy().replace('\\', "/")
        ),
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
            "overwrite",
            "--yes",
        ]),
    );

    let out = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "home")
        .unwrap();
    let tags: Vec<&str> = item["tags"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tag| tag.as_str())
        .collect();
    assert_eq!(item["path"].as_str().unwrap(), other.to_string_lossy().replace('\\', "/"));
    assert_eq!(tags, vec!["b"]);
    assert_eq!(item["visits"].as_u64(), Some(1));
}

#[test]
fn history_import_creates_imported_entries() {
    let env = TestEnv::new();
    let history = env.root.join("ConsoleHost_history.txt");
    fs::write(
        &history,
        "cd C:\\work\\foo\nz C:\\work\\bar\nWrite-Host ignore-me\n",
    )
    .unwrap();

    run_ok(
        env.cmd().args([
            "bookmark",
            "import",
            "--from",
            "history",
            "-i",
            history.to_str().unwrap(),
        ]),
    );

    let out = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let stdout = stdout_text(&out);
    assert!(stdout.contains("\"source\":\"imported\""));
    assert!(stdout.contains("C:/work/foo"));
    assert!(stdout.contains("C:/work/bar"));
}

#[test]
fn completion_script_lists_new_bookmark_subcommands() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args(["completion", "bash"]));
    let stdout = stdout_text(&out);
    for name in ["zi", "oi", "learn", "pin", "unpin", "delete", "keys", "all"] {
        assert!(
            stdout.contains(&format!(" {name}")) || stdout.contains(&format!("\"{name}\"")),
            "missing completion item: {name}\n{stdout}"
        );
    }
}
