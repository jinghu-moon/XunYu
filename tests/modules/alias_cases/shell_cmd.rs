//! shell alias 命令测试：ls / find / which / sync / export / import

use super::common::*;
use crate::common::*;

// ── ls ────────────────────────────────────────────────────────────────────────

#[test]
fn alias_ls_empty_is_ok() {
    let env = TestEnv::new();
    do_setup(&env);
    run_ok(alias_cmd(&env).args(["alias", "ls"]));
}

#[test]
fn alias_ls_shows_added_entry() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status", "--desc", "git st"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "ls"]));
    let combined = combined_str(&out);
    assert!(combined.contains("gs"), "alias name missing in ls output");
    assert!(
        combined.contains("git status"),
        "command missing in ls output"
    );
}

#[test]
fn alias_ls_type_cmd_filter() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let exe = make_fake_exe(&env, "appfoo");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "appfoo",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    let out = run_ok(alias_cmd(&env).args(["alias", "ls", "--type", "cmd"]));
    let combined = combined_str(&out);
    assert!(combined.contains("gs"));
    // app 条目不应出现
    assert!(
        !combined.contains("appfoo"),
        "app entry leaked into cmd filter"
    );
}

#[test]
fn alias_ls_tag_filter() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gitstat", "git status", "--tag", "git"]));
    run_ok(alias_cmd(&env).args(["alias", "add", "listfiles", "ls -la", "--tag", "shell"]));

    let out = run_ok(alias_cmd(&env).args(["alias", "ls", "--tag", "git"]));
    let stdout = combined_str(&out);
    assert!(stdout.contains("gitstat"), "tagged entry missing");
    assert!(!stdout.contains("listfiles"), "untagged entry leaked");
}

#[test]
fn alias_ls_type_app_filter() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "myshell", "echo hi"]));
    let exe = make_fake_exe(&env, "myapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "myapp",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    let out = run_ok(alias_cmd(&env).args(["alias", "ls", "--type", "app"]));
    let combined = combined_str(&out);
    assert!(combined.contains("myapp"), "app entry missing");
    assert!(
        !combined.contains("myshell"),
        "shell entry leaked into app filter"
    );
}

#[test]
fn alias_ls_json_tag_filter() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status", "--tag", "git"]));
    run_ok(alias_cmd(&env).args(["alias", "add", "ll", "ls -la", "--tag", "shell"]));

    let out = run_ok(alias_cmd(&env).args(["alias", "ls", "--tag", "git", "--json"]));
    let stdout = stdout_str(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    let alias_map = v["alias"].as_object().expect("alias key");
    assert!(alias_map.contains_key("gs"), "gs missing from json");
    assert!(
        !alias_map.contains_key("ll"),
        "ll leaked into json tag filter"
    );
}

#[test]
fn alias_ls_json_output_is_valid() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "ls", "--json"]));
    let stdout = stdout_str(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert!(v.get("alias").is_some(), "alias key missing");
    assert!(v.get("app").is_some(), "app key missing");
}

// ── desc ──────────────────────────────────────────────────────────────────────

#[test]
fn alias_add_desc_stored_and_shown() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args([
        "alias",
        "add",
        "gs",
        "git status",
        "--desc",
        "show git status",
    ]));

    let toml = read_toml(&env);
    assert!(toml.contains("show git status"), "desc not in toml");

    let out = run_ok(alias_cmd(&env).args(["alias", "ls"]));
    let combined = combined_str(&out);
    assert!(combined.contains("show git status"), "desc not shown in ls");
}

// ── find ──────────────────────────────────────────────────────────────────────

#[test]
fn alias_find_exact_name_match() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "find", "gs"]));
    assert!(combined_str(&out).contains("gs"));
}

#[test]
fn alias_find_keyword_in_command() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "find", "git"]));
    assert!(combined_str(&out).contains("gs"));
}

#[test]
fn alias_find_no_match_prints_message() {
    let env = TestEnv::new();
    do_setup(&env);

    let out = run_ok(alias_cmd(&env).args(["alias", "find", "xyzzy_no_such_alias"]));
    let combined = combined_str(&out);
    assert!(
        combined.contains("No alias")
            || combined.contains("no alias")
            || combined.contains("matched"),
        "expected no-match message, got: {combined}"
    );
}

#[test]
fn alias_find_scores_exact_name_higher_than_partial() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    run_ok(alias_cmd(&env).args(["alias", "add", "gst", "git stash"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "find", "gs"]));
    let combined = combined_str(&out);
    // gs 和 gst 都应出现，gs 得分更高排在前面
    assert!(combined.contains("gs"));
}

#[test]
fn alias_find_matches_app_alias() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "vscode");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "code",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    let out = run_ok(alias_cmd(&env).args(["alias", "find", "code"]));
    let combined = combined_str(&out);
    assert!(combined.contains("code"), "app alias not found by find");
}

// ── which ─────────────────────────────────────────────────────────────────────

#[test]
fn alias_which_shows_shim_path() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "which", "gs"]));
    let combined = combined_str(&out);
    assert!(
        combined.contains("gs.exe") || combined.contains("shim"),
        "shim path missing"
    );
}

#[test]
fn alias_which_shows_shim_content() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status", "--mode", "cmd"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "which", "gs"]));
    let combined = combined_str(&out);
    // .shim 内容应包含 type = cmd
    assert!(
        combined.contains("type = cmd"),
        "shim content missing in which"
    );
}

#[test]
fn alias_which_nonexistent_fails() {
    let env = TestEnv::new();
    do_setup(&env);

    let out = alias_cmd(&env)
        .args(["alias", "which", "no_such_alias_xyzzy"])
        .output()
        .unwrap();
    let err = !out.status.success();
    assert!(err, "expected not-found error: {}", combined_str(&out));
}

// ── sync ──────────────────────────────────────────────────────────────────────

#[test]
fn alias_sync_rebuilds_missing_shim() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    assert_shim_exists(&env, "gs");

    // 手动删除 shim
    let shim_exe = shims_dir(&env).join("gs.exe");
    std::fs::remove_file(&shim_exe).unwrap();
    assert!(!shim_exe.exists());

    // sync 后应重建
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    assert_shim_exists(&env, "gs");
}

#[test]
fn alias_sync_multiple_aliases() {
    let env = TestEnv::new();
    do_setup(&env);

    for i in 0..5 {
        run_ok(alias_cmd(&env).args(["alias", "add", &format!("a{i}"), &format!("cmd{i}")]));
    }
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    for i in 0..5 {
        assert_shim_exists(&env, &format!("a{i}"));
    }
}

#[test]
fn alias_sync_mixed_shell_and_app() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let exe = make_fake_exe(&env, "myapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "myapp",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    std::fs::remove_file(shims_dir(&env).join("gs.exe")).unwrap();
    std::fs::remove_file(shims_dir(&env).join("myapp.exe")).unwrap();

    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    assert_shim_exists(&env, "gs");
    assert_shim_exists(&env, "myapp");
}

// ── export / import ───────────────────────────────────────────────────────────

#[test]
fn alias_export_includes_app_aliases() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let exe = make_fake_exe(&env, "myapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "myapp",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    let export_path = env.root.join("export.toml");
    run_ok(alias_cmd(&env).args(["alias", "export", "-o", export_path.to_str().unwrap()]));

    let content = std::fs::read_to_string(&export_path).unwrap();
    assert!(
        content.contains("git status"),
        "shell alias missing from export"
    );
    assert!(content.contains("myapp"), "app alias missing from export");
}

#[test]
fn alias_import_nonexistent_file_fails() {
    let env = TestEnv::new();
    do_setup(&env);

    let out = alias_cmd(&env)
        .args(["alias", "import", "nonexistent_file_xyzzy.toml"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "should fail on missing file");
}
