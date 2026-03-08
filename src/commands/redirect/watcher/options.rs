use std::env;

const DEFAULT_DEBOUNCE_MS: u64 = 800;
const DEFAULT_SETTLE_MS: u64 = 500;
const DEFAULT_RETRY_MS: u64 = 1_000;
const DEFAULT_SCAN_RECHECK_MS: u64 = 10_000;
const DEFAULT_MAX_PATHS_PER_BATCH: usize = 256;
const DEFAULT_MAX_RETRY_PATHS_PER_BATCH: usize = 128;
const DEFAULT_MAX_SWEEP_DIRS_PER_BATCH: usize = 64;
const DEFAULT_SWEEP_MAX_DEPTH: usize = 32;

#[derive(Clone)]
pub(super) struct WatchOptions {
    pub(super) debounce_ms: u64,
    pub(super) settle_ms: u64,
    pub(super) retry_ms: u64,
    pub(super) scan_recheck_ms: u64,
    pub(super) max_batches: Option<u64>,
    pub(super) buffer_len: u32,
    pub(super) max_paths_per_batch: usize,
    pub(super) max_retry_paths_per_batch: usize,
    pub(super) max_sweep_dirs_per_batch: usize,
    pub(super) sweep_max_depth: usize,
}

impl WatchOptions {
    pub(super) fn from_env() -> Self {
        let debounce_ms = env_u64("XUN_REDIRECT_WATCH_DEBOUNCE_MS", DEFAULT_DEBOUNCE_MS);
        let settle_ms = env_u64("XUN_REDIRECT_WATCH_SETTLE_MS", DEFAULT_SETTLE_MS);
        let retry_ms = env_u64("XUN_REDIRECT_WATCH_RETRY_MS", DEFAULT_RETRY_MS);
        let scan_recheck_ms = env_u64(
            "XUN_REDIRECT_WATCH_SCAN_RECHECK_MS",
            DEFAULT_SCAN_RECHECK_MS,
        );
        let max_batches = env::var("XUN_REDIRECT_WATCH_MAX_BATCHES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| *v > 0);
        let max_paths_per_batch =
            env_usize("XUN_REDIRECT_WATCH_MAX_PATHS", DEFAULT_MAX_PATHS_PER_BATCH);
        let max_retry_paths_per_batch = env_usize(
            "XUN_REDIRECT_WATCH_MAX_RETRY_PATHS",
            DEFAULT_MAX_RETRY_PATHS_PER_BATCH,
        );
        let max_sweep_dirs_per_batch = env_usize(
            "XUN_REDIRECT_WATCH_MAX_SWEEP_DIRS",
            DEFAULT_MAX_SWEEP_DIRS_PER_BATCH,
        );
        let sweep_max_depth = env_usize(
            "XUN_REDIRECT_WATCH_SWEEP_MAX_DEPTH",
            DEFAULT_SWEEP_MAX_DEPTH,
        );
        Self {
            debounce_ms,
            settle_ms,
            retry_ms,
            scan_recheck_ms,
            max_batches,
            buffer_len: 64 * 1024,
            max_paths_per_batch,
            max_retry_paths_per_batch,
            max_sweep_dirs_per_batch,
            sweep_max_depth,
        }
    }

    pub(super) fn should_exit(&self, batch_count: u64) -> bool {
        self.max_batches.map(|m| batch_count >= m).unwrap_or(false)
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}
