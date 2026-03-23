//! backup export/create/convert/restore 基准测试
//!
//! 运行：
//!   cargo bench --bench backup_export_bench_divan --features xunbak

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use divan::{AllocProfiler, Bencher};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

fn xun_bin() -> PathBuf {
    let release = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/xun.exe");
    if release.exists() {
        return release;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/xun.exe")
}

fn populate_files(root: &PathBuf, n: usize) {
    let dirs = ["src", "docs", "assets", "deep/nested", "path with spaces"];
    for d in &dirs {
        fs::create_dir_all(root.join(d)).unwrap();
    }
    for i in 0..n {
        let d = dirs[i % dirs.len()];
        let size = 1024 + (i * 37) % 8192;
        let content = "x".repeat(size);
        fs::write(root.join(d).join(format!("file_{i:04}.txt")), content).unwrap();
    }
    fs::write(root.join("empty.txt"), "").unwrap();
}

const CFG: &str = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "bench" },
  "retention": { "maxBackups": 20, "deleteCount": 1 },
  "include": [ "src", "docs", "assets", "deep", "path with spaces", "empty.txt" ],
  "exclude": []
}"#;

fn run_xun(root: &PathBuf, args: &[&str]) {
    let status = Command::new(xun_bin())
        .args(args)
        .env("XUN_DB", root.join(".xun.json"))
        .env("USERPROFILE", root)
        .env("HOME", root)
        .env("XUN_NON_INTERACTIVE", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("failed to run xun");
    assert!(status.success(), "xun command failed: {args:?}");
}

#[divan::bench]
fn create_zip_200_files(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_export_bench_create_zip");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    populate_files(&tmp, 200);
    fs::write(tmp.join(".xun-bak.json"), CFG).unwrap();

    bencher.bench(|| {
        let _ = fs::remove_file(tmp.join("artifact.zip"));
        run_xun(
            &tmp,
            &[
                "backup",
                "create",
                "-C",
                tmp.to_str().unwrap(),
                "--format",
                "zip",
                "-o",
                "artifact.zip",
            ],
        );
    });

    let _ = fs::remove_dir_all(&tmp);
}

#[divan::bench]
fn create_7z_200_files(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_export_bench_create_7z");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    populate_files(&tmp, 200);
    fs::write(tmp.join(".xun-bak.json"), CFG).unwrap();

    bencher.bench(|| {
        let _ = fs::remove_file(tmp.join("artifact.7z"));
        run_xun(
            &tmp,
            &[
                "backup",
                "create",
                "-C",
                tmp.to_str().unwrap(),
                "--format",
                "7z",
                "-o",
                "artifact.7z",
            ],
        );
    });

    let _ = fs::remove_dir_all(&tmp);
}

#[divan::bench]
fn convert_xunbak_to_zip_200_files(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_export_bench_convert_zip");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    populate_files(&tmp, 200);
    fs::write(tmp.join(".xun-bak.json"), CFG).unwrap();
    run_xun(
        &tmp,
        &[
            "backup",
            "create",
            "-C",
            tmp.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            "artifact.xunbak",
        ],
    );

    bencher.bench(|| {
        let _ = fs::remove_file(tmp.join("converted.zip"));
        run_xun(
            &tmp,
            &[
                "backup",
                "convert",
                tmp.join("artifact.xunbak").to_str().unwrap(),
                "--format",
                "zip",
                "-o",
                tmp.join("converted.zip").to_str().unwrap(),
            ],
        );
    });

    let _ = fs::remove_dir_all(&tmp);
}

#[divan::bench]
fn convert_xunbak_to_7z_200_files(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_export_bench_convert_7z");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    populate_files(&tmp, 200);
    fs::write(tmp.join(".xun-bak.json"), CFG).unwrap();
    run_xun(
        &tmp,
        &[
            "backup",
            "create",
            "-C",
            tmp.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            "artifact.xunbak",
        ],
    );

    bencher.bench(|| {
        let _ = fs::remove_file(tmp.join("converted.7z"));
        run_xun(
            &tmp,
            &[
                "backup",
                "convert",
                tmp.join("artifact.xunbak").to_str().unwrap(),
                "--format",
                "7z",
                "-o",
                tmp.join("converted.7z").to_str().unwrap(),
            ],
        );
    });

    let _ = fs::remove_dir_all(&tmp);
}

#[divan::bench]
fn restore_zip_200_files(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_export_bench_restore_zip");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    populate_files(&tmp, 200);
    fs::write(tmp.join(".xun-bak.json"), CFG).unwrap();
    run_xun(
        &tmp,
        &[
            "backup",
            "create",
            "-C",
            tmp.to_str().unwrap(),
            "--format",
            "zip",
            "-o",
            "artifact.zip",
        ],
    );

    bencher.bench(|| {
        let restore = tmp.join("restore_zip");
        let _ = fs::remove_dir_all(&restore);
        run_xun(
            &tmp,
            &[
                "backup",
                "restore",
                tmp.join("artifact.zip").to_str().unwrap(),
                "--to",
                restore.to_str().unwrap(),
                "-C",
                tmp.to_str().unwrap(),
                "-y",
            ],
        );
    });

    let _ = fs::remove_dir_all(&tmp);
}

#[divan::bench]
fn restore_7z_200_files(bencher: Bencher) {
    let tmp = std::env::temp_dir().join("xun_export_bench_restore_7z");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    populate_files(&tmp, 200);
    fs::write(tmp.join(".xun-bak.json"), CFG).unwrap();
    run_xun(
        &tmp,
        &[
            "backup",
            "create",
            "-C",
            tmp.to_str().unwrap(),
            "--format",
            "7z",
            "-o",
            "artifact.7z",
        ],
    );

    bencher.bench(|| {
        let restore = tmp.join("restore_7z");
        let _ = fs::remove_dir_all(&restore);
        run_xun(
            &tmp,
            &[
                "backup",
                "restore",
                tmp.join("artifact.7z").to_str().unwrap(),
                "--to",
                restore.to_str().unwrap(),
                "-C",
                tmp.to_str().unwrap(),
                "-y",
            ],
        );
    });

    let _ = fs::remove_dir_all(&tmp);
}
