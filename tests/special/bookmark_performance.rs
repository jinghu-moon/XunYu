#![cfg(windows)]

#[path = "../support/mod.rs"]
mod common;

use common::{
    TestEnv, assert_under_ms, env_u64, env_usize, measure_working_set_peak_bytes, run_ok_status,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;
use xun::bookmark_state::Store;

const CACHE_DISABLED_ENV: [(&str, &str); 1] = [("XUN_BM_DISABLE_BINARY_CACHE", "1")];
const CACHE_ENABLED_ENV: [(&str, &str); 1] = [("XUN_BM_ENABLE_BINARY_CACHE", "1")];

#[derive(Clone, Copy)]
struct WorkloadCase {
    label: &'static str,
    args: &'static [&'static str],
}

const REALISTIC_Z_ARGS_API_GATEWAY: &[&str] =
    &["bookmark", "z", "api", "gateway", "--list", "--tsv"];
const REALISTIC_Z_ARGS_DOCS_PORTAL: &[&str] =
    &["bookmark", "z", "docs", "portal", "--list", "--tsv"];
const REALISTIC_Z_ARGS_OPS_TOOLKIT: &[&str] =
    &["bookmark", "z", "ops", "toolkit", "--list", "--tsv"];
const REALISTIC_Z_ARGS_BILLING_SVC: &[&str] =
    &["bookmark", "z", "billing", "svc", "--list", "--tsv"];
const REALISTIC_Z_ARGS_PROXY_AGENT: &[&str] =
    &["bookmark", "z", "proxy", "agent", "--list", "--tsv"];
const REALISTIC_Z_ARGS_CLI_PROFILES: &[&str] =
    &["bookmark", "z", "cli", "profiles", "--list", "--tsv"];

const REALISTIC_COMPLETE_ARGS_API: &[&str] = &["__complete", "bookmark", "z", "api"];
const REALISTIC_COMPLETE_ARGS_DOC: &[&str] = &["__complete", "bookmark", "z", "doc"];
const REALISTIC_COMPLETE_ARGS_TOOL: &[&str] = &["__complete", "bookmark", "z", "tool"];
const REALISTIC_COMPLETE_ARGS_BILL: &[&str] = &["__complete", "bookmark", "z", "bill"];
const REALISTIC_COMPLETE_ARGS_PROX: &[&str] = &["__complete", "bookmark", "z", "prox"];
const REALISTIC_COMPLETE_ARGS_CLI: &[&str] = &["__complete", "bookmark", "z", "cli"];

const REALISTIC_Z_CASES: &[WorkloadCase] = &[
    WorkloadCase {
        label: "api_gateway",
        args: REALISTIC_Z_ARGS_API_GATEWAY,
    },
    WorkloadCase {
        label: "docs_portal",
        args: REALISTIC_Z_ARGS_DOCS_PORTAL,
    },
    WorkloadCase {
        label: "ops_toolkit",
        args: REALISTIC_Z_ARGS_OPS_TOOLKIT,
    },
    WorkloadCase {
        label: "billing_svc",
        args: REALISTIC_Z_ARGS_BILLING_SVC,
    },
    WorkloadCase {
        label: "proxy_agent",
        args: REALISTIC_Z_ARGS_PROXY_AGENT,
    },
    WorkloadCase {
        label: "cli_profiles",
        args: REALISTIC_Z_ARGS_CLI_PROFILES,
    },
];

const REALISTIC_COMPLETE_CASES: &[WorkloadCase] = &[
    WorkloadCase {
        label: "api",
        args: REALISTIC_COMPLETE_ARGS_API,
    },
    WorkloadCase {
        label: "doc",
        args: REALISTIC_COMPLETE_ARGS_DOC,
    },
    WorkloadCase {
        label: "tool",
        args: REALISTIC_COMPLETE_ARGS_TOOL,
    },
    WorkloadCase {
        label: "bill",
        args: REALISTIC_COMPLETE_ARGS_BILL,
    },
    WorkloadCase {
        label: "prox",
        args: REALISTIC_COMPLETE_ARGS_PROX,
    },
    WorkloadCase {
        label: "cli",
        args: REALISTIC_COMPLETE_ARGS_CLI,
    },
];

fn write_large_store(env: &TestEnv, total: usize) {
    let mut bookmarks = Vec::with_capacity(total);
    for i in 0..total {
        let name = format!("client-{i:05}");
        let path = format!("C:/bench/projects/{name}");
        bookmarks.push(json!({
            "id": format!("{i}"),
            "name": name,
            "name_norm": format!("client-{i:05}"),
            "path": path,
            "path_norm": path.to_ascii_lowercase(),
            "source": "Explicit",
            "pinned": i == 0,
            "tags": if i % 2 == 0 { vec!["work"] } else { vec!["bench"] },
            "desc": "",
            "workspace": if i % 3 == 0 { Some("xunyu") } else { None::<&str> },
            "created_at": 1,
            "last_visited": 1_700_000_000u64.saturating_add(i as u64),
            "visit_count": 1u32.saturating_add((i % 50) as u32),
            "frecency_score": 1.0 + (i % 100) as f64
        }));
    }
    let body = json!({
        "schema_version": 1,
        "bookmarks": bookmarks
    });
    fs::write(
        env.root.join(".xun.bookmark.json"),
        serde_json::to_string(&body).unwrap(),
    )
    .unwrap();
}

fn realistic_domain(i: usize) -> (&'static str, &'static str, &'static [&'static str]) {
    match i % 8 {
        0 => ("api", "gateway", &["rust", "backend", "service"]),
        1 => ("docs", "portal", &["docs", "content", "web"]),
        2 => ("ops", "toolkit", &["infra", "tooling", "automation"]),
        3 => ("billing", "svc", &["payments", "backend", "finance"]),
        4 => ("media", "encoder", &["media", "assets", "pipeline"]),
        5 => ("search", "indexer", &["search", "backend", "index"]),
        6 => ("proxy", "agent", &["network", "ops", "edge"]),
        _ => ("shell", "profiles", &["cli", "devx", "profiles"]),
    }
}

fn realistic_team(i: usize) -> &'static str {
    match (i / 8) % 6 {
        0 => "core",
        1 => "platform",
        2 => "devx",
        3 => "infra",
        4 => "web",
        _ => "data",
    }
}

fn realistic_workspace(i: usize) -> Option<&'static str> {
    match (i / 11) % 5 {
        0 => Some("xunyu"),
        1 => Some("platform"),
        2 => Some("docs"),
        3 => Some("ops"),
        _ => None,
    }
}

fn realistic_root_bucket(i: usize) -> &'static str {
    match (i / 17) % 5 {
        0 => "repos",
        1 => "services",
        2 => "tools",
        3 => "docs",
        _ => "workspace",
    }
}

fn realistic_lane(i: usize) -> &'static str {
    match (i / 29) % 4 {
        0 => "apps",
        1 => "libs",
        2 => "ops",
        _ => "sandbox",
    }
}

fn realistic_source(i: usize) -> &'static str {
    match i % 10 {
        7 | 8 => "Imported",
        9 => "Learned",
        _ => "Explicit",
    }
}

fn realistic_name(i: usize) -> String {
    let (domain, artifact, _) = realistic_domain(i);
    let team = realistic_team(i);
    match i % 4 {
        0 => format!("{domain}-{artifact}-{team}-{i:05}"),
        1 => format!("{domain}_{artifact}_{team}_{i:05}"),
        2 => format!("{domain}.{artifact}.{team}.{i:05}"),
        _ => format!("{domain}-{artifact}.v2-{team}-{i:05}"),
    }
}

fn realistic_relative_segments(i: usize, name: &str) -> Vec<String> {
    let (domain, artifact, _) = realistic_domain(i);
    let mut segments = vec![
        realistic_root_bucket(i).to_string(),
        realistic_team(i).to_string(),
    ];
    if let Some(workspace) = realistic_workspace(i) {
        segments.push(workspace.to_string());
    }
    segments.push(domain.to_string());
    segments.push(artifact.to_string());
    segments.push(realistic_lane(i).to_string());
    segments.push(name.to_string());
    segments
}

fn realistic_tags(i: usize) -> Vec<String> {
    let (domain, _, extra) = realistic_domain(i);
    let mut tags = vec![domain.to_string(), realistic_team(i).to_string()];
    tags.extend(extra.iter().map(|tag| (*tag).to_string()));
    if let Some(workspace) = realistic_workspace(i) {
        tags.push(workspace.to_string());
    }
    if i % 9 == 0 {
        tags.push("favorite".to_string());
    }
    if i % 13 == 0 {
        tags.push("daily".to_string());
    }
    tags.sort();
    tags.dedup();
    tags
}

fn write_realistic_store_existing(env: &TestEnv, total: usize) {
    let base = env.root.join("realistic-existing");
    fs::create_dir_all(&base).unwrap();
    let mut bookmarks = Vec::with_capacity(total);
    for i in 0..total {
        let name = realistic_name(i);
        let mut dir = base.clone();
        for segment in realistic_relative_segments(i, &name) {
            dir.push(segment);
        }
        fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().replace('\\', "/");
        let tags = realistic_tags(i);
        let workspace = realistic_workspace(i);
        bookmarks.push(json!({
            "id": format!("real-{i:05}"),
            "name": name,
            "name_norm": realistic_name(i).to_ascii_lowercase(),
            "path": path.clone(),
            "path_norm": path.to_ascii_lowercase(),
            "source": realistic_source(i),
            "pinned": i % 97 == 0,
            "tags": tags,
            "desc": format!("{} {}", realistic_root_bucket(i), realistic_lane(i)),
            "workspace": workspace,
            "created_at": 1_690_000_000u64.saturating_add((i % 1_024) as u64),
            "last_visited": 1_700_000_000u64.saturating_add(((i * 97) % 86_400) as u64),
            "visit_count": 1u32.saturating_add(((i * 17) % 400) as u32),
            "frecency_score": 1.0 + ((i * 13) % 200) as f64 / 10.0
        }));
    }
    let body = json!({
        "schema_version": 1,
        "bookmarks": bookmarks
    });
    fs::write(
        env.root.join(".xun.bookmark.json"),
        serde_json::to_string(&body).unwrap(),
    )
    .unwrap();
}

fn write_large_store_existing(env: &TestEnv, total: usize) {
    let base = env.root.join("existing");
    fs::create_dir_all(&base).unwrap();
    let mut bookmarks = Vec::with_capacity(total);
    for i in 0..total {
        let name = format!("client-{i:05}");
        let dir = base.join(&name);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().replace('\\', "/");
        bookmarks.push(json!({
            "id": format!("{i}"),
            "name": name,
            "name_norm": format!("client-{i:05}"),
            "path": path.clone(),
            "path_norm": path.to_ascii_lowercase(),
            "source": "Explicit",
            "pinned": i == 0,
            "tags": if i % 2 == 0 { vec!["work"] } else { vec!["bench"] },
            "desc": "",
            "workspace": if i % 3 == 0 { Some("xunyu") } else { None::<&str> },
            "created_at": 1,
            "last_visited": 1_700_000_000u64.saturating_add(i as u64),
            "visit_count": 1u32.saturating_add((i % 50) as u32),
            "frecency_score": 1.0 + (i % 100) as f64
        }));
    }
    let body = json!({
        "schema_version": 1,
        "bookmarks": bookmarks
    });
    fs::write(
        env.root.join(".xun.bookmark.json"),
        serde_json::to_string(&body).unwrap(),
    )
    .unwrap();
}

fn compact_store_via_touch(env: &TestEnv, name: &str) {
    run_ok_status(env.cmd().args(["bookmark", "touch", name]));
}

fn warm_binary_cache(env: &TestEnv, keyword: &str) {
    run_ok_status(
        env.cmd()
            .env("XUN_BM_ENABLE_BINARY_CACHE", "1")
            .args(["bookmark", "z", keyword, "--list", "--tsv"]),
    );
}

fn measure_complete_peak_bytes(
    env: &TestEnv,
    total: usize,
    index_min_items: usize,
    warm_index: bool,
) -> u64 {
    write_large_store(env, total);

    if warm_index {
        run_ok_status(
            env.cmd()
                .env("_BM_INDEX_MIN_ITEMS", index_min_items.to_string())
                .args(["__complete", "bookmark", "z", "client"]),
        );
    }

    let sample_ms = env_u64("XUN_TEST_BM_MEM_SAMPLE_MS").unwrap_or(25);
    let mut cmd = env.cmd();
    cmd.env("_BM_INDEX_MIN_ITEMS", index_min_items.to_string())
        .args(["__complete", "bookmark", "z", "client"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    measure_working_set_peak_bytes(cmd.spawn().unwrap(), sample_ms)
}

fn resolve_release_xun() -> Option<PathBuf> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let candidate = PathBuf::from(manifest)
        .join("target")
        .join("release")
        .join(if cfg!(windows) { "xun.exe" } else { "xun" });
    candidate.is_file().then_some(candidate)
}

fn resolve_release_bm() -> Option<PathBuf> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let candidate = PathBuf::from(manifest)
        .join("target")
        .join("release")
        .join(if cfg!(windows) { "bm.exe" } else { "bm" });
    candidate.is_file().then_some(candidate)
}

fn measure_release_case_matrix(
    exe: &Path,
    env: &TestEnv,
    cases: &[WorkloadCase],
    iters: usize,
    extra_env: &[(&str, &str)],
) -> Vec<(&'static str, u64)> {
    cases
        .iter()
        .map(|case| {
            (
                case.label,
                measure_release_avg_ms_with_env(exe, env, case.args, iters, extra_env),
            )
        })
        .collect()
}

fn avg_case_ms(results: &[(&'static str, u64)]) -> u64 {
    if results.is_empty() {
        return 0;
    }
    results.iter().map(|(_, ms)| *ms).sum::<u64>() / results.len() as u64
}

fn max_case_ms(results: &[(&'static str, u64)]) -> u64 {
    results.iter().map(|(_, ms)| *ms).max().unwrap_or(0)
}

fn render_case_summary(results: &[(&'static str, u64)]) -> String {
    results
        .iter()
        .map(|(label, ms)| format!("{label}={ms}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn assert_case_matrix_budget(
    label: &str,
    results: &[(&'static str, u64)],
    avg_key: &str,
    max_key: &str,
) {
    let avg_ms = avg_case_ms(results);
    let max_ms = max_case_ms(results);
    if let Some(limit) = env_u64(avg_key) {
        assert!(avg_ms <= limit, "{label} avg {avg_ms}ms > {limit}ms");
    }
    if let Some(limit) = env_u64(max_key) {
        assert!(max_ms <= limit, "{label} max {max_ms}ms > {limit}ms");
    }
}

fn measure_release_avg_ms(exe: &Path, env: &TestEnv, args: &[&str], iters: usize) -> u64 {
    measure_release_avg_ms_with_env(exe, env, args, iters, &[])
}

fn measure_release_avg_ms_with_env(
    exe: &Path,
    env: &TestEnv,
    args: &[&str],
    iters: usize,
    extra_env: &[(&str, &str)],
) -> u64 {
    let start = Instant::now();
    for _ in 0..iters {
        let mut cmd = std::process::Command::new(exe);
        cmd.env("_BM_DATA_FILE", env.root.join(".xun.bookmark.json"))
            .env("USERPROFILE", &env.root)
            .env("HOME", &env.root)
            .env("XUN_NON_INTERACTIVE", "1")
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        for (key, value) in extra_env {
            cmd.env(key, value);
        }
        let status = cmd.status().expect("run release xun");
        assert!(status.success(), "release command failed: {:?}", args);
    }
    start.elapsed().as_millis() as u64 / iters.max(1) as u64
}

fn measure_store_load_avg_ms(env: &TestEnv, total: usize, iters: usize) -> u64 {
    write_large_store(env, total);
    let path = env.root.join(".xun.bookmark.json");
    let start = Instant::now();
    for _ in 0..iters {
        let store = Store::load(&path).expect("load store");
        std::hint::black_box(store.bookmarks.len());
    }
    start.elapsed().as_millis() as u64 / iters.max(1) as u64
}

fn measure_store_load_avg_ms_existing(
    env: &TestEnv,
    total: usize,
    iters: usize,
    compact_before_measure: bool,
) -> (u64, u64) {
    write_large_store_existing(env, total);
    if compact_before_measure {
        compact_store_via_touch(env, "client-00000");
    }
    let path = env.root.join(".xun.bookmark.json");
    let bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let start = Instant::now();
    for _ in 0..iters {
        let store = Store::load(&path).expect("load store");
        std::hint::black_box(store.bookmarks.len());
    }
    (
        start.elapsed().as_millis() as u64 / iters.max(1) as u64,
        bytes,
    )
}

fn measure_realistic_store_load_avg_ms_existing(
    env: &TestEnv,
    total: usize,
    iters: usize,
) -> (u64, u64) {
    write_realistic_store_existing(env, total);
    let path = env.root.join(".xun.bookmark.json");
    let bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let start = Instant::now();
    for _ in 0..iters {
        let store = Store::load(&path).expect("load realistic store");
        std::hint::black_box(store.bookmarks.len());
    }
    (
        start.elapsed().as_millis() as u64 / iters.max(1) as u64,
        bytes,
    )
}

#[test]
#[ignore]
fn perf_bookmark_z_list_5000() {
    let env = TestEnv::new();
    let total = env_usize("XUN_TEST_BM_ITEMS", 5_000);
    let iters = env_usize("XUN_TEST_BM_Z_ITERS", 20);
    write_large_store(&env, total);

    let start = Instant::now();
    for _ in 0..iters {
        run_ok_status(
            env.cmd()
                .args(["bookmark", "z", "client", "0123", "--list", "--tsv"]),
        );
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() as u64 / iters.max(1) as u64;
    eprintln!(
        "perf: bookmark_z_list items={} iters={} avg_ms={}",
        total, iters, avg_ms
    );
    if let Some(max_avg) = env_u64("XUN_TEST_BM_Z_LIST_AVG_MS") {
        assert!(
            avg_ms <= max_avg,
            "bookmark z --list avg {avg_ms}ms > {max_avg}ms"
        );
    }
}

#[test]
#[ignore]
fn perf_bookmark_complete_5000() {
    let env = TestEnv::new();
    let total = env_usize("XUN_TEST_BM_ITEMS", 5_000);
    let iters = env_usize("XUN_TEST_BM_COMPLETE_ITERS", 20);
    write_large_store(&env, total);

    let start = Instant::now();
    for _ in 0..iters {
        run_ok_status(env.cmd().args(["__complete", "bookmark", "z", "client"]));
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() as u64 / iters.max(1) as u64;
    eprintln!(
        "perf: bookmark_complete items={} iters={} avg_ms={}",
        total, iters, avg_ms
    );
    if let Some(max_avg) = env_u64("XUN_TEST_BM_COMPLETE_AVG_MS") {
        assert!(
            avg_ms <= max_avg,
            "__complete bookmark z avg {avg_ms}ms > {max_avg}ms"
        );
    }
}

#[test]
#[ignore]
fn perf_bookmark_complete_working_set_peak() {
    let env = TestEnv::new();
    let total = env_usize("XUN_TEST_BM_ITEMS", 5_000);
    write_large_store(&env, total);

    let sample_ms = env_u64("XUN_TEST_BM_MEM_SAMPLE_MS").unwrap_or(25);
    let mut cmd = env.cmd();
    cmd.args(["__complete", "bookmark", "z", "client"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let peak = measure_working_set_peak_bytes(cmd.spawn().unwrap(), sample_ms);
    eprintln!(
        "perf: bookmark_complete_ws_peak items={} peak_bytes={} sample_ms={}",
        total, peak, sample_ms
    );
    if let Some(max_ws) = env_u64("XUN_TEST_BM_COMPLETE_WS_MAX") {
        assert!(peak <= max_ws, "working set peak {peak} > {max_ws}");
    }
}

#[test]
#[ignore]
fn perf_bookmark_complete_memory_attribution_matrix() {
    let empty = TestEnv::new();
    let ws_empty = measure_complete_peak_bytes(&empty, 0, 1, false);

    let five_no_index = TestEnv::new();
    let ws_5k_no_index = measure_complete_peak_bytes(&five_no_index, 5_000, 20_000, false);

    let five_force_cold = TestEnv::new();
    let ws_5k_force_cold = measure_complete_peak_bytes(&five_force_cold, 5_000, 1, false);

    let five_force_warm = TestEnv::new();
    let ws_5k_force_warm = measure_complete_peak_bytes(&five_force_warm, 5_000, 1, true);

    let twenty_force_cold = TestEnv::new();
    let ws_20k_force_cold = measure_complete_peak_bytes(&twenty_force_cold, 20_000, 1, false);

    let twenty_force_warm = TestEnv::new();
    let ws_20k_force_warm = measure_complete_peak_bytes(&twenty_force_warm, 20_000, 1, true);

    let mib = |bytes: u64| -> f64 { bytes as f64 / (1024.0 * 1024.0) };
    eprintln!(
        "perf: bookmark_mem_matrix empty_mib={:.2} 5k_no_index_mib={:.2} 5k_no_index_delta_mib={:.2} 5k_force_cold_mib={:.2} 5k_force_cold_delta_mib={:.2} 5k_force_warm_mib={:.2} 5k_force_warm_delta_mib={:.2} 20k_force_cold_mib={:.2} 20k_force_cold_delta_mib={:.2} 20k_force_warm_mib={:.2} 20k_force_warm_delta_mib={:.2}",
        mib(ws_empty),
        mib(ws_5k_no_index),
        mib(ws_5k_no_index.saturating_sub(ws_empty)),
        mib(ws_5k_force_cold),
        mib(ws_5k_force_cold.saturating_sub(ws_empty)),
        mib(ws_5k_force_warm),
        mib(ws_5k_force_warm.saturating_sub(ws_empty)),
        mib(ws_20k_force_cold),
        mib(ws_20k_force_cold.saturating_sub(ws_empty)),
        mib(ws_20k_force_warm),
        mib(ws_20k_force_warm.saturating_sub(ws_empty)),
    );
}

#[test]
#[ignore]
fn perf_bookmark_z_list_elapsed_budget() {
    let env = TestEnv::new();
    let total = env_usize("XUN_TEST_BM_ITEMS", 5_000);
    write_large_store(&env, total);

    let start = Instant::now();
    run_ok_status(
        env.cmd()
            .args(["bookmark", "z", "client", "0123", "--list", "--tsv"]),
    );
    let elapsed = start.elapsed();
    eprintln!(
        "perf: bookmark_z_single items={} elapsed_ms={}",
        total,
        elapsed.as_millis()
    );
    assert_under_ms("bookmark-z-list", elapsed, "XUN_TEST_BM_Z_SINGLE_MAX_MS");
}

#[test]
#[ignore]
fn perf_bookmark_release_end_to_end() {
    let Some(exe) = resolve_release_xun() else {
        eprintln!("perf: release xun.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let total = env_usize("XUN_TEST_BM_ITEMS", 5_000);
    let iters = env_usize("XUN_TEST_BM_RELEASE_ITERS", 20);
    write_large_store(&env, total);

    let z_avg = measure_release_avg_ms(
        &exe,
        &env,
        &["bookmark", "z", "client", "0123", "--list", "--tsv"],
        iters,
    );
    let complete_avg = measure_release_avg_ms(
        &exe,
        &env,
        &["__complete", "bookmark", "z", "client"],
        iters,
    );

    eprintln!(
        "perf: bookmark_release items={} iters={} z_avg_ms={} complete_avg_ms={}",
        total, iters, z_avg, complete_avg
    );

    if let Some(max_avg) = env_u64("XUN_TEST_BM_RELEASE_Z_AVG_MS") {
        assert!(
            z_avg <= max_avg,
            "release bookmark z avg {z_avg}ms > {max_avg}ms"
        );
    }
    if let Some(max_avg) = env_u64("XUN_TEST_BM_RELEASE_COMPLETE_AVG_MS") {
        assert!(
            complete_avg <= max_avg,
            "release bookmark complete avg {complete_avg}ms > {max_avg}ms"
        );
    }
}

#[test]
#[ignore]
fn perf_bookmark_release_realistic_mix() {
    let Some(exe) = resolve_release_xun() else {
        eprintln!("perf: release xun.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let total = env_usize("XUN_TEST_BM_REALISTIC_ITEMS", 10_000);
    let iters = env_usize("XUN_TEST_BM_REALISTIC_ITERS", 10);
    write_realistic_store_existing(&env, total);

    let z_results =
        measure_release_case_matrix(&exe, &env, REALISTIC_Z_CASES, iters, &CACHE_DISABLED_ENV);
    let complete_results = measure_release_case_matrix(
        &exe,
        &env,
        REALISTIC_COMPLETE_CASES,
        iters,
        &CACHE_DISABLED_ENV,
    );

    eprintln!(
        "perf: bookmark_realistic_release items={} iters={} z_avg_ms={} z_max_ms={} complete_avg_ms={} complete_max_ms={} z_cases=[{}] complete_cases=[{}]",
        total,
        iters,
        avg_case_ms(&z_results),
        max_case_ms(&z_results),
        avg_case_ms(&complete_results),
        max_case_ms(&complete_results),
        render_case_summary(&z_results),
        render_case_summary(&complete_results),
    );

    assert_case_matrix_budget(
        "bookmark-realistic-z",
        &z_results,
        "XUN_TEST_BM_REALISTIC_RELEASE_Z_AVG_MS",
        "XUN_TEST_BM_REALISTIC_RELEASE_Z_MAX_MS",
    );
    assert_case_matrix_budget(
        "bookmark-realistic-complete",
        &complete_results,
        "XUN_TEST_BM_REALISTIC_RELEASE_COMPLETE_AVG_MS",
        "XUN_TEST_BM_REALISTIC_RELEASE_COMPLETE_MAX_MS",
    );
}

#[test]
#[ignore]
fn perf_bookmark_release_realistic_mix_binary_cache() {
    let Some(exe) = resolve_release_xun() else {
        eprintln!("perf: release xun.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let total = env_usize("XUN_TEST_BM_REALISTIC_ITEMS", 10_000);
    let iters = env_usize("XUN_TEST_BM_REALISTIC_ITERS", 10);
    write_realistic_store_existing(&env, total);
    let warm_name = realistic_name(0);
    compact_store_via_touch(&env, &warm_name);
    warm_binary_cache(&env, "api");

    let z_results =
        measure_release_case_matrix(&exe, &env, REALISTIC_Z_CASES, iters, &CACHE_ENABLED_ENV);
    let complete_results = measure_release_case_matrix(
        &exe,
        &env,
        REALISTIC_COMPLETE_CASES,
        iters,
        &CACHE_ENABLED_ENV,
    );

    eprintln!(
        "perf: bookmark_realistic_release_binary_cache items={} iters={} z_avg_ms={} z_max_ms={} complete_avg_ms={} complete_max_ms={} z_cases=[{}] complete_cases=[{}]",
        total,
        iters,
        avg_case_ms(&z_results),
        max_case_ms(&z_results),
        avg_case_ms(&complete_results),
        max_case_ms(&complete_results),
        render_case_summary(&z_results),
        render_case_summary(&complete_results),
    );

    assert_case_matrix_budget(
        "bookmark-realistic-z-binary-cache",
        &z_results,
        "XUN_TEST_BM_REALISTIC_CACHE_RELEASE_Z_AVG_MS",
        "XUN_TEST_BM_REALISTIC_CACHE_RELEASE_Z_MAX_MS",
    );
    assert_case_matrix_budget(
        "bookmark-realistic-complete-binary-cache",
        &complete_results,
        "XUN_TEST_BM_REALISTIC_CACHE_RELEASE_COMPLETE_AVG_MS",
        "XUN_TEST_BM_REALISTIC_CACHE_RELEASE_COMPLETE_MAX_MS",
    );
}

#[test]
#[ignore]
fn perf_bm_release_end_to_end() {
    let Some(exe) = resolve_release_bm() else {
        eprintln!("perf: release bm.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let total = env_usize("XUN_TEST_BM_ITEMS", 5_000);
    let iters = env_usize("XUN_TEST_BM_RELEASE_ITERS", 20);
    write_large_store(&env, total);

    let z_avg = measure_release_avg_ms(
        &exe,
        &env,
        &["z", "client", "0123", "--list", "--tsv"],
        iters,
    );

    eprintln!(
        "perf: bm_release items={} iters={} z_avg_ms={}",
        total, iters, z_avg
    );

    if let Some(max_avg) = env_u64("XUN_TEST_BM_LITE_RELEASE_Z_AVG_MS") {
        assert!(z_avg <= max_avg, "release bm z avg {z_avg}ms > {max_avg}ms");
    }
}

#[test]
#[ignore]
fn perf_bookmark_release_compare_matrix() {
    let Some(xun) = resolve_release_xun() else {
        eprintln!("perf: release xun.exe missing, run `cargo build --release` first");
        return;
    };
    let Some(bm) = resolve_release_bm() else {
        eprintln!("perf: release bm.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let total = env_usize("XUN_TEST_BM_ITEMS", 5_000);
    let iters = env_usize("XUN_TEST_BM_RELEASE_ITERS", 20);
    write_large_store_existing(&env, total);

    let xun_z = measure_release_avg_ms(
        &xun,
        &env,
        &["bookmark", "z", "client", "0123", "--list", "--tsv"],
        iters,
    );
    let bm_z = measure_release_avg_ms(
        &bm,
        &env,
        &["z", "client", "0123", "--list", "--tsv"],
        iters,
    );
    let xun_zi = measure_release_avg_ms(&xun, &env, &["bookmark", "zi", "client", "0123"], iters);
    let bm_zi = measure_release_avg_ms(&bm, &env, &["zi", "client", "0123"], iters);
    let xun_complete = measure_release_avg_ms(
        &xun,
        &env,
        &["__complete", "bookmark", "z", "client"],
        iters,
    );
    let bm_complete_backend =
        measure_release_avg_ms(&bm, &env, &["z", "client", "--list", "--tsv"], iters);

    eprintln!(
        "perf: bookmark_compare_matrix items={} iters={} xun_z_ms={} bm_z_ms={} xun_zi_ms={} bm_zi_ms={} xun_complete_ms={} bm_complete_backend_ms={}",
        total, iters, xun_z, bm_z, xun_zi, bm_zi, xun_complete, bm_complete_backend
    );
}

#[test]
#[ignore]
fn perf_bookmark_store_load_20000() {
    let env = TestEnv::new();
    let iters = env_usize("XUN_TEST_BM_STORE_LOAD_ITERS", 10);
    let avg_ms = measure_store_load_avg_ms(&env, 20_000, iters);
    eprintln!(
        "perf: bookmark_store_load items={} iters={} avg_ms={}",
        20_000, iters, avg_ms
    );
}

#[test]
#[ignore]
fn perf_bookmark_store_load_50000() {
    let env = TestEnv::new();
    let iters = env_usize("XUN_TEST_BM_STORE_LOAD_ITERS", 5);
    let avg_ms = measure_store_load_avg_ms(&env, 50_000, iters);
    eprintln!(
        "perf: bookmark_store_load items={} iters={} avg_ms={}",
        50_000, iters, avg_ms
    );
}

#[test]
#[ignore]
fn perf_bookmark_store_load_20000_compact() {
    let env = TestEnv::new();
    let iters = env_usize("XUN_TEST_BM_STORE_LOAD_ITERS", 10);
    let (avg_ms, bytes) = measure_store_load_avg_ms_existing(&env, 20_000, iters, true);
    eprintln!(
        "perf: bookmark_store_load_compact items={} iters={} avg_ms={} bytes={}",
        20_000, iters, avg_ms, bytes
    );
}

#[test]
#[ignore]
fn perf_bookmark_store_load_realistic_20000() {
    let env = TestEnv::new();
    let total = env_usize("XUN_TEST_BM_REALISTIC_STORE_LOAD_ITEMS", 20_000);
    let iters = env_usize("XUN_TEST_BM_STORE_LOAD_ITERS", 10);
    let (avg_ms, bytes) = measure_realistic_store_load_avg_ms_existing(&env, total, iters);
    eprintln!(
        "perf: bookmark_store_load_realistic items={} iters={} avg_ms={} bytes={}",
        total, iters, avg_ms, bytes
    );
    if let Some(limit) = env_u64("XUN_TEST_BM_STORE_LOAD_REALISTIC_AVG_MS") {
        assert!(
            avg_ms <= limit,
            "realistic bookmark store load avg {avg_ms}ms > {limit}ms"
        );
    }
}

#[test]
#[ignore]
fn perf_bookmark_store_load_50000_compact() {
    let env = TestEnv::new();
    let iters = env_usize("XUN_TEST_BM_STORE_LOAD_ITERS", 5);
    let (avg_ms, bytes) = measure_store_load_avg_ms_existing(&env, 50_000, iters, true);
    eprintln!(
        "perf: bookmark_store_load_compact items={} iters={} avg_ms={} bytes={}",
        50_000, iters, avg_ms, bytes
    );
}

#[test]
#[ignore]
fn perf_bookmark_release_compare_matrix_20000() {
    let Some(xun) = resolve_release_xun() else {
        eprintln!("perf: release xun.exe missing, run `cargo build --release` first");
        return;
    };
    let Some(bm) = resolve_release_bm() else {
        eprintln!("perf: release bm.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let iters = env_usize("XUN_TEST_BM_RELEASE_ITERS_LARGE", 10);
    write_large_store_existing(&env, 20_000);

    let xun_z = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["bookmark", "z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    let bm_z = measure_release_avg_ms_with_env(
        &bm,
        &env,
        &["z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    let xun_complete = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["__complete", "bookmark", "z", "client"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    eprintln!(
        "perf: bookmark_compare_matrix_20000 iters={} xun_z_ms={} bm_z_ms={} xun_complete_ms={}",
        iters, xun_z, bm_z, xun_complete
    );
}

#[test]
#[ignore]
fn perf_bookmark_release_compare_matrix_20000_compact() {
    let Some(xun) = resolve_release_xun() else {
        eprintln!("perf: release xun.exe missing, run `cargo build --release` first");
        return;
    };
    let Some(bm) = resolve_release_bm() else {
        eprintln!("perf: release bm.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let iters = env_usize("XUN_TEST_BM_RELEASE_ITERS_LARGE", 10);
    write_large_store_existing(&env, 20_000);
    compact_store_via_touch(&env, "client-00000");

    let bytes = fs::metadata(env.root.join(".xun.bookmark.json"))
        .map(|m| m.len())
        .unwrap_or(0);

    let xun_z = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["bookmark", "z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    let bm_z = measure_release_avg_ms_with_env(
        &bm,
        &env,
        &["z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    let xun_complete = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["__complete", "bookmark", "z", "client"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    eprintln!(
        "perf: bookmark_compare_matrix_20000_compact iters={} bytes={} xun_z_ms={} bm_z_ms={} xun_complete_ms={}",
        iters, bytes, xun_z, bm_z, xun_complete
    );
}

#[test]
#[ignore]
fn perf_bookmark_release_compare_matrix_20000_binary_cache() {
    let Some(xun) = resolve_release_xun() else {
        eprintln!("perf: release xun.exe missing, run `cargo build --release` first");
        return;
    };
    let Some(bm) = resolve_release_bm() else {
        eprintln!("perf: release bm.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let iters = env_usize("XUN_TEST_BM_RELEASE_ITERS_LARGE", 10);
    write_large_store_existing(&env, 20_000);
    compact_store_via_touch(&env, "client-00000");
    warm_binary_cache(&env, "client");

    let bytes = fs::metadata(env.root.join(".xun.bookmark.json"))
        .map(|m| m.len())
        .unwrap_or(0);

    let xun_z = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["bookmark", "z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_ENABLED_ENV,
    );
    let bm_z = measure_release_avg_ms_with_env(
        &bm,
        &env,
        &["z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_ENABLED_ENV,
    );
    let xun_complete = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["__complete", "bookmark", "z", "client"],
        iters,
        &CACHE_ENABLED_ENV,
    );
    eprintln!(
        "perf: bookmark_compare_matrix_20000_binary_cache iters={} bytes={} xun_z_ms={} bm_z_ms={} xun_complete_ms={}",
        iters, bytes, xun_z, bm_z, xun_complete
    );
}

#[test]
#[ignore]
fn perf_bookmark_release_compare_matrix_50000() {
    let Some(xun) = resolve_release_xun() else {
        eprintln!("perf: release xun.exe missing, run `cargo build --release` first");
        return;
    };
    let Some(bm) = resolve_release_bm() else {
        eprintln!("perf: release bm.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let iters = env_usize("XUN_TEST_BM_RELEASE_ITERS_HUGE", 2);
    write_large_store_existing(&env, 50_000);

    let xun_z = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["bookmark", "z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    let bm_z = measure_release_avg_ms_with_env(
        &bm,
        &env,
        &["z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    let xun_complete = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["__complete", "bookmark", "z", "client"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    eprintln!(
        "perf: bookmark_compare_matrix_50000 iters={} xun_z_ms={} bm_z_ms={} xun_complete_ms={}",
        iters, xun_z, bm_z, xun_complete
    );
}

#[test]
#[ignore]
fn perf_bookmark_release_compare_matrix_50000_compact() {
    let Some(xun) = resolve_release_xun() else {
        eprintln!("perf: release xun.exe missing, run `cargo build --release` first");
        return;
    };
    let Some(bm) = resolve_release_bm() else {
        eprintln!("perf: release bm.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let iters = env_usize("XUN_TEST_BM_RELEASE_ITERS_HUGE", 2);
    write_large_store_existing(&env, 50_000);
    compact_store_via_touch(&env, "client-00000");

    let bytes = fs::metadata(env.root.join(".xun.bookmark.json"))
        .map(|m| m.len())
        .unwrap_or(0);

    let xun_z = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["bookmark", "z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    let bm_z = measure_release_avg_ms_with_env(
        &bm,
        &env,
        &["z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    let xun_complete = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["__complete", "bookmark", "z", "client"],
        iters,
        &CACHE_DISABLED_ENV,
    );
    eprintln!(
        "perf: bookmark_compare_matrix_50000_compact iters={} bytes={} xun_z_ms={} bm_z_ms={} xun_complete_ms={}",
        iters, bytes, xun_z, bm_z, xun_complete
    );
}

#[test]
#[ignore]
fn perf_bookmark_release_compare_matrix_50000_binary_cache() {
    let Some(xun) = resolve_release_xun() else {
        eprintln!("perf: release xun.exe missing, run `cargo build --release` first");
        return;
    };
    let Some(bm) = resolve_release_bm() else {
        eprintln!("perf: release bm.exe missing, run `cargo build --release` first");
        return;
    };

    let env = TestEnv::new();
    let iters = env_usize("XUN_TEST_BM_RELEASE_ITERS_HUGE", 2);
    write_large_store_existing(&env, 50_000);
    compact_store_via_touch(&env, "client-00000");
    warm_binary_cache(&env, "client");

    let bytes = fs::metadata(env.root.join(".xun.bookmark.json"))
        .map(|m| m.len())
        .unwrap_or(0);

    let xun_z = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["bookmark", "z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_ENABLED_ENV,
    );
    let bm_z = measure_release_avg_ms_with_env(
        &bm,
        &env,
        &["z", "client", "0123", "--list", "--tsv"],
        iters,
        &CACHE_ENABLED_ENV,
    );
    let xun_complete = measure_release_avg_ms_with_env(
        &xun,
        &env,
        &["__complete", "bookmark", "z", "client"],
        iters,
        &CACHE_ENABLED_ENV,
    );
    eprintln!(
        "perf: bookmark_compare_matrix_50000_binary_cache iters={} bytes={} xun_z_ms={} bm_z_ms={} xun_complete_ms={}",
        iters, bytes, xun_z, bm_z, xun_complete
    );
}
