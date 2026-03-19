//! app alias 命令测试：app add / rm / ls / which / sync / scan

use super::common::*;
use crate::common::*;

// ── app add ───────────────────────────────────────────────────────────────────

#[test]
fn app_add_persists_to_toml() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "myapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "myapp",
        exe.to_str().unwrap(),
        "--desc",
        "My App",
        "--no-apppaths",
    ]));

    let toml = read_toml(&env);
    assert!(toml.contains("[app.myapp]"), "app section missing");
    assert!(toml.contains("My App"), "desc missing");
}

#[test]
fn app_add_rejects_duplicate_without_force() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "dupapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "dupapp",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    let out = run_err(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "dupapp",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));
    let err = stderr_str(&out);
    assert!(
        err.contains("already exists") || err.contains("force"),
        "expected duplicate error: {err}"
    );
}

#[test]
fn app_add_force_overwrites() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe1 = make_fake_exe(&env, "app_v1");
    let exe2 = make_fake_exe(&env, "app_v2");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "myapp",
        exe1.to_str().unwrap(),
        "--no-apppaths",
    ]));
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "myapp",
        exe2.to_str().unwrap(),
        "--no-apppaths",
        "--force",
    ]));

    let toml = read_toml(&env);
    assert!(toml.contains("app_v2"), "force overwrite failed");
    assert!(!toml.contains("app_v1"), "old exe still present");
}

#[test]
fn app_add_force_replaces_existing_shell_alias() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "switcher", "git status"]));

    let exe = make_fake_exe(&env, "switcher_app");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "switcher",
        exe.to_str().unwrap(),
        "--no-apppaths",
        "--force",
    ]));

    let toml = read_toml(&env);
    assert!(toml.contains("[app.switcher]"), "app alias missing after force replace");
    assert!(!toml.contains("[alias.switcher]"), "shell alias should be removed after force replace");
    assert_shim_exists(&env, "switcher");
    assert_file_contains(&cmd_macrofile(&env), "doskey switcher=");
}

#[test]
fn app_add_with_args_persisted() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "argapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "argapp",
        exe.to_str().unwrap(),
        "--args",
        "--verbose",
        "--no-apppaths",
    ]));

    let toml = read_toml(&env);
    assert!(toml.contains("--verbose"), "args not persisted");
}

#[test]
fn app_add_with_tags_persisted() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "tagapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "tagapp",
        exe.to_str().unwrap(),
        "--tag",
        "editor,tools",
        "--no-apppaths",
    ]));

    let toml = read_toml(&env);
    assert!(toml.contains("editor"), "tag missing");
}

// ── app rm ────────────────────────────────────────────────────────────────────

#[test]
fn app_rm_removes_entry_and_shim() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "rmapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "rmapp",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));
    assert_shim_exists(&env, "rmapp");

    run_ok(alias_cmd(&env).args(["alias", "app", "rm", "rmapp"]));

    let toml = read_toml(&env);
    assert!(!toml.contains("[app.rmapp]"), "entry not removed from TOML");
    assert_shim_absent(&env, "rmapp");
}

#[test]
fn app_rm_nonexistent_is_graceful() {
    let env = TestEnv::new();
    do_setup(&env);
    run_ok(alias_cmd(&env).args(["alias", "app", "rm", "ghostapp"]));
}

#[test]
fn app_rm_multiple_names() {
    let env = TestEnv::new();
    do_setup(&env);

    for name in ["app1", "app2", "app3"] {
        let exe = make_fake_exe(&env, name);
        run_ok(alias_cmd(&env).args([
            "alias",
            "app",
            "add",
            name,
            exe.to_str().unwrap(),
            "--no-apppaths",
        ]));
    }
    run_ok(alias_cmd(&env).args(["alias", "app", "rm", "app1", "app2"]));

    let toml = read_toml(&env);
    assert!(!toml.contains("[app.app1]"));
    assert!(!toml.contains("[app.app2]"));
    assert!(toml.contains("[app.app3]"), "app3 should still exist");
}

// ── app ls ────────────────────────────────────────────────────────────────────

#[test]
fn app_ls_shows_added_entry() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "lsapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "lsapp",
        exe.to_str().unwrap(),
        "--desc",
        "List App",
        "--no-apppaths",
    ]));

    let out = run_ok(alias_cmd(&env).args(["alias", "app", "ls"]));
    let combined = combined_str(&out);
    assert!(combined.contains("lsapp"), "app name missing");
    assert!(combined.contains("List App"), "desc missing");
}

#[test]
fn app_ls_json_output_is_valid() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "jsonapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "jsonapp",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    let out = run_ok(alias_cmd(&env).args(["alias", "app", "ls", "--json"]));
    let stdout = stdout_str(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("app ls --json not valid JSON: {e}\n{stdout}"));
    assert!(v.get("jsonapp").is_some(), "app entry missing in JSON");
}

// ── app which ─────────────────────────────────────────────────────────────────

#[test]
fn app_which_shows_exe_and_shim() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "whichapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "whichapp",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    let out = run_ok(alias_cmd(&env).args(["alias", "app", "which", "whichapp"]));
    let combined = combined_str(&out);
    assert!(combined.contains("whichapp"), "name missing");
    assert!(combined.contains(exe.to_str().unwrap()), "exe path missing");
    assert!(
        combined.contains(".shim") || combined.contains("shim"),
        "shim missing"
    );
}

#[test]
fn app_which_nonexistent_fails() {
    let env = TestEnv::new();
    do_setup(&env);

    let out = run_err(alias_cmd(&env).args(["alias", "app", "which", "noapp"]));
    let err = stderr_str(&out);
    assert!(
        err.contains("not found") || err.contains("alias"),
        "expected not-found: {err}"
    );
}

// ── app sync ──────────────────────────────────────────────────────────────────

#[test]
fn app_sync_rebuilds_shims() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "syncapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "syncapp",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    // 手动删除 shim exe
    let shim_exe = shims_dir(&env).join("syncapp.exe");
    std::fs::remove_file(&shim_exe).unwrap();

    run_ok(alias_cmd(&env).args(["alias", "app", "sync"]));
    assert_shim_exists(&env, "syncapp");
}

// ── app scan ──────────────────────────────────────────────────────────────────

#[test]
fn app_scan_json_returns_array() {
    let env = TestEnv::new();
    do_setup(&env);

    // scan --json 不交互，直接输出 JSON
    let out = run_ok(alias_cmd(&env).args(["alias", "app", "scan", "--source", "path", "--json"]));
    let stdout = stdout_str(&out);
    // 应是 JSON 数组
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("app scan --json not valid JSON: {e}\n{stdout}"));
    assert!(v.is_array(), "expected JSON array from app scan");
}

#[test]
fn app_scan_filter_reduces_results() {
    let env = TestEnv::new();
    do_setup(&env);

    // 用极不可能存在的关键词过滤，结果应为空数组
    let out = run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "scan",
        "--source",
        "path",
        "--filter",
        "xyzzy_no_such_app_ever",
        "--json",
    ]));
    let stdout = stdout_str(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("filtered scan not JSON: {e}\n{stdout}"));
    assert_eq!(
        v.as_array().map(|a| a.len()).unwrap_or(usize::MAX),
        0,
        "expected empty result for impossible filter"
    );
}

// ── rm 覆盖 shell alias 和 app alias ─────────────────────────────────────────

#[test]
fn alias_rm_removes_app_alias_too() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "crossrm");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "crossrm",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));
    assert_shim_exists(&env, "crossrm");

    // xun alias rm（非 app rm）也能删除 app 条目
    run_ok(alias_cmd(&env).args(["alias", "rm", "crossrm"]));

    let toml = read_toml(&env);
    assert!(
        !toml.contains("[app.crossrm]"),
        "app entry not removed by alias rm"
    );
    assert_shim_absent(&env, "crossrm");
}
