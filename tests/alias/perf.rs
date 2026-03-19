//! alias 模块性能测试：覆盖全量操作与后续增量优化目标场景
//!
//! 环境变量控制：
//!   XUN_TEST_ALIAS_COUNT             — 批量测试的 alias 数量（默认 200）
//!   XUN_TEST_ALIAS_DELTA_COUNT       — 增量写路径测试使用的 alias 数量（默认 min(64, COUNT)）
//!   XUN_TEST_ALIAS_ADD_MS            — 单条 add 操作最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_SYNC_MS           — sync 操作最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_LS_MS             — ls 操作最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_FIND_MS           — find 操作最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_RM_MS             — 单条 rm 操作最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_FORCE_MS          — 单条 force overwrite 最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_FILTERED_ADD_MS   — 带 shell filter 的 add 最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_IMPORT_MIXED_MS   — 混合 shell/app import 最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_APP_SYNC_MS       — app sync 最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_SYNC_ORPHAN_MS    — 含 orphan 清理的 sync 最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_IMPORT_DUP_MS     — 重复项 skip import 最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_IMPORT_FORCE_MS   — 重复项 force import 最大允许毫秒数（不设则跳过断言）

use std::time::Instant;

use super::common::*;
use crate::common::*;

fn alias_count() -> usize {
    std::env::var("XUN_TEST_ALIAS_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200)
}

fn delta_alias_count() -> usize {
    std::env::var("XUN_TEST_ALIAS_DELTA_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| alias_count().min(64))
}

fn perf_app_count() -> usize {
    12
}

fn import_alias_count() -> usize {
    delta_alias_count().min(32)
}

fn orphan_count() -> usize {
    delta_alias_count().min(24)
}

/// 批量添加 N 条 shell alias（cmd 类型，避免 exe 解析额外扰动）
fn batch_add_shell_aliases(env: &TestEnv, n: usize) {
    for i in 0..n {
        run_ok(alias_cmd(env).args([
            "alias",
            "add",
            &format!("bench_{i:04}"),
            &format!("echo bench_{i}"),
        ]));
    }
}

/// 批量添加 N 条 app alias（关闭 apppaths，聚焦 alias 自身成本）
fn batch_add_app_aliases(env: &TestEnv, n: usize) {
    let exe = make_fake_exe(env, "bench_app_base");
    let exe_str = exe.to_str().unwrap().to_string();
    for i in 0..n {
        run_ok(alias_cmd(env).args([
            "alias",
            "app",
            "add",
            &format!("mixapp_{i:03}"),
            &exe_str,
            "--no-apppaths",
        ]));
    }
}

fn create_orphan_shims(env: &TestEnv, n: usize) {
    let shims = shims_dir(env);
    std::fs::create_dir_all(&shims).unwrap();
    for i in 0..n {
        let name = format!("ghost_{i:03}");
        std::fs::write(
            shims.join(format!("{name}.shim")),
            b"type = cmd\ncmd = echo ghost\nwait = true\n",
        )
        .unwrap();
        std::fs::copy(
            r"C:\Windows\System32\cmd.exe",
            shims.join(format!("{name}.exe")),
        )
        .unwrap();
    }
}

// ── 单条 add 延迟 ─────────────────────────────────────────────────────────────

#[test]
fn perf_single_add_under_500ms() {
    let env = TestEnv::new();
    do_setup(&env);

    let start = Instant::now();
    run_ok(alias_cmd(&env).args(["alias", "add", "perf_one", "echo perf"]));
    let elapsed = start.elapsed();

    eprintln!("perf: single add = {}ms", elapsed.as_millis());
    assert_under_ms("single add", elapsed, "XUN_TEST_ALIAS_ADD_MS");
}

// ── 批量 add ─────────────────────────────────────────────────────────────────

#[test]
fn perf_batch_add_shell_aliases() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = alias_count();

    let start = Instant::now();
    batch_add_shell_aliases(&env, n);
    let elapsed = start.elapsed();

    eprintln!("perf: batch add {n} aliases = {}ms", elapsed.as_millis());
    let toml = read_toml(&env);
    assert!(
        toml.contains("[alias.bench_0000]"),
        "first alias missing after batch add"
    );
    assert!(
        toml.contains(&format!("[alias.bench_{:04}]", n - 1)),
        "last alias missing after batch add"
    );
}

// ── 单条 rm 延迟 ──────────────────────────────────────────────────────────────

#[test]
fn perf_single_rm_in_large_alias_set() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = delta_alias_count();

    batch_add_shell_aliases(&env, n);

    let start = Instant::now();
    run_ok(alias_cmd(&env).args(["alias", "rm", "bench_0000"]));
    let elapsed = start.elapsed();

    eprintln!("perf: rm in {n} aliases = {}ms", elapsed.as_millis());
    assert_under_ms("single rm", elapsed, "XUN_TEST_ALIAS_RM_MS");

    let toml = read_toml(&env);
    assert!(
        !toml.contains("[alias.bench_0000]"),
        "alias entry still present after rm"
    );
    assert_shim_absent(&env, "bench_0000");
}

// ── force overwrite 延迟 ─────────────────────────────────────────────────────

#[test]
fn perf_force_overwrite_existing_alias() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = delta_alias_count();

    batch_add_shell_aliases(&env, n);

    let start = Instant::now();
    run_ok(alias_cmd(&env).args([
        "alias",
        "add",
        "bench_0000",
        "echo bench_override",
        "--force",
    ]));
    let elapsed = start.elapsed();

    eprintln!(
        "perf: force overwrite in {n} aliases = {}ms",
        elapsed.as_millis()
    );
    assert_under_ms("force overwrite", elapsed, "XUN_TEST_ALIAS_FORCE_MS");

    let toml = read_toml(&env);
    assert!(
        toml.contains("echo bench_override"),
        "overwrite not persisted"
    );
    assert_shim_contains(&env, "bench_0000", "echo bench_override");
}

// ── sync 延迟 ─────────────────────────────────────────────────────────────────

#[test]
fn perf_sync_after_batch_add() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = alias_count();

    batch_add_shell_aliases(&env, n);

    let start = Instant::now();
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    let elapsed = start.elapsed();

    eprintln!("perf: sync {n} aliases = {}ms", elapsed.as_millis());
    assert_under_ms("alias sync", elapsed, "XUN_TEST_ALIAS_SYNC_MS");

    assert_shim_exists(&env, "bench_0000");
    assert_shim_exists(&env, &format!("bench_{:04}", n - 1));
}

// ── ls 延迟 ───────────────────────────────────────────────────────────────────

#[test]
fn perf_ls_after_batch_add() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = alias_count();

    batch_add_shell_aliases(&env, n);

    let start = Instant::now();
    let out = run_ok(alias_cmd(&env).args(["alias", "ls"]));
    let elapsed = start.elapsed();

    eprintln!("perf: ls {n} aliases = {}ms", elapsed.as_millis());
    assert_under_ms("alias ls", elapsed, "XUN_TEST_ALIAS_LS_MS");

    let combined = combined_str(&out);
    assert!(
        combined.contains("bench_0000"),
        "ls output missing first entry"
    );
}

// ── ls --json 延迟 ────────────────────────────────────────────────────────────

#[test]
fn perf_ls_json_after_batch_add() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = alias_count();

    batch_add_shell_aliases(&env, n);

    let start = Instant::now();
    let out = run_ok(alias_cmd(&env).args(["alias", "ls", "--json"]));
    let elapsed = start.elapsed();

    eprintln!("perf: ls --json {n} aliases = {}ms", elapsed.as_millis());

    let stdout = stdout_str(&out);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("ls --json invalid: {e}"));
    let count = v
        .get("alias")
        .and_then(|a| a.as_object())
        .map(|o| o.len())
        .unwrap_or(0);
    assert_eq!(count, n, "ls --json count mismatch: got {count}, want {n}");
}

// ── find 延迟 ─────────────────────────────────────────────────────────────────

#[test]
fn perf_find_in_large_alias_set() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = alias_count();

    batch_add_shell_aliases(&env, n);

    let start = Instant::now();
    let out = run_ok(alias_cmd(&env).args(["alias", "find", "bench"]));
    let elapsed = start.elapsed();

    eprintln!("perf: find in {n} aliases = {}ms", elapsed.as_millis());
    assert_under_ms("alias find", elapsed, "XUN_TEST_ALIAS_FIND_MS");

    let combined = combined_str(&out);
    assert!(combined.contains("bench_0000"), "find missing results");
}

// ── 带 shell filter 的 add 延迟 ───────────────────────────────────────────────

#[test]
fn perf_filtered_add_in_large_alias_set() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = delta_alias_count();

    batch_add_shell_aliases(&env, n);

    let start = Instant::now();
    run_ok(alias_cmd(&env).args([
        "alias",
        "add",
        "bench_ps_only",
        "Get-Process",
        "--shell",
        "ps",
    ]));
    let elapsed = start.elapsed();

    eprintln!(
        "perf: filtered add in {n} aliases = {}ms",
        elapsed.as_millis()
    );
    assert_under_ms("filtered add", elapsed, "XUN_TEST_ALIAS_FILTERED_ADD_MS");

    assert_file_contains(
        &powershell_profile(&env),
        "Set-Alias bench_ps_only Get-Process",
    );
    assert_file_not_contains(&cmd_macrofile(&env), "doskey bench_ps_only=");
}

// ── export / import 往返 ─────────────────────────────────────────────────────

#[test]
fn perf_export_import_roundtrip() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = alias_count();

    batch_add_shell_aliases(&env, n);

    let export_path = env.root.join("bench_export.toml");
    let start = Instant::now();
    run_ok(alias_cmd(&env).args(["alias", "export", "-o", export_path.to_str().unwrap()]));
    let export_elapsed = start.elapsed();
    eprintln!(
        "perf: export {n} aliases = {}ms",
        export_elapsed.as_millis()
    );

    let dst_env = TestEnv::new();
    do_setup(&dst_env);
    let start = Instant::now();
    run_ok(alias_cmd(&dst_env).args(["alias", "import", export_path.to_str().unwrap()]));
    let import_elapsed = start.elapsed();
    eprintln!(
        "perf: import {n} aliases = {}ms",
        import_elapsed.as_millis()
    );

    let toml = read_toml(&dst_env);
    assert!(
        toml.contains("[alias.bench_0000]"),
        "import roundtrip failed"
    );
}

// ── shell/app 混合 import 延迟 ────────────────────────────────────────────────

#[test]
fn perf_import_mixed_alias_and_app_roundtrip() {
    let src_env = TestEnv::new();
    do_setup(&src_env);
    let alias_n = delta_alias_count();
    let app_n = perf_app_count();

    batch_add_shell_aliases(&src_env, alias_n);
    batch_add_app_aliases(&src_env, app_n);

    let export_path = src_env.root.join("mixed_export.toml");
    run_ok(alias_cmd(&src_env).args(["alias", "export", "-o", export_path.to_str().unwrap()]));

    let dst_env = TestEnv::new();
    do_setup(&dst_env);

    let start = Instant::now();
    run_ok(alias_cmd(&dst_env).args(["alias", "import", export_path.to_str().unwrap()]));
    let elapsed = start.elapsed();

    eprintln!(
        "perf: mixed import shell={} app={} = {}ms",
        alias_n,
        app_n,
        elapsed.as_millis()
    );
    assert_under_ms("mixed import", elapsed, "XUN_TEST_ALIAS_IMPORT_MIXED_MS");

    let toml = read_toml(&dst_env);
    assert!(
        toml.contains("[alias.bench_0000]"),
        "shell alias missing after mixed import"
    );
    assert!(
        toml.contains(&format!("[app.mixapp_{:03}]", app_n - 1)),
        "app alias missing after mixed import"
    );
}

// ── app add 批量延迟 ──────────────────────────────────────────────────────────

#[test]
fn perf_batch_app_add() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = 20usize;
    let exe = make_fake_exe(&env, "bench_base");
    let exe_str = exe.to_str().unwrap().to_string();

    let start = Instant::now();
    for i in 0..n {
        run_ok(alias_cmd(&env).args([
            "alias",
            "app",
            "add",
            &format!("bapp_{i:03}"),
            &exe_str,
            "--no-apppaths",
        ]));
    }
    let elapsed = start.elapsed();

    eprintln!("perf: batch app add {n} = {}ms", elapsed.as_millis());

    let toml = read_toml(&env);
    assert!(toml.contains("[app.bapp_000]"), "first app alias missing");
    assert!(
        toml.contains(&format!("[app.bapp_{:03}]", n - 1)),
        "last app alias missing"
    );
}

// ── app sync 延迟 ─────────────────────────────────────────────────────────────

#[test]
fn perf_app_sync_after_batch_app_add() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = perf_app_count();

    batch_add_app_aliases(&env, n);

    let start = Instant::now();
    run_ok(alias_cmd(&env).args(["alias", "app", "sync"]));
    let elapsed = start.elapsed();

    eprintln!("perf: app sync {n} aliases = {}ms", elapsed.as_millis());
    assert_under_ms("app sync", elapsed, "XUN_TEST_ALIAS_APP_SYNC_MS");

    assert_shim_exists(&env, "mixapp_000");
    assert_shim_exists(&env, &format!("mixapp_{:03}", n - 1));
}

// ── 含 orphan 清理的 sync 延迟 ─────────────────────────────────────────────────

#[test]
fn perf_sync_with_orphan_cleanup() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = delta_alias_count();
    let ghosts = orphan_count();

    batch_add_shell_aliases(&env, n);
    create_orphan_shims(&env, ghosts);

    let start = Instant::now();
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    let elapsed = start.elapsed();

    eprintln!(
        "perf: sync with {} aliases and {} orphans = {}ms",
        n,
        ghosts,
        elapsed.as_millis()
    );
    assert_under_ms(
        "sync orphan cleanup",
        elapsed,
        "XUN_TEST_ALIAS_SYNC_ORPHAN_MS",
    );

    for i in 0..ghosts {
        assert_shim_absent(&env, &format!("ghost_{i:03}"));
    }
}

// ── 重复项 import(skip) 延迟 ─────────────────────────────────────────────────

#[test]
fn perf_import_skips_many_duplicates() {
    let src_env = TestEnv::new();
    do_setup(&src_env);
    let n = import_alias_count();
    batch_add_shell_aliases(&src_env, n);

    let export_path = src_env.root.join("dup_skip_export.toml");
    run_ok(alias_cmd(&src_env).args(["alias", "export", "-o", export_path.to_str().unwrap()]));

    let dst_env = TestEnv::new();
    do_setup(&dst_env);
    batch_add_shell_aliases(&dst_env, n);
    run_ok(alias_cmd(&dst_env).args([
        "alias",
        "add",
        "bench_0000",
        "echo local_override",
        "--force",
    ]));

    let start = Instant::now();
    run_ok(alias_cmd(&dst_env).args(["alias", "import", export_path.to_str().unwrap()]));
    let elapsed = start.elapsed();

    eprintln!(
        "perf: import skip duplicates {} aliases = {}ms",
        n,
        elapsed.as_millis()
    );
    assert_under_ms(
        "import skip duplicates",
        elapsed,
        "XUN_TEST_ALIAS_IMPORT_DUP_MS",
    );

    let toml = read_toml(&dst_env);
    assert!(
        toml.contains("echo local_override"),
        "duplicate skip import should keep local override"
    );
}

// ── 重复项 import(force) 延迟 ────────────────────────────────────────────────

#[test]
fn perf_import_force_over_many_duplicates() {
    let src_env = TestEnv::new();
    do_setup(&src_env);
    let n = import_alias_count();
    batch_add_shell_aliases(&src_env, n);

    let export_path = src_env.root.join("dup_force_export.toml");
    run_ok(alias_cmd(&src_env).args(["alias", "export", "-o", export_path.to_str().unwrap()]));

    let dst_env = TestEnv::new();
    do_setup(&dst_env);
    batch_add_shell_aliases(&dst_env, n);
    run_ok(alias_cmd(&dst_env).args([
        "alias",
        "add",
        "bench_0000",
        "echo local_override",
        "--force",
    ]));

    let start = Instant::now();
    run_ok(alias_cmd(&dst_env).args(["alias", "import", export_path.to_str().unwrap(), "--force"]));
    let elapsed = start.elapsed();

    eprintln!(
        "perf: import force duplicates {} aliases = {}ms",
        n,
        elapsed.as_millis()
    );
    assert_under_ms(
        "import force duplicates",
        elapsed,
        "XUN_TEST_ALIAS_IMPORT_FORCE_MS",
    );

    let toml = read_toml(&dst_env);
    assert!(
        toml.contains("echo bench_0"),
        "force import should overwrite local override"
    );
}

// ── sync 幂等性 + 性能 ────────────────────────────────────────────────────────

#[test]
fn perf_sync_idempotent_second_run_faster() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = alias_count();

    batch_add_shell_aliases(&env, n);

    let t1 = Instant::now();
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    let first = t1.elapsed();

    let t2 = Instant::now();
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    let second = t2.elapsed();

    eprintln!(
        "perf: sync idempotent first={}ms second={}ms",
        first.as_millis(),
        second.as_millis()
    );
    assert!(
        second.as_millis() <= first.as_millis() * 2 + 500,
        "second sync unexpectedly slow: first={}ms second={}ms",
        first.as_millis(),
        second.as_millis()
    );
}
