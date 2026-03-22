#![cfg(windows)]

#[path = "../support/mod.rs"]
mod common;

use std::fs;
use std::path::PathBuf;

use common::env_usize;
use xun::path_guard::{validate_paths, PathPolicy};

fn path_string(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}

fn trace_env_enabled() -> bool {
    std::env::var("XUN_PG_TRACE")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

fn warn_trace_disabled() {
    if !trace_env_enabled() {
        eprintln!("trace:path_guard disabled (set XUN_PG_TRACE=1 to view stage timings)");
    }
}

#[test]
#[ignore]
fn trace_dedupe_hot() {
    warn_trace_disabled();
    let dir = tempfile::tempdir().expect("tempdir");
    let total = env_usize("XUN_PG_TRACE_DEDUPE_TOTAL", 2048).max(64);
    let unique = env_usize("XUN_PG_TRACE_DEDUPE_UNIQUE", 64).min(total);

    let mut inputs: Vec<String> = Vec::with_capacity(total);
    for idx in 0..unique {
        let path = dir.path().join(format!("u{idx}.txt"));
        inputs.push(path_string(&path));
    }
    let dup = dir.path().join("dup.txt");
    for _ in unique..total {
        inputs.push(path_string(&dup));
    }

    let policy = PathPolicy::for_output();
    let result = validate_paths(inputs, &policy);
    let expected_unique = if total > unique { unique + 1 } else { unique };
    assert_eq!(result.ok.len(), expected_unique);
    assert_eq!(result.deduped, total - expected_unique);
}

#[test]
#[ignore]
fn trace_dedupe_variants() {
    warn_trace_disabled();
    let dir = tempfile::tempdir().expect("tempdir");
    let base = dir.path().join("mixcase").join("file.txt");
    let base_str = path_string(&base);
    let upper = path_string(&dir.path().join("MIXCASE").join("FILE.TXT"));
    let forward = base_str.replace('\\', "/");
    let trailing = format!("{base_str}\\");

    let policy = PathPolicy::for_output();
    let result = validate_paths(vec![base_str, upper, forward, trailing], &policy);
    assert_eq!(result.ok.len(), 1);
    assert_eq!(result.deduped, 3);
}

#[test]
#[ignore]
fn trace_batch_probe_hot() {
    warn_trace_disabled();
    let dir = tempfile::tempdir().expect("tempdir");
    let existing = env_usize("XUN_PG_TRACE_PROBE_EXISTING", 64).max(16);
    let missing = env_usize("XUN_PG_TRACE_PROBE_MISSING", 64).max(16);

    let mut inputs: Vec<String> = Vec::with_capacity(existing + missing);
    for idx in 0..existing {
        let path = dir.path().join(format!("file_{idx}.txt"));
        fs::write(&path, "ok").expect("write");
        inputs.push(path_string(&path));
    }
    for idx in 0..missing {
        let path = dir.path().join(format!("missing_{idx}.txt"));
        inputs.push(path_string(&path));
    }

    let policy = PathPolicy::for_read();
    let result = validate_paths(inputs, &policy);
    assert_eq!(result.ok.len(), existing);
    assert_eq!(result.issues.len(), missing);
}

#[test]
#[ignore]
fn trace_probe_threshold_small() {
    warn_trace_disabled();
    let dir = tempfile::tempdir().expect("tempdir");
    let existing = env_usize("XUN_PG_TRACE_PROBE_SMALL_EXISTING", 4).max(1);
    let missing = env_usize("XUN_PG_TRACE_PROBE_SMALL_MISSING", 4).max(1);

    let mut inputs: Vec<String> = Vec::with_capacity(existing + missing);
    for idx in 0..existing {
        let path = dir.path().join(format!("small_{idx}.txt"));
        fs::write(&path, "ok").expect("write");
        inputs.push(path_string(&path));
    }
    for idx in 0..missing {
        let path = dir.path().join(format!("missing_small_{idx}.txt"));
        inputs.push(path_string(&path));
    }

    let policy = PathPolicy::for_read();
    let result = validate_paths(inputs, &policy);
    assert_eq!(result.ok.len(), existing);
    assert_eq!(result.issues.len(), missing);
}

#[test]
#[ignore]
fn trace_expand_env() {
    warn_trace_disabled();

    let mut policy = PathPolicy::for_output();
    policy.expand_env = true;
    policy.allow_relative = true;

    let input = "%TEMP%\\xun-path-guard-trace.txt".to_string();
    let result = validate_paths(vec![input], &policy);
    assert_eq!(result.ok.len(), 1);
    assert!(result.issues.is_empty());
}

#[test]
#[ignore]
fn trace_relative_resolve() {
    warn_trace_disabled();
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("rel.txt");
    fs::write(&file, "ok").expect("write");

    let mut policy = PathPolicy::for_output();
    policy.allow_relative = true;
    policy.cwd_snapshot = Some(dir.path().to_path_buf());

    let result = validate_paths(vec!["rel.txt".to_string()], &policy);
    assert_eq!(result.ok.len(), 1);
    assert!(result.issues.is_empty());
}
