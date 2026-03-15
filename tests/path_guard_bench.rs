use std::time::{Duration, Instant};

use xun::path_guard::{validate_paths, PathPolicy};

const EXISTING_FILES: usize = 500;
const TOTAL_PATHS: usize = 1000;
const RUNS: usize = 3;

#[test]
#[ignore]
fn bench_validate_paths_parallel() {
    let dir = tempfile::tempdir().expect("tempdir");
    for idx in 0..EXISTING_FILES {
        let path = dir.path().join(format!("file_{idx}.txt"));
        std::fs::write(&path, "ok").expect("write");
    }

    let mut inputs: Vec<String> = Vec::with_capacity(TOTAL_PATHS);
    for idx in 0..EXISTING_FILES {
        let path = dir.path().join(format!("file_{idx}.txt"));
        inputs.push(path.to_string_lossy().to_string());
    }
    for idx in 0..(TOTAL_PATHS - EXISTING_FILES) {
        let path = dir.path().join(format!("missing_{idx}.txt"));
        inputs.push(path.to_string_lossy().to_string());
    }

    let policy = PathPolicy::for_read();
    let mut samples: Vec<Duration> = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let start = Instant::now();
        let result = validate_paths(inputs.clone(), &policy);
        let elapsed = start.elapsed();
        assert_eq!(result.ok.len(), EXISTING_FILES);
        samples.push(elapsed);
    }

    let total: Duration = samples.iter().copied().sum();
    let avg = total / RUNS as u32;

    let cpu = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let cpu_id = std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_string());

    println!(
        "bench:path_guard total_paths={} existing={} runs={} avg_ms={} cpu_threads={} cpu_id={}",
        TOTAL_PATHS,
        EXISTING_FILES,
        RUNS,
        avg.as_millis(),
        cpu,
        cpu_id
    );
    println!("bench:path_guard disk_type=unknown");
}
