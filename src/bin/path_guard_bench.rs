use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use xun::path_guard::{PathPolicy, validate_paths, validate_paths_owned};

const DEFAULT_EXISTING: usize = 2_500;
const DEFAULT_TOTAL: usize = 5_000;
const DEFAULT_RUNS: usize = 5;

struct BenchDir {
    path: PathBuf,
}

impl Drop for BenchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn main() {
    let existing = env_usize("XUN_PG_BENCH_EXISTING", DEFAULT_EXISTING);
    let total = env_usize("XUN_PG_BENCH_TOTAL", DEFAULT_TOTAL).max(existing);
    let runs = env_usize("XUN_PG_BENCH_RUNS", DEFAULT_RUNS).max(1);
    let warmup_ms = env_usize("XUN_PG_BENCH_WARMUP_MS", 0);

    let dir = make_temp_dir();
    let _guard = BenchDir { path: dir.clone() };

    for idx in 0..existing {
        let path = dir.join(format!("file_{idx}.txt"));
        fs::write(&path, "ok").expect("write");
    }

    let mut inputs: Vec<PathBuf> = Vec::with_capacity(total);
    for idx in 0..existing {
        inputs.push(dir.join(format!("file_{idx}.txt")));
    }
    for idx in 0..(total - existing) {
        inputs.push(dir.join(format!("missing_{idx}.txt")));
    }

    if warmup_ms > 0 {
        std::thread::sleep(Duration::from_millis(warmup_ms as u64));
    }

    let policy = PathPolicy::for_read();
    let avg_borrow = bench_borrow(&inputs, &policy, existing, runs);
    let avg_owned = bench_owned(&inputs, &policy, existing, runs);

    let cpu = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let cpu_id = std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_string());

    println!(
        "bench:path_guard mode=borrow total_paths={} existing={} runs={} avg_ms={} cpu_threads={} cpu_id={}",
        total,
        existing,
        runs,
        avg_borrow.as_millis(),
        cpu,
        cpu_id
    );
    println!(
        "bench:path_guard mode=owned total_paths={} existing={} runs={} avg_ms={} cpu_threads={} cpu_id={}",
        total,
        existing,
        runs,
        avg_owned.as_millis(),
        cpu,
        cpu_id
    );
    println!("bench:path_guard disk_type=unknown");
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn make_temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    for _ in 0..32 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let candidate = base.join(format!(
            "xun-path-guard-bench-{}-{nanos}",
            std::process::id()
        ));
        if fs::create_dir(&candidate).is_ok() {
            return candidate;
        }
    }
    panic!("failed to create bench temp dir");
}

fn bench_borrow(inputs: &[PathBuf], policy: &PathPolicy, existing: usize, runs: usize) -> Duration {
    let mut samples: Vec<Duration> = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = Instant::now();
        let result = validate_paths(inputs.iter(), policy);
        let elapsed = start.elapsed();
        assert_eq!(result.ok.len(), existing);
        samples.push(elapsed);
    }
    let total_time: Duration = samples.iter().copied().sum();
    total_time / runs as u32
}

fn bench_owned(inputs: &[PathBuf], policy: &PathPolicy, existing: usize, runs: usize) -> Duration {
    let mut samples: Vec<Duration> = Vec::with_capacity(runs);
    for _ in 0..runs {
        let owned_inputs = inputs.to_vec();
        let start = Instant::now();
        let result = validate_paths_owned(owned_inputs, policy);
        let elapsed = start.elapsed();
        assert_eq!(result.ok.len(), existing);
        samples.push(elapsed);
    }
    let total_time: Duration = samples.iter().copied().sum();
    total_time / runs as u32
}
