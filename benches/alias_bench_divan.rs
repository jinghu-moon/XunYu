//! alias 模块 Divan 基准测试套件
//!
//! 覆盖维度：
//!   1. classify_mode: shell operator 检测延迟
//!   2. fuzzy_score: 搜索打分吞吐
//!   3. parse_selection: 选择字符串解析延迟
//!   4. shell_alias_to_shim: shim 内容生成吞吐
//!   5. app_alias_to_shim: app shim 内容生成吞吐
//!   6. config_to_sync_entries: 配置转 sync 条目吞吐
//!   7. config_roundtrip: TOML 序列化 + 反序列化往返

#![cfg(windows)]

use std::collections::BTreeMap;
use std::hint::black_box;

use divan::{AllocProfiler, Bencher};

use xun::alias::config::{AliasMode, AppAlias, Config, ShellAlias};
use xun::alias::output::{fuzzy_score, parse_selection};
use xun::alias::shim_gen::{app_alias_to_shim, shell_alias_to_shim};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// ── 测试夹具 ──────────────────────────────────────────────────────────────────

fn make_shell_alias(cmd: &str, mode: AliasMode) -> ShellAlias {
    ShellAlias {
        command: cmd.to_string(),
        desc: Some("bench desc".to_string()),
        tags: vec!["bench".to_string()],
        shells: vec![],
        mode,
    }
}

fn make_app_alias(exe: &str) -> AppAlias {
    AppAlias {
        exe: exe.to_string(),
        args: Some("--flag".to_string()),
        desc: Some("bench app".to_string()),
        tags: vec!["bench".to_string()],
        register_apppaths: false,
    }
}

fn make_config(n: usize) -> Config {
    let mut alias = BTreeMap::new();
    let mut app = BTreeMap::new();
    for i in 0..n {
        alias.insert(
            format!("alias_{i:04}"),
            make_shell_alias(&format!("echo bench_{i}"), AliasMode::Auto),
        );
        app.insert(
            format!("app_{i:04}"),
            make_app_alias(&format!(r"C:\bench\app_{i:04}.exe")),
        );
    }
    Config { alias, app }
}

// ── 1. classify_mode ─────────────────────────────────────────────────────────

#[divan::bench(args = [
    "git status",
    "git log | head -20",
    "echo hello > out.txt",
    r"C:\Windows\System32\notepad.exe",
    "cd /tmp && ls -la",
])]
fn classify_shell_alias(bencher: Bencher, cmd: &str) {
    let alias = make_shell_alias(cmd, AliasMode::Auto);
    bencher.bench(|| {
        black_box(shell_alias_to_shim(black_box(&alias)));
    });
}

#[divan::bench(args = [AliasMode::Auto, AliasMode::Cmd, AliasMode::Exe])]
fn classify_by_mode(bencher: Bencher, mode: AliasMode) {
    let alias = make_shell_alias("git status", mode);
    bencher.bench(|| {
        black_box(shell_alias_to_shim(black_box(&alias)));
    });
}

// ── 2. fuzzy_score ───────────────────────────────────────────────────────────

#[divan::bench(args = [1, 10, 100, 1_000])]
fn fuzzy_score_bench(bencher: Bencher, n: usize) {
    let names: Vec<String> = (0..n).map(|i| format!("alias_{i:04}")).collect();
    let cmds: Vec<String> = (0..n).map(|i| format!("echo bench_{i}")).collect();
    let kw = "bench";
    bencher.bench(|| {
        let mut total = 0i32;
        for i in 0..n {
            total += fuzzy_score(
                black_box(&names[i]),
                black_box(&cmds[i]),
                black_box("bench desc"),
                black_box(kw),
            );
        }
        black_box(total)
    });
}

// ── 3. parse_selection ───────────────────────────────────────────────────────

#[divan::bench(args = [
    "1",
    "1,3,5",
    "1-5",
    "1,3-5,7,9-12",
    "a",
])]
fn parse_selection_bench(bencher: Bencher, input: &str) {
    bencher.bench(|| {
        black_box(parse_selection(black_box(input), black_box(100)));
    });
}

// ── 4. shell_alias_to_shim ───────────────────────────────────────────────────

#[divan::bench]
fn shell_to_shim_cmd_type(bencher: Bencher) {
    let alias = make_shell_alias("git log | head", AliasMode::Auto);
    bencher.bench(|| {
        black_box(shell_alias_to_shim(black_box(&alias)));
    });
}

#[divan::bench]
fn shell_to_shim_exe_type(bencher: Bencher) {
    let alias = make_shell_alias(r"C:\Windows\System32\notepad.exe", AliasMode::Exe);
    bencher.bench(|| {
        black_box(shell_alias_to_shim(black_box(&alias)));
    });
}

// ── 5. app_alias_to_shim ─────────────────────────────────────────────────────

#[divan::bench]
fn app_to_shim_no_args(bencher: Bencher) {
    let app = AppAlias {
        exe: r"C:\Program Files\MyApp\app.exe".to_string(),
        args: None,
        desc: None,
        tags: vec![],
        register_apppaths: false,
    };
    bencher.bench(|| {
        black_box(app_alias_to_shim(black_box(&app)));
    });
}

#[divan::bench]
fn app_to_shim_with_args(bencher: Bencher) {
    let app = make_app_alias(r"C:\Program Files\MyApp\app.exe");
    bencher.bench(|| {
        black_box(app_alias_to_shim(black_box(&app)));
    });
}

// ── 6. config_to_sync_entries ────────────────────────────────────────────────

#[divan::bench(args = [10, 50, 200, 1_000])]
fn config_to_sync_entries_bench(bencher: Bencher, n: usize) {
    let cfg = make_config(n);
    bencher.bench(|| {
        black_box(xun::alias::shim_gen::config_to_sync_entries(black_box(
            &cfg,
        )));
    });
}

// ── 7. TOML 往返 ─────────────────────────────────────────────────────────────

#[divan::bench(args = [10, 50, 200])]
fn config_toml_serialize(bencher: Bencher, n: usize) {
    let cfg = make_config(n);
    bencher.bench(|| {
        black_box(toml::to_string_pretty(black_box(&cfg)).unwrap());
    });
}

#[divan::bench(args = [10, 50, 200])]
fn config_toml_deserialize(bencher: Bencher, n: usize) {
    let cfg = make_config(n);
    let text = toml::to_string_pretty(&cfg).unwrap();
    bencher.bench(|| {
        black_box(toml::from_str::<Config>(black_box(&text)).unwrap());
    });
}
