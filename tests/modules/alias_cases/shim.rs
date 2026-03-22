//! shim 生成与同步测试：classify_mode、.shim 内容、sync_all 幂等性

use super::common::*;
use crate::common::*;

// ── classify_mode via add ─────────────────────────────────────────────────────

#[test]
fn shim_auto_mode_pipe_produces_cmd_type() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "piped", "git log | head"]));

    assert_shim_exists(&env, "piped");
    assert_shim_contains(&env, "piped", "type = cmd");
    assert_shim_contains(&env, "piped", "git log | head");
}

#[test]
fn shim_auto_mode_redirect_produces_cmd_type() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "redir", "echo hello > out.txt"]));

    assert_shim_exists(&env, "redir");
    assert_shim_contains(&env, "redir", "type = cmd");
}

#[test]
fn shim_auto_mode_and_operator_produces_cmd_type() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "andop", "cd /tmp && ls"]));

    assert_shim_contains(&env, "andop", "type = cmd");
}

#[test]
fn shim_mode_cmd_forced_produces_cmd_type() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "notepad", "notepad.exe", "--mode", "cmd"]));

    assert_shim_contains(&env, "notepad", "type = cmd");
    assert_shim_contains(&env, "notepad", "wait = true");
}

#[test]
fn shim_mode_exe_with_absolute_path_produces_exe_type() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "mytool");
    run_ok(alias_cmd(&env).args([
        "alias",
        "add",
        "mytool",
        exe.to_str().unwrap(),
        "--mode",
        "exe",
    ]));

    assert_shim_contains(&env, "mytool", "type = exe");
    assert_shim_contains(&env, "mytool", exe.to_str().unwrap());
}

#[test]
fn shim_auto_mode_absolute_exe_path_detected() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "autotool");
    run_ok(alias_cmd(&env).args(["alias", "add", "autotool", exe.to_str().unwrap()]));

    assert_shim_contains(&env, "autotool", "type = exe");
}

#[test]
fn shim_exe_with_fixed_args_stored() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "tool_args");
    run_ok(alias_cmd(&env).args([
        "alias",
        "add",
        "tool_args",
        &format!("{} --verbose", exe.to_str().unwrap()),
        "--mode",
        "exe",
    ]));

    assert_shim_contains(&env, "tool_args", "args = --verbose");
}

// ── shim 文件生成与删除 ────────────────────────────────────────────────────────

#[test]
fn shim_created_on_add_removed_on_rm() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "rmtest", "git status"]));
    assert_shim_exists(&env, "rmtest");

    run_ok(alias_cmd(&env).args(["alias", "rm", "rmtest"]));
    assert_shim_absent(&env, "rmtest");
}

#[test]
fn shim_sync_removes_orphan_shims() {
    let env = TestEnv::new();
    do_setup(&env);

    // 添加再删除，shim 已被 rm 清除
    run_ok(alias_cmd(&env).args(["alias", "add", "orphan", "git status"]));
    assert_shim_exists(&env, "orphan");
    run_ok(alias_cmd(&env).args(["alias", "rm", "orphan"]));
    assert_shim_absent(&env, "orphan");

    // 手动在 shims 目录放一个游离 .shim 文件
    let shims = shims_dir(&env);
    std::fs::write(shims.join("ghost.shim"), b"type = cmd\ncmd = echo\n").unwrap();
    // 复制一个 exe 占位（不必是真正 shim exe）
    std::fs::copy(r"C:\Windows\System32\cmd.exe", shims.join("ghost.exe")).unwrap();

    // sync 后游离 shim 应被清除
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    assert_shim_absent(&env, "ghost");
}

#[test]
fn shim_sync_is_idempotent() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    run_ok(alias_cmd(&env).args(["alias", "sync"]));

    assert_shim_exists(&env, "gs");
}

#[test]
fn shim_content_updated_on_force_overwrite() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status", "--mode", "cmd"]));
    assert_shim_contains(&env, "gs", "git status");

    run_ok(alias_cmd(&env).args([
        "alias",
        "add",
        "gs",
        "git stash",
        "--mode",
        "cmd",
        "--force",
    ]));
    assert_shim_contains(&env, "gs", "git stash");
}

// ── app alias shim ────────────────────────────────────────────────────────────

#[test]
fn app_shim_type_exe_with_path() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "myapp");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "myapp",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    assert_shim_exists(&env, "myapp");
    assert_shim_contains(&env, "myapp", "type = exe");
    assert_shim_contains(&env, "myapp", exe.to_str().unwrap());
}

#[test]
fn app_shim_with_fixed_args() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "appargs");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "appargs",
        exe.to_str().unwrap(),
        "--args",
        "--flag",
        "--no-apppaths",
    ]));

    assert_shim_contains(&env, "appargs", "args = --flag");
}

#[test]
fn app_shim_removed_on_app_rm() {
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
    assert_shim_absent(&env, "rmapp");
}
