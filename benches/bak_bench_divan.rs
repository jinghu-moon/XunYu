//! backup/restore 模块端到端基准测试
//!
//! 覆盖维度：
//!   1. full_backup_500    - 500 文件全量备份（目录）
//!   2. incremental_50     - 50 文件增量备份（基于 500 文件全量）
//!   3. full_backup_zip    - 500 文件全量备份（zip 压缩）
//!   4. restore_full       - 全量目录还原
//!   5. restore_zip        - zip 全量还原
//!
//! 运行：cargo bench --bench bak_bench_divan

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use divan::{AllocProfiler, Bencher};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// ── 工具函数 ──────────────────────────────────────────────────────────────────

fn xun_bin() -> PathBuf {
    let release = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/xun.exe");
    if release.exists() {
        return release;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/xun.exe")
}

/// 在 tmpdir 下生成 n 个随机大小文件（500-3000 字节），分布在多个子目录
fn populate_files(root: &PathBuf, n: usize) {
    let dirs = ["src/components", "src/utils", "src/hooks", "src/pages", "public"];
    for d in &dirs {
        fs::create_dir_all(root.join(d)).unwrap();
    }
    for i in 0..n {
        let d = dirs[i % dirs.len()];
        let size = 500 + (i * 53) % 2500;
        let content = "x".repeat(size);
        fs::write(root.join(d).join(format!("file_{i:04}.ts")), content).unwrap();
    }
}

const BAK_CFG_NO_COMPRESS: &str = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "bench" },
  "retention": { "maxBackups": 20, "deleteCount": 1 },
  "include": [ "src", "public" ],
  "exclude": []
}"#;

const BAK_CFG_COMPRESS: &str = r#"{
  "storage": { "backupsDir": "A_backups", "compress": true },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "bench" },
  "retention": { "maxBackups": 20, "deleteCount": 1 },
  "include": [ "src", "public" ],
  "exclude": []
}"#;

fn run_backup(root: &PathBuf, extra_args: &[&str]) {
    let status = Command::new(xun_bin())
        .arg("backup")
        .arg("-C")
        .arg(root)
        .arg("-m")
        .arg("bench")
        .args(extra_args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("failed to run xun backup");
    assert!(status.success(), "xun backup failed");
}

fn run_backup_with_env(root: &PathBuf, extra_args: &[&str], envs: &[(&str, &str)]) {
    let mut cmd = Command::new(xun_bin());
    cmd.arg("backup")
        .arg("-C")
        .arg(root)
        .arg("-m")
        .arg("bench")
        .args(extra_args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let status = cmd.status().expect("failed to run xun backup");
    assert!(status.success(), "xun backup failed");
}

fn run_restore(root: &PathBuf, name: &str, extra_args: &[&str]) {
    let status = Command::new(xun_bin())
        .arg("restore")
        .arg(name)
        .arg("-C")
        .arg(root)
        .arg("-y")
        .args(extra_args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("failed to run xun restore");
    assert!(status.success(), "xun restore failed");
}

fn mutate_first_n_files(root: &PathBuf, n: usize) {
    let dirs = ["src/components", "src/utils", "src/hooks", "src/pages", "public"];
    for i in 0..n {
        let dir = dirs[i % dirs.len()];
        fs::write(
            root.join(dir).join(format!("file_{i:04}.ts")),
            format!("modified-{i}-{}", "y".repeat(512)),
        )
        .unwrap();
    }
}

fn find_backup(backups: &PathBuf, prefix: &str, suffix: &str) -> String {
    fs::read_dir(backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| n.starts_with(prefix) && n.ends_with(suffix))
        .unwrap_or_else(|| panic!("backup {prefix}*{suffix} not found in {backups:?}"))
}

// ── 基准 1：全量备份 500 文件（目录）────────────────────────────────────────────

#[divan::bench]
fn full_backup_500_dir(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_bak_bench_full_dir");
    let _ = fs::remove_dir_all(&tmp);
    populate_files(&tmp, 500);
    fs::write(tmp.join(".xun-bak.json"), BAK_CFG_NO_COMPRESS).unwrap();

    bencher.bench(|| {
        // 每次清空备份目录重跑，确保每次都是全量 v1
        let backups = tmp.join("A_backups");
        let _ = fs::remove_dir_all(&backups);
        run_backup(&tmp, &[]);
    });

    let _ = fs::remove_dir_all(&tmp);
}

// ── 基准 2：增量备份（50 文件变更，基于 500 文件全量）────────────────────────────

#[divan::bench]
fn incremental_50_changed(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_bak_bench_incr");
    let _ = fs::remove_dir_all(&tmp);
    populate_files(&tmp, 500);
    fs::write(tmp.join(".xun-bak.json"), BAK_CFG_NO_COMPRESS).unwrap();

    // 先建全量基线 v1
    run_backup(&tmp, &[]);

    // 修改 50 个文件作为增量变更夹具（bench 前准备好，bench 中只跑 bak）
    for i in 0..50usize {
        let d = ["src/components", "src/utils", "src/hooks", "src/pages", "public"][i % 5];
        fs::write(
            tmp.join(d).join(format!("file_{i:04}.ts")),
            format!("modified-{i}"),
        )
        .unwrap();
    }

    bencher.bench(|| {
        // 每次清掉 v2+ 保留 v1 baseline，然后增量备份
        let backups = tmp.join("A_backups");
        // 删除 v2 以上（保留 v1 作为 baseline）
        if let Ok(rd) = fs::read_dir(&backups) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if !n.starts_with("v1-") && !n.ends_with(".meta.json") {
                    let p = e.path();
                    if p.is_dir() { let _ = fs::remove_dir_all(&p); }
                    else { let _ = fs::remove_file(&p); }
                }
            }
        }
        run_backup(&tmp, &["--incremental"]);
    });

    let _ = fs::remove_dir_all(&tmp);
}

// ── 基准 3：全量备份 500 文件（zip 压缩）────────────────────────────────────────

#[divan::bench]
fn full_backup_500_zip(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_bak_bench_full_zip");
    let _ = fs::remove_dir_all(&tmp);
    populate_files(&tmp, 500);
    fs::write(tmp.join(".xun-bak.json"), BAK_CFG_COMPRESS).unwrap();

    bencher.bench(|| {
        let backups = tmp.join("A_backups");
        let _ = fs::remove_dir_all(&backups);
        run_backup(&tmp, &[]);
    });

    let _ = fs::remove_dir_all(&tmp);
}

// ── 基准 3.5：v2 full backup（50 文件变更）默认策略 vs 强制 std ────────────────

#[divan::bench]
fn full_backup_v2_50_changed_default_strategy(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_bak_bench_v2_default");
    let _ = fs::remove_dir_all(&tmp);
    populate_files(&tmp, 500);
    fs::write(tmp.join(".xun-bak.json"), BAK_CFG_NO_COMPRESS).unwrap();
    run_backup(&tmp, &[]);
    mutate_first_n_files(&tmp, 50);

    bencher.bench(|| {
        let backups = tmp.join("A_backups");
        if let Ok(rd) = fs::read_dir(&backups) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if !n.starts_with("v1-") && !n.ends_with(".meta.json") {
                    let p = e.path();
                    if p.is_dir() {
                        let _ = fs::remove_dir_all(&p);
                    } else {
                        let _ = fs::remove_file(&p);
                    }
                }
            }
        }
        run_backup(&tmp, &[]);
    });

    let _ = fs::remove_dir_all(&tmp);
}

#[divan::bench]
fn full_backup_v2_50_changed_force_std(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_bak_bench_v2_std");
    let _ = fs::remove_dir_all(&tmp);
    populate_files(&tmp, 500);
    fs::write(tmp.join(".xun-bak.json"), BAK_CFG_NO_COMPRESS).unwrap();
    run_backup(&tmp, &[]);
    mutate_first_n_files(&tmp, 50);

    bencher.bench(|| {
        let backups = tmp.join("A_backups");
        if let Ok(rd) = fs::read_dir(&backups) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if !n.starts_with("v1-") && !n.ends_with(".meta.json") {
                    let p = e.path();
                    if p.is_dir() {
                        let _ = fs::remove_dir_all(&p);
                    } else {
                        let _ = fs::remove_file(&p);
                    }
                }
            }
        }
        run_backup_with_env(&tmp, &[], &[("XUN_COPY_BACKEND", "std")]);
    });

    let _ = fs::remove_dir_all(&tmp);
}

// ── 基准 4：全量目录还原 ─────────────────────────────────────────────────────────

#[divan::bench]
fn restore_full_dir(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_bak_bench_restore_dir");
    let _ = fs::remove_dir_all(&tmp);
    populate_files(&tmp, 500);
    fs::write(tmp.join(".xun-bak.json"), BAK_CFG_NO_COMPRESS).unwrap();
    run_backup(&tmp, &[]);

    let backups = tmp.join("A_backups");
    let name = find_backup(&backups, "v1-", "");
    // 不能以 .zip 结尾（目录备份）
    assert!(!name.ends_with(".zip"), "expected dir backup");

    bencher.bench(|| {
        run_restore(&tmp, &name, &[]);
    });

    let _ = fs::remove_dir_all(&tmp);
}

// ── 基准 5：zip 还原 ──────────────────────────────────────────────────────────────

#[divan::bench]
fn restore_zip(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_bak_bench_restore_zip");
    let _ = fs::remove_dir_all(&tmp);
    populate_files(&tmp, 500);
    fs::write(tmp.join(".xun-bak.json"), BAK_CFG_COMPRESS).unwrap();
    run_backup(&tmp, &[]);

    let backups = tmp.join("A_backups");
    let zip_name = find_backup(&backups, "v1-", ".zip");
    // restore 命令接受不带 .zip 后缀的名称
    let name = zip_name.trim_end_matches(".zip").to_string();

    bencher.bench(|| {
        run_restore(&tmp, &name, &[]);
    });

    let _ = fs::remove_dir_all(&tmp);
}
