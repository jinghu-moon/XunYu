#![cfg(windows)]

#[path = "../support/mod.rs"]
mod common;

use common::*;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::thread;
use std::time::Instant;

fn find_projects_dir() -> PathBuf {
    std::env::var("XUN_TEST_FIND_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("D:/100_Projects"))
}

fn find_projects_glob() -> String {
    std::env::var("XUN_TEST_FIND_GLOB").unwrap_or_else(|_| "*.js".to_string())
}

const PERF_BACKUP_CFG_NO_COMPRESS: &str = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "perf" },
  "retention": { "maxBackups": 20, "deleteCount": 1 },
  "include": [ "src", "public" ],
  "exclude": []
}"#;

const PERF_BACKUP_CFG_COMPRESS: &str = r#"{
  "storage": { "backupsDir": "A_backups", "compress": true },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "perf" },
  "retention": { "maxBackups": 20, "deleteCount": 1 },
  "include": [ "src", "public" ],
  "exclude": []
}"#;

fn populate_backup_perf_files(root: &std::path::Path, total: usize) {
    let dirs = ["src/components", "src/utils", "src/hooks", "src/pages", "public"];
    for dir in dirs {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    for i in 0..total {
        let dir = dirs[i % dirs.len()];
        let size = 500 + (i * 53) % 2500;
        fs::write(
            root.join(dir).join(format!("file_{i:04}.ts")),
            "x".repeat(size),
        )
        .unwrap();
    }
}

fn write_backup_perf_config(root: &std::path::Path, compress: bool) {
    let content = if compress {
        PERF_BACKUP_CFG_COMPRESS
    } else {
        PERF_BACKUP_CFG_NO_COMPRESS
    };
    fs::write(root.join(".xun-bak.json"), content).unwrap();
}

fn find_backup_name(
    backups_root: &std::path::Path,
    prefix: &str,
    suffix: Option<&str>,
) -> String {
    fs::read_dir(backups_root)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|name| {
            name.starts_with(prefix) && suffix.is_none_or(|suffix| name.ends_with(suffix))
        })
        .unwrap_or_else(|| {
            panic!(
                "backup {}*{} not found in {}",
                prefix,
                suffix.unwrap_or(""),
                backups_root.display()
            )
        })
}

#[test]
#[ignore]
fn perf_tree_large_directory() {
    let env = TestEnv::new();
    let heavy = HeavyTestGuard::new();
    let root = heavy.info.root.clone();

    let start = Instant::now();
    let mut cmd = env.cmd();
    cmd.args([
        "tree",
        root.to_str().unwrap(),
        "--stats-only",
        "--fast",
        "--plain",
        "--no-clip",
    ]);
    run_ok_status(&mut cmd);
    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_millis();
    eprintln!(
        "perf: tree files={} bytes={} per_dir={} elapsed_ms={}",
        heavy.info.files, heavy.info.bytes, heavy.info.per_dir, elapsed_ms
    );
    assert_under_ms("tree", elapsed, "XUN_TEST_TREE_MAX_MS");
}

#[test]
#[ignore]
fn speed_list_hot() {
    let env = TestEnv::new();
    let base = env.root.join("hot");
    fs::create_dir_all(&base).unwrap();

    let items = env_usize("XUN_TEST_LIST_ITEMS", 200);
    for i in 0..items {
        let dir = base.join(format!("d{}", i));
        fs::create_dir_all(&dir).unwrap();
        run_ok(
            env.cmd()
                .args(["set", format!("k{}", i).as_str(), dir.to_str().unwrap()]),
        );
    }

    let iters = env_usize("XUN_TEST_LIST_ITERS", 20);
    let start = Instant::now();
    for _ in 0..iters {
        run_ok(env.cmd().args(["list"]));
    }
    let elapsed = start.elapsed();
    let avg_ms = (elapsed.as_millis() as u64) / iters.max(1) as u64;
    eprintln!(
        "perf: list items={} iters={} avg_ms={}",
        items, iters, avg_ms
    );
    if let Some(max_avg) = env_u64("XUN_TEST_LIST_AVG_MS") {
        assert!(avg_ms <= max_avg, "list avg {avg_ms}ms > {max_avg}ms");
    }
}

#[test]
#[ignore]
fn resource_handle_count_stable() {
    let env = TestEnv::new();
    let work = env.root.join("res");
    fs::create_dir_all(&work).unwrap();
    run_ok(env.cmd().args(["set", "home", work.to_str().unwrap()]));

    let base = handle_count();
    let iters = env_usize("XUN_TEST_HANDLE_ITERS", 50);
    for _ in 0..iters {
        run_ok(env.cmd().args(["list"]));
    }
    let after = handle_count();
    let delta = after.saturating_sub(base);
    eprintln!(
        "perf: handles base={} after={} delta={}",
        base, after, delta
    );
    if let Some(max_delta) = env_u64("XUN_TEST_HANDLE_DELTA") {
        let delta_u64 = delta as u64;
        assert!(
            delta_u64 <= max_delta,
            "handle delta {delta_u64} > {max_delta}"
        );
    }
}

#[test]
#[ignore]
fn resource_memory_working_set_stable() {
    let env = TestEnv::new();
    let base_dir = env.root.join("mem");
    fs::create_dir_all(&base_dir).unwrap();
    run_ok(env.cmd().args(["set", "home", base_dir.to_str().unwrap()]));

    let base = working_set_bytes();
    let iters = env_usize("XUN_TEST_MEM_ITERS", 50);
    for _ in 0..iters {
        run_ok(env.cmd().args(["list"]));
    }
    let after = working_set_bytes();
    let delta = after.saturating_sub(base);
    eprintln!(
        "perf: working_set base={} after={} delta={}",
        base, after, delta
    );
    if let Some(max_delta) = env_u64("XUN_TEST_WS_DELTA") {
        assert!(
            delta <= max_delta,
            "working set delta {delta} > {max_delta}"
        );
    }
    if let Some(max_abs) = env_u64("XUN_TEST_WS_MAX") {
        assert!(after <= max_abs, "working set {after} > {max_abs}");
    }
}

#[test]
#[ignore]
fn resource_cpu_peak_percent() {
    let env = TestEnv::new();
    let heavy = HeavyTestGuard::new();
    let root = heavy.info.root.clone();

    let sample_ms = env_u64("XUN_TEST_CPU_SAMPLE_MS").unwrap_or(50);
    let mut cmd = env.cmd();
    cmd.args([
        "tree",
        root.to_str().unwrap(),
        "--stats-only",
        "--fast",
        "--plain",
        "--no-clip",
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    let child = cmd.spawn().unwrap();

    let peak = measure_cpu_peak_percent(child, sample_ms);
    eprintln!("perf: cpu_peak_percent={:.2} sample_ms={}", peak, sample_ms);
    if let Some(max_cpu) = env_u64("XUN_TEST_CPU_PEAK_MAX") {
        assert!(peak <= max_cpu as f64, "cpu peak {peak:.2}% > {max_cpu}%");
    }
}

#[test]
#[ignore]
fn perf_find_projects_js_count() {
    let env = TestEnv::new();
    let dir = find_projects_dir();
    let glob = find_projects_glob();
    if !dir.exists() {
        eprintln!("perf: find skip, missing dir={}", dir.display());
        return;
    }

    let start = Instant::now();
    let mut cmd = env.cmd();
    cmd.args(["find", dir.to_str().unwrap(), "--include", &glob, "--count"]);
    run_ok_status(&mut cmd);
    let elapsed = start.elapsed();
    eprintln!(
        "perf: find glob={} dir={} elapsed_ms={}",
        glob,
        dir.display(),
        elapsed.as_millis()
    );
    assert_under_ms("find-glob", elapsed, "XUN_TEST_FIND_JS_MAX_MS");
}

#[test]
#[ignore]
fn perf_find_projects_js_cpu_peak_percent() {
    let env = TestEnv::new();
    let dir = find_projects_dir();
    let glob = find_projects_glob();
    if !dir.exists() {
        eprintln!("perf: find js cpu skip, missing dir={}", dir.display());
        return;
    }

    let sample_ms = env_u64("XUN_TEST_CPU_SAMPLE_MS").unwrap_or(50);
    let mut cmd = env.cmd();
    cmd.args(["find", dir.to_str().unwrap(), "--include", &glob, "--count"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = cmd.spawn().unwrap();

    let peak = measure_cpu_peak_percent(child, sample_ms);
    eprintln!(
        "perf: find glob={} cpu_peak_percent={:.2} sample_ms={}",
        glob, peak, sample_ms
    );
    if let Some(max_cpu) = env_u64("XUN_TEST_FIND_JS_CPU_PEAK_MAX") {
        assert!(peak <= max_cpu as f64, "cpu peak {peak:.2}% > {max_cpu}%");
    }
}

#[test]
#[ignore]
fn perf_find_projects_js_working_set_peak() {
    let env = TestEnv::new();
    let dir = find_projects_dir();
    let glob = find_projects_glob();
    if !dir.exists() {
        eprintln!("perf: find js mem skip, missing dir={}", dir.display());
        return;
    }

    let sample_ms = env_u64("XUN_TEST_FIND_JS_MEM_SAMPLE_MS").unwrap_or(50);
    let mut cmd = env.cmd();
    cmd.args(["find", dir.to_str().unwrap(), "--include", &glob, "--count"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = cmd.spawn().unwrap();

    let peak = measure_working_set_peak_bytes(child, sample_ms);
    eprintln!(
        "perf: find glob={} working_set_peak_bytes={} sample_ms={}",
        glob, peak, sample_ms
    );
    if let Some(max_ws) = env_u64("XUN_TEST_FIND_JS_WS_PEAK_MAX") {
        assert!(peak <= max_ws, "working set peak {peak} > {max_ws}");
    }
}

#[test]
#[ignore]
fn perf_backup_full_500_files() {
    let env = TestEnv::new();
    let root = env.root.join("perf-backup-full");
    let files = env_usize("XUN_TEST_BACKUP_PERF_FILES", 500);
    fs::create_dir_all(&root).unwrap();
    populate_backup_perf_files(&root, files);
    write_backup_perf_config(&root, false);

    let start = Instant::now();
    run_ok_status(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "perf-full"]),
    );
    let elapsed = start.elapsed();
    eprintln!("perf: backup_full files={} elapsed_ms={}", files, elapsed.as_millis());
    assert_under_ms("backup-full", elapsed, "XUN_TEST_BACKUP_FULL_MAX_MS");
}

#[test]
#[ignore]
fn perf_backup_incremental_50_changed_files() {
    let env = TestEnv::new();
    let root = env.root.join("perf-backup-incremental");
    let files = env_usize("XUN_TEST_BACKUP_PERF_FILES", 500);
    let changed = env_usize("XUN_TEST_BACKUP_PERF_CHANGED", 50);
    fs::create_dir_all(&root).unwrap();
    populate_backup_perf_files(&root, files);
    write_backup_perf_config(&root, false);

    run_ok_status(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "baseline"]),
    );

    thread::sleep(std::time::Duration::from_millis(50));
    let dirs = ["src/components", "src/utils", "src/hooks", "src/pages", "public"];
    for i in 0..changed {
        let dir = dirs[i % dirs.len()];
        fs::write(
            root.join(dir).join(format!("file_{i:04}.ts")),
            format!("modified-{i}-{}", "y".repeat(512)),
        )
        .unwrap();
    }

    let start = Instant::now();
    run_ok_status(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "perf-incremental",
        "--incremental",
    ]));
    let elapsed = start.elapsed();
    eprintln!(
        "perf: backup_incremental files={} changed={} elapsed_ms={}",
        files,
        changed,
        elapsed.as_millis()
    );
    assert_under_ms(
        "backup-incremental",
        elapsed,
        "XUN_TEST_BACKUP_INCREMENTAL_MAX_MS",
    );
}

#[test]
#[ignore]
fn perf_restore_dir_500_files() {
    let env = TestEnv::new();
    let root = env.root.join("perf-restore-dir");
    let dest = env.root.join("perf-restore-dir-output");
    let files = env_usize("XUN_TEST_BACKUP_PERF_FILES", 500);
    fs::create_dir_all(&root).unwrap();
    populate_backup_perf_files(&root, files);
    write_backup_perf_config(&root, false);

    run_ok_status(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "restore-dir"]),
    );
    let name = find_backup_name(&root.join("A_backups"), "v1-", None);

    let start = Instant::now();
    run_ok_status(env.cmd().args([
        "restore",
        &name,
        "-C",
        root.to_str().unwrap(),
        "--to",
        dest.to_str().unwrap(),
        "-y",
    ]));
    let elapsed = start.elapsed();
    eprintln!("perf: restore_dir files={} elapsed_ms={}", files, elapsed.as_millis());
    assert_under_ms("restore-dir", elapsed, "XUN_TEST_RESTORE_DIR_MAX_MS");
}

#[test]
#[ignore]
fn perf_restore_zip_500_files() {
    let env = TestEnv::new();
    let root = env.root.join("perf-restore-zip");
    let dest = env.root.join("perf-restore-zip-output");
    let files = env_usize("XUN_TEST_BACKUP_PERF_FILES", 500);
    fs::create_dir_all(&root).unwrap();
    populate_backup_perf_files(&root, files);
    write_backup_perf_config(&root, true);

    run_ok_status(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "restore-zip"]),
    );
    let zip_name = find_backup_name(&root.join("A_backups"), "v1-", Some(".zip"));
    let name = zip_name.trim_end_matches(".zip").to_string();

    let start = Instant::now();
    run_ok_status(env.cmd().args([
        "restore",
        &name,
        "-C",
        root.to_str().unwrap(),
        "--to",
        dest.to_str().unwrap(),
        "-y",
    ]));
    let elapsed = start.elapsed();
    eprintln!("perf: restore_zip files={} elapsed_ms={}", files, elapsed.as_millis());
    assert_under_ms("restore-zip", elapsed, "XUN_TEST_RESTORE_ZIP_MAX_MS");
}

#[cfg(feature = "lock")]
#[test]
fn perf_lock_who_single_file_under_200ms() {
    let env = TestEnv::new();
    let file = env.root.join("perf-lock-who.txt");
    fs::write(&file, "data").unwrap();

    let start = Instant::now();
    let out = run_raw(
        env.cmd()
            .args(["lock", "who", file.to_str().unwrap(), "--format", "json"]),
    );
    if !out.status.success() {
        if is_lock_query_env_unavailable(&out) {
            return;
        }
        panic!(
            "lock who failed: {}\nstderr: {}\nstdout: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
    }

    let elapsed = start.elapsed();
    // `cargo test` runs in debug by default. A strict "200ms" threshold only makes
    // sense in release builds or on fast machines; keep debug robust by using a
    // higher default while still allowing overrides via env.
    let default_max_ms = if cfg!(debug_assertions) { 5_000 } else { 200 };
    let max_ms = env_u64("XUN_TEST_LOCK_WHO_MAX_MS").unwrap_or(default_max_ms);
    let elapsed_ms = elapsed.as_millis() as u64;
    assert!(
        elapsed_ms <= max_ms,
        "lock who elapsed {elapsed_ms}ms > {max_ms}ms"
    );
}

#[cfg(feature = "lock")]
#[test]
fn perf_rm_delete_1k_files_under_5s() {
    let env = TestEnv::new();
    let dir = env.root.join("perf-rm-1k");
    fs::create_dir_all(&dir).unwrap();
    for i in 0..1000usize {
        fs::write(dir.join(format!("f{i:04}.txt")), "x").unwrap();
    }

    let start = Instant::now();
    run_ok(env.cmd().args(["rm", dir.to_str().unwrap()]));
    let elapsed = start.elapsed();
    let max_ms = env_u64("XUN_TEST_RM_1K_MAX_MS").unwrap_or(5_000);
    let elapsed_ms = elapsed.as_millis() as u64;
    assert!(
        elapsed_ms <= max_ms,
        "rm 1k elapsed {elapsed_ms}ms > {max_ms}ms"
    );
    assert!(!dir.exists(), "target directory should be deleted");
}
