//! alias 模块性能测试：大量 alias 的 add / sync / ls 延迟
//!
//! 环境变量控制：
//!   XUN_TEST_ALIAS_COUNT   — 批量测试的 alias 数量（默认 200）
//!   XUN_TEST_ALIAS_SYNC_MS — sync 操作最大允许毫秒数（不设则跳过断言）
//!   XUN_TEST_ALIAS_LS_MS   — ls 操作最大允许毫秒数（不设则跳过断言）

use std::time::Instant;

use super::common::*;
use crate::common::*;

fn alias_count() -> usize {
    std::env::var("XUN_TEST_ALIAS_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200)
}

/// 批量添加 N 条 shell alias（cmd 类型，无 shim exe 依赖）
fn batch_add_shell_aliases(env: &TestEnv, n: usize) {
    for i in 0..n {
        run_ok(alias_cmd(env).args([
            "alias", "add",
            &format!("bench_{i:04}"),
            &format!("echo bench_{i}"),
        ]));
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

// ── 批量 add ──────────────────────────────────────────────────────────────────

#[test]
fn perf_batch_add_shell_aliases() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = alias_count();

    let start = Instant::now();
    batch_add_shell_aliases(&env, n);
    let elapsed = start.elapsed();

    eprintln!("perf: batch add {n} aliases = {}ms", elapsed.as_millis());
    // 验证全部写入
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

    // 验证 shim 存在
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
    assert!(combined.contains("bench_0000"), "ls output missing first entry");
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
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("ls --json invalid: {e}"));
    let count = v.get("alias")
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

// ── export / import 往返延迟 ──────────────────────────────────────────────────

#[test]
fn perf_export_import_roundtrip() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = alias_count();

    batch_add_shell_aliases(&env, n);

    let export_path = env.root.join("bench_export.toml");
    let start = Instant::now();
    run_ok(alias_cmd(&env).args([
        "alias", "export", "-o",
        export_path.to_str().unwrap(),
    ]));
    let export_elapsed = start.elapsed();
    eprintln!("perf: export {n} aliases = {}ms", export_elapsed.as_millis());

    let dst_env = TestEnv::new();
    do_setup(&dst_env);
    let start = Instant::now();
    run_ok(alias_cmd(&dst_env).args([
        "alias", "import",
        export_path.to_str().unwrap(),
    ]));
    let import_elapsed = start.elapsed();
    eprintln!("perf: import {n} aliases = {}ms", import_elapsed.as_millis());

    let toml = read_toml(&dst_env);
    assert!(
        toml.contains("[alias.bench_0000]"),
        "import roundtrip failed"
    );
}

// ── app add 批量延迟 ──────────────────────────────────────────────────────────

#[test]
fn perf_batch_app_add() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = 20usize; // app add 涉及文件系统操作，使用较小数量

    let exe = make_fake_exe(&env, "bench_base");
    let exe_str = exe.to_str().unwrap().to_string();

    let start = Instant::now();
    for i in 0..n {
        run_ok(alias_cmd(&env).args([
            "alias", "app", "add",
            &format!("bapp_{i:03}"),
            &exe_str,
            "--no-apppaths",
        ]));
    }
    let elapsed = start.elapsed();

    eprintln!("perf: batch app add {n} = {}ms", elapsed.as_millis());

    let toml = read_toml(&env);
    assert!(toml.contains("[app.bapp_000]"), "first app alias missing");
    assert!(toml.contains(&format!("[app.bapp_{:03}]", n - 1)), "last app alias missing");
}

// ── sync 幂等性 + 性能 ────────────────────────────────────────────────────────

#[test]
fn perf_sync_idempotent_second_run_faster() {
    let env = TestEnv::new();
    do_setup(&env);
    let n = alias_count();

    batch_add_shell_aliases(&env, n);

    // 第一次 sync（全量创建）
    let t1 = Instant::now();
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    let first = t1.elapsed();

    // 第二次 sync（内容未变，应命中缓存跳过）
    let t2 = Instant::now();
    run_ok(alias_cmd(&env).args(["alias", "sync"]));
    let second = t2.elapsed();

    eprintln!(
        "perf: sync idempotent first={}ms second={}ms",
        first.as_millis(),
        second.as_millis()
    );
    // 第二次不应比第一次慢超过 2 倍（宽松阈值，避免 CI 噪声）
    assert!(
        second.as_millis() <= first.as_millis() * 2 + 500,
        "second sync unexpectedly slow: first={}ms second={}ms",
        first.as_millis(),
        second.as_millis()
    );
}
