use crate::common::*;
use serde_json::{Value, json};
use std::fs;

#[test]
fn export_import_json_roundtrip() {
    let env1 = TestEnv::new();
    let work = env1.root.join("work");
    fs::create_dir_all(&work).unwrap();
    run_ok(env1.cmd().args([
        "bookmark",
        "set",
        "home",
        work.to_str().unwrap(),
        "--workspace",
        "xunyu",
    ]));

    let export_path = env1.root.join("export.json");
    run_ok(env1.cmd().args([
        "bookmark",
        "export",
        "--format",
        "json",
        "--out",
        export_path.to_str().unwrap(),
    ]));

    let env2 = TestEnv::new();
    run_ok(env2.cmd().args([
        "bookmark",
        "import",
        "--format",
        "json",
        "--input",
        export_path.to_str().unwrap(),
    ]));

    let output = run_ok(env2.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = v.as_array().unwrap();
    let item = arr.iter().find(|x| x["name"] == "home").unwrap();
    assert_eq!(item["workspace"].as_str(), Some("xunyu"));
}

#[test]
fn import_overwrite_requires_yes() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();
    run_ok(
        env.cmd()
            .args(["bookmark", "set", "home", work.to_str().unwrap()]),
    );

    let import_path = env.root.join("import.json");
    let other = env.root.join("other");
    fs::create_dir_all(&other).unwrap();
    let content = json!([{
        "name": "home",
        "path": other.to_str().unwrap(),
        "tags": [],
        "visits": 0,
        "last_visited": 0
    }]);
    fs::write(&import_path, serde_json::to_string(&content).unwrap()).unwrap();

    let out = run_err(env.cmd().args([
        "bookmark",
        "import",
        "--format",
        "json",
        "--input",
        import_path.to_str().unwrap(),
        "--mode",
        "overwrite",
    ]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Overwrite import requires --yes."));

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

#[test]
fn export_tsv_has_fields() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();
    run_ok(env.cmd().args([
        "bookmark",
        "set",
        "home",
        work.to_str().unwrap(),
        "--workspace",
        "xunyu",
    ]));

    let output = run_ok(env.cmd().args(["bookmark", "export", "--format", "tsv"]));
    let line = String::from_utf8_lossy(&output.stdout);
    let first = line.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split('\t').collect();
    assert_eq!(parts.len(), 6);
    assert_eq!(parts[5], "xunyu");
}

#[test]
fn import_tsv_works() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    let tsv_path = env.root.join("import.tsv");
    let line = format!("home\t{}\tdev,cli\t2\t100\txunyu\n", work.to_str().unwrap());
    fs::write(&tsv_path, line).unwrap();

    run_ok(env.cmd().args([
        "bookmark",
        "import",
        "--format",
        "tsv",
        "--input",
        tsv_path.to_str().unwrap(),
    ]));

    let output = run_ok(env.cmd().args(["bookmark", "list", "--format", "json"]));
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let item = v
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "home")
        .unwrap();
    assert_eq!(item["visits"].as_u64().unwrap(), 2);
    assert_eq!(item["workspace"].as_str(), Some("xunyu"));
}

#[test]
fn import_merge_merges_without_overwriting_when_path_is_empty() {
    let env = TestEnv::new();
    let work = env.root.join("work");
    fs::create_dir_all(&work).unwrap();

    run_ok(
        env.cmd()
            .args(["bookmark", "set", "home", work.to_str().unwrap(), "-t", "a"]),
    );
    run_ok(env.cmd().args(["bookmark", "touch", "home"]));
    run_ok(env.cmd().args(["bookmark", "touch", "home"]));

    let import_path = env.root.join("import_merge.json");
    let content = json!([{
        "name": "home",
        "path": "",
        "tags": ["b"],
        "visits": 0,
        "last_visited": 0
    }]);
    fs::write(&import_path, serde_json::to_string(&content).unwrap()).unwrap();

    run_ok(env.cmd().args([
        "bookmark",
        "import",
        "--format",
        "json",
        "--input",
        import_path.to_str().unwrap(),
        "--mode",
        "merge",
    ]));

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
    let tags: Vec<&str> = item["tags"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t.as_str())
        .collect();
    assert!(tags.contains(&"a"));
    assert!(tags.contains(&"b"));
    assert!(item["visits"].as_u64().unwrap_or(0) >= 2);
    assert!(item["last_visited"].as_u64().unwrap_or(0) > 0);
}
