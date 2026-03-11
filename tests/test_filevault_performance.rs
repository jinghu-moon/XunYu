#![cfg(all(windows, feature = "crypt"))]

mod common;

use common::*;
use std::fs::{self, File};
use std::io::Write;
use std::process::Stdio;
use std::time::Instant;

fn mib(value: usize) -> usize {
    value * 1024 * 1024
}

fn env_mib(key: &str, default_mib: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map(mib)
        .unwrap_or_else(|| mib(default_mib))
}

fn parse_usize_list(value: &str) -> Vec<usize> {
    let mut values: Vec<usize> = value
        .split(|ch| ch == ',' || ch == ';' || ch == ' ')
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<usize>().ok()
            }
        })
        .filter(|value| *value > 0)
        .collect();
    values.sort_unstable();
    values.dedup();
    values
}

fn env_usize_list(key: &str) -> Option<Vec<usize>> {
    std::env::var(key)
        .ok()
        .map(|value| parse_usize_list(&value))
        .filter(|values| !values.is_empty())
}

fn env_mib_list(list_key: &str, single_key: &str, default_mib: &[usize]) -> Vec<usize> {
    if let Some(values) = env_usize_list(list_key) {
        return values;
    }
    if let Ok(value) = std::env::var(single_key) {
        if let Ok(single) = value.parse::<usize>() {
            if single > 0 {
                return vec![single];
            }
        }
    }
    default_mib.to_vec()
}

fn env_kib_list(list_key: &str, default_kib: &[usize]) -> Vec<usize> {
    env_usize_list(list_key).unwrap_or_else(|| default_kib.to_vec())
}

fn env_bytes_list(list_key: &str, single_key: &str, default_bytes: &[usize]) -> Vec<usize> {
    if let Some(values) = env_usize_list(list_key) {
        return values;
    }
    if let Ok(value) = std::env::var(single_key) {
        if let Ok(single) = value.parse::<usize>() {
            if single > 0 {
                return vec![single];
            }
        }
    }
    default_bytes.to_vec()
}

fn write_pattern_file(path: &std::path::Path, size_bytes: usize) {
    let mut file = File::create(path).unwrap();
    let chunk = vec![0xA5u8; 1024 * 1024];
    let mut remaining = size_bytes;
    while remaining > 0 {
        let take = remaining.min(chunk.len());
        file.write_all(&chunk[..take]).unwrap();
        remaining -= take;
    }
    file.flush().unwrap();
}

fn throughput_mib_per_sec(size_bytes: usize, elapsed: std::time::Duration) -> f64 {
    let secs = elapsed.as_secs_f64().max(0.000_001);
    (size_bytes as f64 / (1024.0 * 1024.0)) / secs
}

fn ops_per_sec(count: usize, elapsed: std::time::Duration) -> f64 {
    let secs = elapsed.as_secs_f64().max(0.000_001);
    count as f64 / secs
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn stddev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let avg = mean(values);
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - avg;
            delta * delta
        })
        .sum::<f64>()
        / (values.len() as f64 - 1.0);
    variance.sqrt()
}

#[test]
#[ignore]
fn perf_filevault_encrypt_decrypt_throughput() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-perf-throughput");
    let plain = work.join("payload.bin");
    let vault = work.join("payload.bin.fv");
    let out = work.join("payload.out.bin");
    let size_bytes = env_mib("XUN_TEST_FILEVAULT_THROUGHPUT_MIB", 64);
    let chunk_size = std::env::var("XUN_TEST_FILEVAULT_CHUNK_SIZE")
        .ok()
        .unwrap_or_else(|| "262144".to_string());
    let runs = env_usize("XUN_TEST_FILEVAULT_THROUGHPUT_RUNS", 3).max(1);
    let warmups = env_usize("XUN_TEST_FILEVAULT_THROUGHPUT_WARMUP", 1);

    write_pattern_file(&plain, size_bytes);

    for warmup in 0..warmups {
        run_ok_status(
            env.cmd().args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                vault.to_str().unwrap(),
                "--password",
                "perf-secret",
                "--chunk-size",
                &chunk_size,
            ]),
        );
        run_ok_status(
            env.cmd().args([
                "vault",
                "dec",
                vault.to_str().unwrap(),
                "-o",
                out.to_str().unwrap(),
                "--password",
                "perf-secret",
            ]),
        );
        eprintln!(
            "perf: filevault throughput warmup={} size_mib={} chunk_size={}",
            warmup + 1,
            size_bytes / (1024 * 1024),
            chunk_size
        );
    }

    let mut encrypt_mib_s_runs = Vec::with_capacity(runs);
    let mut decrypt_mib_s_runs = Vec::with_capacity(runs);
    let mut encrypt_ms_runs = Vec::with_capacity(runs);
    let mut decrypt_ms_runs = Vec::with_capacity(runs);

    for run in 0..runs {
        let encrypt_start = Instant::now();
        run_ok_status(
            env.cmd().args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                vault.to_str().unwrap(),
                "--password",
                "perf-secret",
                "--chunk-size",
                &chunk_size,
            ]),
        );
        let encrypt_elapsed = encrypt_start.elapsed();

        let decrypt_start = Instant::now();
        run_ok_status(
            env.cmd().args([
                "vault",
                "dec",
                vault.to_str().unwrap(),
                "-o",
                out.to_str().unwrap(),
                "--password",
                "perf-secret",
            ]),
        );
        let decrypt_elapsed = decrypt_start.elapsed();

        let encrypt_mib_s = throughput_mib_per_sec(size_bytes, encrypt_elapsed);
        let decrypt_mib_s = throughput_mib_per_sec(size_bytes, decrypt_elapsed);
        encrypt_mib_s_runs.push(encrypt_mib_s);
        decrypt_mib_s_runs.push(decrypt_mib_s);
        encrypt_ms_runs.push(encrypt_elapsed.as_secs_f64() * 1000.0);
        decrypt_ms_runs.push(decrypt_elapsed.as_secs_f64() * 1000.0);

        eprintln!(
            "perf: filevault throughput run={} size_mib={} chunk_size={} enc_ms={} enc_mib_s={:.2} dec_ms={} dec_mib_s={:.2}",
            run + 1,
            size_bytes / (1024 * 1024),
            chunk_size,
            encrypt_elapsed.as_millis(),
            encrypt_mib_s,
            decrypt_elapsed.as_millis(),
            decrypt_mib_s
        );
    }

    let enc_mean = mean(&encrypt_mib_s_runs);
    let enc_std = stddev(&encrypt_mib_s_runs);
    let dec_mean = mean(&decrypt_mib_s_runs);
    let dec_std = stddev(&decrypt_mib_s_runs);
    let enc_ms_mean = mean(&encrypt_ms_runs);
    let enc_ms_std = stddev(&encrypt_ms_runs);
    let dec_ms_mean = mean(&decrypt_ms_runs);
    let dec_ms_std = stddev(&decrypt_ms_runs);
    eprintln!(
        "perf: filevault throughput summary runs={} size_mib={} chunk_size={} enc_mib_s_mean={:.2} enc_mib_s_std={:.2} dec_mib_s_mean={:.2} dec_mib_s_std={:.2} enc_ms_mean={:.1} enc_ms_std={:.1} dec_ms_mean={:.1} dec_ms_std={:.1}",
        runs,
        size_bytes / (1024 * 1024),
        chunk_size,
        enc_mean,
        enc_std,
        dec_mean,
        dec_std,
        enc_ms_mean,
        enc_ms_std,
        dec_ms_mean,
        dec_ms_std
    );

    if let Some(min_encrypt) = env_u64("XUN_TEST_FILEVAULT_ENCRYPT_MIN_MIB_S") {
        assert!(
            enc_mean >= min_encrypt as f64,
            "encrypt throughput mean {:.2} MiB/s < {} MiB/s",
            enc_mean,
            min_encrypt
        );
    }
    if let Some(min_decrypt) = env_u64("XUN_TEST_FILEVAULT_DECRYPT_MIN_MIB_S") {
        assert!(
            dec_mean >= min_decrypt as f64,
            "decrypt throughput mean {:.2} MiB/s < {} MiB/s",
            dec_mean,
            min_decrypt
        );
    }
    assert_eq!(fs::metadata(&out).unwrap().len(), size_bytes as u64);
    cleanup_dir(&work);
}

#[test]
#[ignore]
fn perf_filevault_encrypt_peak_working_set_and_handles() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-perf-resources");
    let plain = work.join("resource.bin");
    let vault = work.join("resource.bin.fv");
    let vault_handles = work.join("resource-handles.bin.fv");
    let size_bytes = env_mib("XUN_TEST_FILEVAULT_RESOURCE_MIB", 128);
    let sample_ms = env_u64("XUN_TEST_FILEVAULT_SAMPLE_MS").unwrap_or(50);
    let runs = env_usize("XUN_TEST_FILEVAULT_RESOURCE_RUNS", 3).max(1);
    let warmups = env_usize("XUN_TEST_FILEVAULT_RESOURCE_WARMUP", 1);

    write_pattern_file(&plain, size_bytes);

    for warmup in 0..warmups {
        run_ok_status(
            env.cmd().args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                vault.to_str().unwrap(),
                "--password",
                "perf-secret",
            ]),
        );
        run_ok_status(
            env.cmd().args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                vault_handles.to_str().unwrap(),
                "--password",
                "perf-secret",
            ]),
        );
        eprintln!(
            "perf: filevault resources warmup={} size_mib={} sample_ms={}",
            warmup + 1,
            size_bytes / (1024 * 1024),
            sample_ms
        );
        let _ = fs::remove_file(&vault);
        let _ = fs::remove_file(&vault_handles);
    }

    let mut mem_peaks = Vec::with_capacity(runs);
    let mut handle_peaks = Vec::with_capacity(runs);

    for run in 0..runs {
        let mut mem_cmd = env.cmd();
        mem_cmd
            .args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                vault.to_str().unwrap(),
                "--password",
                "perf-secret",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mem_peak = measure_working_set_peak_bytes(mem_cmd.spawn().unwrap(), sample_ms);
        assert!(vault.exists(), "ciphertext must exist after memory probe run");

        let mut handle_cmd = env.cmd();
        handle_cmd
            .args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                vault_handles.to_str().unwrap(),
                "--password",
                "perf-secret",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let handle_peak = measure_handle_peak_count(handle_cmd.spawn().unwrap(), sample_ms);
        assert!(vault_handles.exists(), "ciphertext must exist after handle probe run");

        mem_peaks.push(mem_peak as f64);
        handle_peaks.push(handle_peak as f64);

        eprintln!(
            "perf: filevault resources run={} size_mib={} sample_ms={} working_set_peak_bytes={} handle_peak_count={}",
            run + 1,
            size_bytes / (1024 * 1024),
            sample_ms,
            mem_peak,
            handle_peak
        );

        let _ = fs::remove_file(&vault);
        let _ = fs::remove_file(&vault_handles);
    }

    let mem_mean = mean(&mem_peaks);
    let mem_std = stddev(&mem_peaks);
    let handle_mean = mean(&handle_peaks);
    let handle_std = stddev(&handle_peaks);
    eprintln!(
        "perf: filevault resources summary runs={} size_mib={} sample_ms={} working_set_peak_bytes_mean={:.0} working_set_peak_bytes_std={:.0} handle_peak_count_mean={:.1} handle_peak_count_std={:.1}",
        runs,
        size_bytes / (1024 * 1024),
        sample_ms,
        mem_mean,
        mem_std,
        handle_mean,
        handle_std
    );

    if let Some(max_ws) = env_u64("XUN_TEST_FILEVAULT_WS_PEAK_MAX") {
        assert!(
            mem_mean <= max_ws as f64,
            "working set peak mean {:.0} > {}",
            mem_mean,
            max_ws
        );
    }
    if let Some(max_handles) = env_u64("XUN_TEST_FILEVAULT_HANDLE_PEAK_MAX") {
        assert!(
            handle_mean <= max_handles as f64,
            "handle peak mean {:.1} > {}",
            handle_mean,
            max_handles
        );
    }
    cleanup_dir(&work);
}

#[test]
#[ignore]
fn perf_filevault_rewrap_vs_reencrypt() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-perf-rewrap");
    let plain = work.join("compare.bin");
    let vault = work.join("compare.bin.fv");
    let reencrypt_vault = work.join("compare.reencrypt.bin.fv");
    let keyfile = work.join("compare.key");
    let size_bytes = env_mib("XUN_TEST_FILEVAULT_COMPARE_MIB", 128);
    let runs = env_usize("XUN_TEST_FILEVAULT_REWRAP_RUNS", 3).max(1);
    let warmups = env_usize("XUN_TEST_FILEVAULT_REWRAP_WARMUP", 1);

    write_pattern_file(&plain, size_bytes);
    fs::write(&keyfile, b"filevault-perf-keyfile").unwrap();

    let prepare_vault = || {
        let _ = fs::remove_file(&vault);
        run_ok_status(
            env.cmd().args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                vault.to_str().unwrap(),
                "--password",
                "old-secret",
            ]),
        );
    };

    for warmup in 0..warmups {
        prepare_vault();
        run_ok_status(
            env.cmd().args([
                "vault",
                "rewrap",
                vault.to_str().unwrap(),
                "--unlock-password",
                "old-secret",
                "--add-password",
                "new-secret",
                "--add-keyfile",
                keyfile.to_str().unwrap(),
                "--remove-slot",
                "password",
            ]),
        );
        let _ = fs::remove_file(&vault);

        run_ok_status(
            env.cmd().args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                reencrypt_vault.to_str().unwrap(),
                "--password",
                "new-secret",
                "--keyfile",
                keyfile.to_str().unwrap(),
            ]),
        );
        let _ = fs::remove_file(&reencrypt_vault);

        eprintln!("perf: filevault rewrap warmup={} size_mib={}", warmup + 1, size_bytes / (1024 * 1024));
    }

    let mut rewrap_ms_runs = Vec::with_capacity(runs);
    let mut reencrypt_ms_runs = Vec::with_capacity(runs);
    let mut reencrypt_mib_s_runs = Vec::with_capacity(runs);
    let mut ratio_runs = Vec::with_capacity(runs);

    for run in 0..runs {
        prepare_vault();
        let rewrap_start = Instant::now();
        run_ok_status(
            env.cmd().args([
                "vault",
                "rewrap",
                vault.to_str().unwrap(),
                "--unlock-password",
                "old-secret",
                "--add-password",
                "new-secret",
                "--add-keyfile",
                keyfile.to_str().unwrap(),
                "--remove-slot",
                "password",
            ]),
        );
        let rewrap_elapsed = rewrap_start.elapsed();
        let _ = fs::remove_file(&vault);

        let reencrypt_start = Instant::now();
        run_ok_status(
            env.cmd().args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                reencrypt_vault.to_str().unwrap(),
                "--password",
                "new-secret",
                "--keyfile",
                keyfile.to_str().unwrap(),
            ]),
        );
        let reencrypt_elapsed = reencrypt_start.elapsed();
        let _ = fs::remove_file(&reencrypt_vault);

        let rewrap_ms = rewrap_elapsed.as_secs_f64() * 1000.0;
        let reencrypt_ms = reencrypt_elapsed.as_secs_f64() * 1000.0;
        let ratio = reencrypt_elapsed.as_secs_f64() / rewrap_elapsed.as_secs_f64().max(0.000_001);
        let reencrypt_mib_s = throughput_mib_per_sec(size_bytes, reencrypt_elapsed);

        rewrap_ms_runs.push(rewrap_ms);
        reencrypt_ms_runs.push(reencrypt_ms);
        reencrypt_mib_s_runs.push(reencrypt_mib_s);
        ratio_runs.push(ratio);

        eprintln!(
            "perf: filevault rewrap run={} rewrap_ms={:.0} reencrypt_ms={:.0} reencrypt_mib_s={:.2} ratio={:.2}",
            run + 1,
            rewrap_ms,
            reencrypt_ms,
            reencrypt_mib_s,
            ratio
        );
    }

    let rewrap_mean = mean(&rewrap_ms_runs);
    let rewrap_std = stddev(&rewrap_ms_runs);
    let reencrypt_mean = mean(&reencrypt_ms_runs);
    let reencrypt_std = stddev(&reencrypt_ms_runs);
    let reencrypt_mib_mean = mean(&reencrypt_mib_s_runs);
    let reencrypt_mib_std = stddev(&reencrypt_mib_s_runs);
    let ratio_mean = mean(&ratio_runs);
    let ratio_std = stddev(&ratio_runs);

    eprintln!("perf: filevault compare-table");
    eprintln!("perf: op\telapsed_ms_mean\telapsed_ms_std\tthroughput_mib_s_mean\tthroughput_mib_s_std\tnotes");
    eprintln!(
        "perf: rewrap\t{:.0}\t{:.0}\t-\t-\tpayload invariant slot replacement",
        rewrap_mean,
        rewrap_std
    );
    eprintln!(
        "perf: reencrypt\t{:.0}\t{:.0}\t{:.2}\t{:.2}\tfull payload encryption",
        reencrypt_mean,
        reencrypt_std,
        reencrypt_mib_mean,
        reencrypt_mib_std
    );
    eprintln!(
        "perf: reencrypt_over_rewrap_ratio_mean={:.2} ratio_std={:.2}",
        ratio_mean,
        ratio_std
    );

    if let Some(min_ratio) = env_u64("XUN_TEST_FILEVAULT_REENCRYPT_OVER_REWRAP_MIN_RATIO") {
        assert!(
            ratio_mean >= min_ratio as f64,
            "ratio mean {:.2} < {}",
            ratio_mean,
            min_ratio
        );
    }
    if env_bool("XUN_TEST_FILEVAULT_REWRAP_ASSERT_FASTER", false) {
        assert!(
            rewrap_mean < reencrypt_mean,
            "rewrap should be faster than re-encrypt in this benchmark"
        );
    }
    cleanup_dir(&work);
}

#[test]
#[ignore]
fn perf_filevault_large_file_throughput() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-perf-large");
    let size_mib_list =
        env_mib_list("XUN_TEST_FILEVAULT_LARGE_MIB_LIST", "XUN_TEST_FILEVAULT_LARGE_MIB", &[2048, 4096, 8192]);
    let chunk_sizes = env_bytes_list(
        "XUN_TEST_FILEVAULT_LARGE_CHUNK_SIZES",
        "XUN_TEST_FILEVAULT_LARGE_CHUNK_SIZE",
        &[65536, 262144, 1048576],
    );
    let runs = env_usize("XUN_TEST_FILEVAULT_LARGE_RUNS", 3).max(1);
    let warmups = env_usize("XUN_TEST_FILEVAULT_LARGE_WARMUP", 1);

    for size_mib in size_mib_list {
        let size_bytes = mib(size_mib);
        let size_root = work.join(format!("size-{size_mib}mib"));
        fs::create_dir_all(&size_root).unwrap();
        let plain = size_root.join("large.bin");

        write_pattern_file(&plain, size_bytes);

        for chunk_size in &chunk_sizes {
            let chunk_size_value = chunk_size.to_string();
            let vault = size_root.join(format!("large-{chunk_size}.bin.fv"));
            let out = size_root.join(format!("large-{chunk_size}.out.bin"));

            for warmup in 0..warmups {
                run_ok_status(
                    env.cmd().args([
                        "vault",
                        "enc",
                        plain.to_str().unwrap(),
                        "-o",
                        vault.to_str().unwrap(),
                        "--password",
                        "perf-secret",
                        "--chunk-size",
                        &chunk_size_value,
                    ]),
                );
                run_ok_status(
                    env.cmd().args([
                        "vault",
                        "dec",
                        vault.to_str().unwrap(),
                        "-o",
                        out.to_str().unwrap(),
                        "--password",
                        "perf-secret",
                    ]),
                );
                eprintln!(
                    "perf: filevault large warmup={} size_mib={} chunk_size={}",
                    warmup + 1,
                    size_bytes / (1024 * 1024),
                    chunk_size
                );
                let _ = fs::remove_file(&vault);
                let _ = fs::remove_file(&out);
            }

            let mut encrypt_mib_s_runs = Vec::with_capacity(runs);
            let mut decrypt_mib_s_runs = Vec::with_capacity(runs);
            let mut encrypt_ms_runs = Vec::with_capacity(runs);
            let mut decrypt_ms_runs = Vec::with_capacity(runs);

            for run in 0..runs {
                let encrypt_start = Instant::now();
                run_ok_status(
                    env.cmd().args([
                        "vault",
                        "enc",
                        plain.to_str().unwrap(),
                        "-o",
                        vault.to_str().unwrap(),
                        "--password",
                        "perf-secret",
                        "--chunk-size",
                        &chunk_size_value,
                    ]),
                );
                let encrypt_elapsed = encrypt_start.elapsed();

                let decrypt_start = Instant::now();
                run_ok_status(
                    env.cmd().args([
                        "vault",
                        "dec",
                        vault.to_str().unwrap(),
                        "-o",
                        out.to_str().unwrap(),
                        "--password",
                        "perf-secret",
                    ]),
                );
                let decrypt_elapsed = decrypt_start.elapsed();

                assert_eq!(fs::metadata(&out).unwrap().len(), size_bytes as u64);

                let encrypt_mib_s = throughput_mib_per_sec(size_bytes, encrypt_elapsed);
                let decrypt_mib_s = throughput_mib_per_sec(size_bytes, decrypt_elapsed);
                encrypt_mib_s_runs.push(encrypt_mib_s);
                decrypt_mib_s_runs.push(decrypt_mib_s);
                encrypt_ms_runs.push(encrypt_elapsed.as_secs_f64() * 1000.0);
                decrypt_ms_runs.push(decrypt_elapsed.as_secs_f64() * 1000.0);

                eprintln!(
                    "perf: filevault large run={} size_mib={} chunk_size={} enc_ms={} enc_mib_s={:.2} dec_ms={} dec_mib_s={:.2}",
                    run + 1,
                    size_bytes / (1024 * 1024),
                    chunk_size,
                    encrypt_elapsed.as_millis(),
                    encrypt_mib_s,
                    decrypt_elapsed.as_millis(),
                    decrypt_mib_s
                );

                let _ = fs::remove_file(&vault);
                let _ = fs::remove_file(&out);
            }

            let enc_mean = mean(&encrypt_mib_s_runs);
            let enc_std = stddev(&encrypt_mib_s_runs);
            let dec_mean = mean(&decrypt_mib_s_runs);
            let dec_std = stddev(&decrypt_mib_s_runs);
            let enc_ms_mean = mean(&encrypt_ms_runs);
            let enc_ms_std = stddev(&encrypt_ms_runs);
            let dec_ms_mean = mean(&decrypt_ms_runs);
            let dec_ms_std = stddev(&decrypt_ms_runs);

            eprintln!(
                "perf: filevault large summary runs={} size_mib={} chunk_size={} enc_mib_s_mean={:.2} enc_mib_s_std={:.2} dec_mib_s_mean={:.2} dec_mib_s_std={:.2} enc_ms_mean={:.1} enc_ms_std={:.1} dec_ms_mean={:.1} dec_ms_std={:.1}",
                runs,
                size_bytes / (1024 * 1024),
                chunk_size,
                enc_mean,
                enc_std,
                dec_mean,
                dec_std,
                enc_ms_mean,
                enc_ms_std,
                dec_ms_mean,
                dec_ms_std
            );

            if let Some(min_encrypt) = env_u64("XUN_TEST_FILEVAULT_LARGE_ENCRYPT_MIN_MIB_S") {
                assert!(
                    enc_mean >= min_encrypt as f64,
                    "large encrypt throughput mean {:.2} MiB/s < {} MiB/s",
                    enc_mean,
                    min_encrypt
                );
            }
            if let Some(min_decrypt) = env_u64("XUN_TEST_FILEVAULT_LARGE_DECRYPT_MIN_MIB_S") {
                assert!(
                    dec_mean >= min_decrypt as f64,
                    "large decrypt throughput mean {:.2} MiB/s < {} MiB/s",
                    dec_mean,
                    min_decrypt
                );
            }
        }

        let _ = fs::remove_file(&plain);
        cleanup_dir(&size_root);
    }
    cleanup_dir(&work);
}

#[test]
#[ignore]
fn perf_filevault_small_files_batch() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-perf-small-batch");
    let default_count = env_usize("XUN_TEST_FILEVAULT_SMALL_COUNT", 256);
    let file_count = env_usize("XUN_TEST_FILEVAULT_SMALL_COUNT_PER_BUCKET", default_count);
    let bucket_kib = env_kib_list("XUN_TEST_FILEVAULT_SMALL_BUCKETS_KIB", &[1, 2, 4, 8, 16, 32, 64]);
    let override_bytes = std::env::var("XUN_TEST_FILEVAULT_SMALL_BYTES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0);
    let chunk_size = std::env::var("XUN_TEST_FILEVAULT_SMALL_CHUNK_SIZE")
        .ok()
        .unwrap_or_else(|| "65536".to_string());
    let runs = env_usize("XUN_TEST_FILEVAULT_SMALL_RUNS", 3).max(1);
    let warmups = env_usize("XUN_TEST_FILEVAULT_SMALL_WARMUP", 1);

    let bucket_list = if let Some(bytes) = override_bytes {
        vec![bytes.max(1)]
    } else {
        bucket_kib.into_iter().map(|kib| kib.max(1) * 1024).collect()
    };

    for size_bytes in bucket_list {
        let kib = size_bytes / 1024;
        let bucket_root = work.join(format!("bucket-{kib}kib"));
        let plain_root = bucket_root.join("plain");
        let cipher_root = bucket_root.join("cipher");
        let out_root = bucket_root.join("out");
        fs::create_dir_all(&plain_root).unwrap();
        fs::create_dir_all(&cipher_root).unwrap();
        fs::create_dir_all(&out_root).unwrap();

        for index in 0..file_count {
            let path = plain_root.join(format!("small-{index:04}.bin"));
            write_pattern_file(&path, size_bytes);
        }

        for warmup in 0..warmups {
            for index in 0..file_count {
                let plain = plain_root.join(format!("small-{index:04}.bin"));
                let vault = cipher_root.join(format!("small-{index:04}.bin.fv"));
                run_ok_status(
                    env.cmd().args([
                        "vault",
                        "enc",
                        plain.to_str().unwrap(),
                        "-o",
                        vault.to_str().unwrap(),
                        "--password",
                        "perf-secret",
                        "--chunk-size",
                        &chunk_size,
                    ]),
                );
            }
            for index in 0..file_count {
                let vault = cipher_root.join(format!("small-{index:04}.bin.fv"));
                let out = out_root.join(format!("small-{index:04}.bin"));
                run_ok_status(
                    env.cmd().args([
                        "vault",
                        "dec",
                        vault.to_str().unwrap(),
                        "-o",
                        out.to_str().unwrap(),
                        "--password",
                        "perf-secret",
                    ]),
                );
            }
            eprintln!(
                "perf: filevault small-batch warmup={} bucket_kib={} count={} size_bytes={} chunk_size={}",
                warmup + 1,
                kib,
                file_count,
                size_bytes,
                chunk_size
            );
            cleanup_dir(&cipher_root);
            cleanup_dir(&out_root);
            fs::create_dir_all(&cipher_root).unwrap();
            fs::create_dir_all(&out_root).unwrap();
        }

        let mut enc_ops_runs = Vec::with_capacity(runs);
        let mut dec_ops_runs = Vec::with_capacity(runs);
        let mut enc_mib_runs = Vec::with_capacity(runs);
        let mut dec_mib_runs = Vec::with_capacity(runs);
        let mut enc_ms_runs = Vec::with_capacity(runs);
        let mut dec_ms_runs = Vec::with_capacity(runs);

        for run in 0..runs {
            let encrypt_start = Instant::now();
            for index in 0..file_count {
                let plain = plain_root.join(format!("small-{index:04}.bin"));
                let vault = cipher_root.join(format!("small-{index:04}.bin.fv"));
                run_ok_status(
                    env.cmd().args([
                        "vault",
                        "enc",
                        plain.to_str().unwrap(),
                        "-o",
                        vault.to_str().unwrap(),
                        "--password",
                        "perf-secret",
                        "--chunk-size",
                        &chunk_size,
                    ]),
                );
            }
            let encrypt_elapsed = encrypt_start.elapsed();

            let decrypt_start = Instant::now();
            for index in 0..file_count {
                let vault = cipher_root.join(format!("small-{index:04}.bin.fv"));
                let out = out_root.join(format!("small-{index:04}.bin"));
                run_ok_status(
                    env.cmd().args([
                        "vault",
                        "dec",
                        vault.to_str().unwrap(),
                        "-o",
                        out.to_str().unwrap(),
                        "--password",
                        "perf-secret",
                    ]),
                );
            }
            let decrypt_elapsed = decrypt_start.elapsed();

            let total_bytes = file_count * size_bytes;
            let enc_ops_s = ops_per_sec(file_count, encrypt_elapsed);
            let dec_ops_s = ops_per_sec(file_count, decrypt_elapsed);
            let enc_mib_s = throughput_mib_per_sec(total_bytes, encrypt_elapsed);
            let dec_mib_s = throughput_mib_per_sec(total_bytes, decrypt_elapsed);
            enc_ops_runs.push(enc_ops_s);
            dec_ops_runs.push(dec_ops_s);
            enc_mib_runs.push(enc_mib_s);
            dec_mib_runs.push(dec_mib_s);
            enc_ms_runs.push(encrypt_elapsed.as_secs_f64() * 1000.0);
            dec_ms_runs.push(decrypt_elapsed.as_secs_f64() * 1000.0);

            eprintln!(
                "perf: filevault small-batch run={} bucket_kib={} count={} size_bytes={} chunk_size={} enc_ms={} enc_ops_s={:.2} enc_mib_s={:.2} dec_ms={} dec_ops_s={:.2} dec_mib_s={:.2}",
                run + 1,
                kib,
                file_count,
                size_bytes,
                chunk_size,
                encrypt_elapsed.as_millis(),
                enc_ops_s,
                enc_mib_s,
                decrypt_elapsed.as_millis(),
                dec_ops_s,
                dec_mib_s
            );

            cleanup_dir(&cipher_root);
            cleanup_dir(&out_root);
            fs::create_dir_all(&cipher_root).unwrap();
            fs::create_dir_all(&out_root).unwrap();
        }

        let enc_ops_mean = mean(&enc_ops_runs);
        let enc_ops_std = stddev(&enc_ops_runs);
        let dec_ops_mean = mean(&dec_ops_runs);
        let dec_ops_std = stddev(&dec_ops_runs);
        let enc_mib_mean = mean(&enc_mib_runs);
        let enc_mib_std = stddev(&enc_mib_runs);
        let dec_mib_mean = mean(&dec_mib_runs);
        let dec_mib_std = stddev(&dec_mib_runs);
        let enc_ms_mean = mean(&enc_ms_runs);
        let enc_ms_std = stddev(&enc_ms_runs);
        let dec_ms_mean = mean(&dec_ms_runs);
        let dec_ms_std = stddev(&dec_ms_runs);

        eprintln!(
            "perf: filevault small-batch summary runs={} bucket_kib={} count={} size_bytes={} chunk_size={} enc_ops_mean={:.2} enc_ops_std={:.2} enc_mib_mean={:.2} enc_mib_std={:.2} dec_ops_mean={:.2} dec_ops_std={:.2} dec_mib_mean={:.2} dec_mib_std={:.2} enc_ms_mean={:.1} enc_ms_std={:.1} dec_ms_mean={:.1} dec_ms_std={:.1}",
            runs,
            kib,
            file_count,
            size_bytes,
            chunk_size,
            enc_ops_mean,
            enc_ops_std,
            enc_mib_mean,
            enc_mib_std,
            dec_ops_mean,
            dec_ops_std,
            dec_mib_mean,
            dec_mib_std,
            enc_ms_mean,
            enc_ms_std,
            dec_ms_mean,
            dec_ms_std
        );

        if let Some(min_ops) = env_u64("XUN_TEST_FILEVAULT_SMALL_MIN_OPS_S") {
            assert!(
                enc_ops_mean >= min_ops as f64,
                "small encrypt ops mean {:.2} < {} ops/s",
                enc_ops_mean,
                min_ops
            );
        }
        if file_count > 0 {
            let last_index = file_count.saturating_sub(1);
            assert!(plain_root.join(format!("small-{last_index:04}.bin")).exists());
        }
        cleanup_dir(&bucket_root);
    }
    cleanup_dir(&work);
}
