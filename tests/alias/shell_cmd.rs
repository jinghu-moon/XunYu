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
    assert!(combined.contains("git status"), "command missing in ls output");
}

#[test]
fn alias_ls_type_cmd_filter() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let exe = make_fake_exe(&env, "appfoo");
    run_ok(alias_cmd(&env).args([
        "alias", "app", "add", "appfoo",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    let out = run_ok(alias_cmd(&env).args(["alias", "ls", "--type", "cmd"]));
    let combined = combined_str(&out);
    assert!(combined.contains("gs"));
    // app 条目不应出现
    assert!(!combined.contains("appfoo"), "app entry leaked into cmd filter");
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
fn alias_ls_json_output_is_valid() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "ls", "--json"]));
    let stdout = stdout_str(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("ls --json not valid JSON: {e}\n{stdout}"));
    assert!(v.get("alias").is_some(), "alias key missing in JSON output");
}

// ── find ──────────────────────────────────────────────────────────────────────

#[test]
fn alias_find_exact_name_match() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "find", "gs"]));
    let combined = combined_str(&out);
    assert!(combined.contains("gs"), "exact match missing");
}

#[test]
fn alias_find_keyword_in_command() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gst", "git status --short"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "find", "status"]));
    let combined = combined_str(&out);
    assert!(combined.contains("gst"), "command keyword match missing");
}

#[test]
fn alias_find_no_match_prints_message() {
    let env = TestEnv::new();
    do_setup(&env);

    let out = run_ok(alias_cmd(&env).args(["alias", "find", "zzznomatch"]));
    let combined = combined_str(&out);
    assert!(
        combined.contains("No alias") || combined.contains("matched") || combined.is_empty(),
        "expected no-match message: {combined}"
    );
}

#[test]
fn alias_find_scores_exact_name_higher_than_partial() {
    let env = TestEnv::new();
    do_setup(&env);

    // "gs" 完全匹配关键词，"gstatus" 前缀匹配
    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    run_ok(alias_cmd(&env).args(["alias", "add", "gstatus", "git status --long"]));

    let out = run_ok(alias_cmd(&env).args(["alias", "find", "gs"]));
    let combined = combined_str(&out);
    // 两个都应出现
    assert!(combined.contains("gs"));
    assert!(combined.contains("gstatus"));
    // "gs" 行应在 "gstatus" 行之前（分数更高）
    let gs_pos = combined.find("gs ").unwrap_or(usize::MAX);
    let gst_pos = combined.find("gstatus").unwrap_or(usize::MAX);
    assert!(gs_pos < gst_pos, "exact match should appear before prefix match");
}

// ── which ─────────────────────────────────────────────────────────────────────

#[test]
fn alias_which_shows_shim_path() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let out = run_ok(alias_cmd(&env).args(["alias", "which", "gs"]));
    let combined = combined_str(&out);
    assert!(combined.contains("gs"), "name missing in which output");
    assert!(combined.contains("git status"), "target missing");
    assert!(combined.contains(".shim") || combined.contains("shim"), "shim path missing");
}

#[test]
fn alias_which_nonexistent_fails() {
    let env = TestEnv::new();
    do_setup(&env);

    let out = run_err(alias_cmd(&env).args(["alias", "which", "doesnotexist"]));
    let err = stderr_str(&out);
    assert!(
        err.contains("not found") || err.contains("Alias"),
        "expected not-found error: {err}"
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
    assert!(combined.contains("type = cmd"), "shim content missing in which");
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
        run_ok(alias_cmd(&env).args([
            "alias", "add",
            &format!("a{i}"),
            &format!("cmd{i}"),
        ]));
    }
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    for i in 0..5 {
        assert_shim_exists(&env, &format!("a{i}"));
    }
}
