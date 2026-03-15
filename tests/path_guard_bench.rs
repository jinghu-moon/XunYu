mod common;

use std::path::PathBuf;
use std::process::Command;

use common::{
    env_u64, measure_handle_peak_count, measure_working_set_peak_with_baseline_bytes,
};

#[test]
#[ignore]
fn bench_path_guard_process_resources() {
    let exe = resolve_bench_exe();
    let output = Command::new(&exe).output().expect("run bench");
    assert!(output.status.success(), "bench failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        print!("{stdout}");
    }

    let sample_ms = env_u64("XUN_PG_BENCH_SAMPLE_MS").unwrap_or(10);
    let warmup_ms = env_u64("XUN_PG_BENCH_WARMUP_MS").unwrap_or(200);

    let (baseline_peak, mem_peak) = measure_working_set_peak_with_baseline_bytes(
        bench_command(&exe, warmup_ms).spawn().expect("spawn bench"),
        sample_ms,
        warmup_ms,
    );
    let mem_delta = mem_peak.saturating_sub(baseline_peak);
    let handle_peak = measure_handle_peak_count(
        bench_command(&exe, warmup_ms).spawn().expect("spawn bench"),
        sample_ms,
    );

    println!(
        "bench:path_guard working_set_peak_bytes={} working_set_baseline_bytes={} working_set_delta_bytes={} handle_peak_count={} sample_ms={} warmup_ms={}",
        mem_peak, baseline_peak, mem_delta, handle_peak, sample_ms, warmup_ms
    );
}

fn bench_command(exe: &PathBuf, warmup_ms: u64) -> Command {
    let mut cmd = Command::new(exe);
    if warmup_ms > 0 {
        cmd.env("XUN_PG_BENCH_WARMUP_MS", warmup_ms.to_string());
    }
    cmd
}

fn resolve_bench_exe() -> PathBuf {
    if let Ok(exe) = std::env::var("CARGO_BIN_EXE_path_guard_bench") {
        return PathBuf::from(exe);
    }

    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = find_profile_dir(&current) {
            let candidate = dir.join(exe_name("path_guard_bench"));
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
        let candidate = PathBuf::from(manifest)
            .join("target")
            .join(profile)
            .join(exe_name("path_guard_bench"));
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "path_guard_bench binary not found. Run: cargo test --bins --test path_guard_bench -- --ignored --nocapture"
    );
}

fn find_profile_dir(current_exe: &std::path::Path) -> Option<PathBuf> {
    let mut cursor = current_exe.parent()?.to_path_buf();
    loop {
        if cursor.file_name().map(|n| n == "deps").unwrap_or(false) {
            return cursor.parent().map(|p| p.to_path_buf());
        }
        if !cursor.pop() {
            return None;
        }
    }
}

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}
