use crate::common::*;
use serde_json::Value;
use std::fs;

#[test]
fn set_warns_on_missing_path() {
    let env = TestEnv::new();
    let missing = env.root.join("missing");

    let out = run_ok(
        env.cmd()
            .args(["bookmark", "set", "missing", missing.to_str().unwrap()]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Warning: Path does not exist"));

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "missing")
        .unwrap();
    assert_eq!(
        item["path"].as_str().unwrap(),
        missing.to_string_lossy().replace('\\', "/")
    );
}

#[test]
fn z_outputs_cd_magic() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", work.to_str().unwrap()]));

    let output = run_ok(env.cmd().args(["bookmark", "z", "home"]));
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(out.trim().starts_with("__BM_CD__ "));
    assert!(out.contains(&work.to_string_lossy().replace('\\', "/")));
}

#[test]
fn z_fuzzy_selects_highest_scored_match_in_non_interactive() {
    let env = TestEnv::new();
    let d1 = env.root.join("d1");
    let d2 = env.root.join("d2");
    fs::create_dir_all(&d1).unwrap();
    fs::create_dir_all(&d2).unwrap();

    // "ab" is an exact consecutive match for pattern "ab", so it should win vs "axb".
    run_ok(env.cmd().args(["bookmark", "set", "ab", d1.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "set", "axb", d2.to_str().unwrap()]));

    let out = run_ok(env.cmd().args(["bookmark", "z", "ab"]));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains(&d1.to_string_lossy().replace('\\', "/")));
    assert!(!s.contains(&d2.to_string_lossy().replace('\\', "/")));
}

#[test]
fn z_no_match_prints_message_and_exits_success() {
    let env = TestEnv::new();
    let work = env.root.join("work2");
    fs::create_dir_all(&work).unwrap();
    run_ok(env.cmd().args(["bookmark", "set", "home", work.to_str().unwrap()]));

    let out = run_ok(env.cmd().args(["bookmark", "z", "nope-nope-nope"]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("No matches found"),
        "unexpected stderr: {err}"
    );
}

#[test]
fn rename_to_existing_fails() {
    let env = TestEnv::new();
    let a = env.root.join("a");
    let b = env.root.join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "old", a.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "set", "new", b.to_str().unwrap()]));
    let out = run_ok(env.cmd().args(["bookmark", "rename", "old", "new"]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("already exists"));

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(arr.iter().any(|x| x["name"] == "old"));
    assert!(arr.iter().any(|x| x["name"] == "new"));
}

#[test]
fn touch_increments_visits() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "home", work.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "touch", "home"]));

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "home")
        .unwrap();
    assert_eq!(item["visits"].as_u64().unwrap(), 1);
}

#[test]
fn save_defaults_to_dir_name() {
    let env = TestEnv::new();
    let proj = env.root.join("proj");
    fs::create_dir_all(&proj).unwrap();

    run_ok(env.cmd().current_dir(&proj).args(["bookmark", "save"]));

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "proj")
        .unwrap();
    assert_eq!(
        item["path"].as_str().unwrap(),
        proj.to_string_lossy().replace('\\', "/")
    );
}

#[test]
fn rename_changes_key() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(env.cmd().args(["bookmark", "set", "old", work.to_str().unwrap()]));
    run_ok(env.cmd().args(["bookmark", "rename", "old", "new"]));

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(arr.iter().any(|x| x["name"] == "new"));
    assert!(!arr.iter().any(|x| x["name"] == "old"));
}

#[test]
fn set_relative_path_is_stored_as_absolute() {
    let env = TestEnv::new();
    let base = env.root.join("base");
    let rel = base.join("subdir");
    fs::create_dir_all(&rel).unwrap();

    run_ok(
        env.cmd()
            .current_dir(&base)
            .args(["bookmark", "set", "rel", "subdir"]),
    );

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "rel")
        .unwrap();
    assert_eq!(
        item["path"].as_str().unwrap(),
        rel.to_string_lossy().replace('\\', "/")
    );
}

#[test]
fn set_refuses_to_overwrite_corrupted_bookmark_db() {
    let env = TestEnv::new();
    let db_path = env.root.join(".xun.bookmark.json");
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();
    fs::write(&db_path, "{not-json").unwrap();

    let out = run_err(env.cmd().args(["bookmark", "set", "home", work.to_str().unwrap()]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Failed to load store"));
    assert_eq!(fs::read_to_string(&db_path).unwrap(), "{not-json");
}

#[test]
fn set_succeeds_even_when_binary_cache_write_fails() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    let cache_path = env.root.join(".xun.bookmark.cache");
    fs::create_dir_all(&cache_path).unwrap();

    let out = run_ok(
        env.cmd()
            .args(["bookmark", "set", "home", work.to_str().unwrap()]),
    );
    assert!(out.status.success());

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "home")
        .unwrap();
    assert_eq!(
        item["path"].as_str().unwrap(),
        work.to_string_lossy().replace('\\', "/")
    );
}
