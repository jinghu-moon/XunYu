# FileVault v13 Performance Compare Baseline (Release)

## Context

Three runs are compared:
- Run A (before optimization): release perf run after large/small benchmarks were added, before encrypt/decrypt single-pass optimizations.
- Run B (after single-pass): release perf run after single-pass digest and merged verify changes.
- Run C (parallel workers=4): release perf run with `XUN_FILEVAULT_WORKERS=4` (throughput only).

Both runs used:
- `cargo test --release --test special_filevault_performance --features crypt -- --ignored --nocapture`
- `chunk_size=262144` (default)
- small-batch: `count=256`, buckets `1/2/4/8/16/32/64 KiB`
- large list: `2048/4096/8192 MiB`

Note: results are machine-load sensitive; use trends, not absolutes.

## Summary (Key Deltas)

| Metric | Run A | Run B | Run C | Delta A->B | Delta B->C |
| --- | --- | --- | --- | --- | --- |
| 64 MiB encrypt throughput | 57.44 MiB/s | 61.79 MiB/s | 78.80 MiB/s | +4.35 MiB/s (+7.6%) | +17.01 MiB/s (+27.5%) |
| 64 MiB decrypt throughput | 121.93 MiB/s | 136.47 MiB/s | 248.75 MiB/s | +14.54 MiB/s (+11.9%) | +112.28 MiB/s (+82.3%) |
| 64 MiB encrypt time | 1114 ms | 1035 ms | 812 ms | -79 ms (-7.1%) | -223 ms (-21.5%) |
| 64 MiB decrypt time | 524 ms | 468 ms | 257 ms | -56 ms (-10.7%) | -211 ms (-45.1%) |
| working_set_peak_bytes | 79,171,584 | 79,159,296 | - | -12,288 bytes | - |
| handle_peak_count | 172 | 171 | - | -1 | - |
| rewrap time | 623 ms | 616 ms | - | -7 ms | - |
| re-encrypt time | 1085 ms | 1030 ms | - | -55 ms | - |
| reencrypt/rewrap ratio | 1.74 | 1.67 | - | -0.07 | - |

## 64 MiB Throughput (chunk=262144)

| Metric | Run A | Run B | Run C |
| --- | --- | --- | --- |
| encrypt MiB/s | 57.44 | 61.79 | 78.80 |
| decrypt MiB/s | 121.93 | 136.47 | 248.75 |
| encrypt ms | 1114 | 1035 | 812 |
| decrypt ms | 524 | 468 | 257 |

## Resource Usage (128 MiB, sample_ms=50)

| Metric | Run A | Run B |
| --- | --- | --- |
| working_set_peak_bytes | 79,171,584 | 79,159,296 |
| handle_peak_count | 172 | 171 |

## Rewrap vs Re-encrypt (128 MiB)

| Metric | Run A | Run B |
| --- | --- | --- |
| rewrap ms | 623 | 616 |
| re-encrypt ms | 1085 | 1030 |
| reencrypt throughput MiB/s | 117.88 | 124.24 |
| ratio | 1.74 | 1.67 |

## Large Files (chunk=262144)

| Size | Run A enc MiB/s | Run B enc MiB/s | Run A dec MiB/s | Run B dec MiB/s |
| --- | --- | --- | --- | --- |
| 2048 MiB | 143.36 | 121.56 | 171.74 | 149.92 |
| 4096 MiB | 131.37 | 114.56 | 114.45 | 132.20 |
| 8192 MiB | 124.10 | 146.47 | 142.12 | 192.52 |

## Large Files (parallel comparison, chunk=262144)

| Size | workers=1 enc MiB/s | workers=4 enc MiB/s | workers=1 dec MiB/s | workers=4 dec MiB/s |
| --- | --- | --- | --- | --- |
| 2048 MiB | 152.44 | 285.96 | 188.16 | 450.67 |
| 4096 MiB | 153.60 | 269.76 | 162.88 | 380.88 |
| 8192 MiB | 138.85 | 330.04 | 191.43 | 368.95 |

## Large Files (chunk sweep, warmup=1, runs=3, mean±std)

Workers=1:

| Size (MiB) | Chunk | Enc MiB/s | Dec MiB/s |
| --- | --- | --- | --- |
| 2048 | 65536 | 100.96±31.40 | 167.50±26.81 |
| 2048 | 262144 | 136.48±18.63 | 171.27±21.57 |
| 2048 | 1048576 | 122.57±14.42 | 144.54±28.33 |
| 4096 | 65536 | 99.28±13.44 | 149.09±1.01 |
| 4096 | 262144 | 131.97±33.59 | 147.83±37.58 |
| 4096 | 1048576 | 152.15±24.03 | 167.95±21.74 |
| 8192 | 65536 | 104.62±6.40 | 168.25±23.24 |
| 8192 | 262144 | 139.51±6.86 | 172.76±23.35 |
| 8192 | 1048576 | 151.59±20.66 | 153.02±13.84 |

Workers=4:

| Size (MiB) | Chunk | Enc MiB/s | Dec MiB/s |
| --- | --- | --- | --- |
| 2048 | 65536 | 221.95±4.24 | 484.54±0.67 |
| 2048 | 262144 | 395.52±3.04 | 508.63±1.54 |
| 2048 | 1048576 | 467.45±2.14 | 476.85±2.66 |
| 4096 | 65536 | 219.26±8.99 | 491.50±4.11 |
| 4096 | 262144 | 318.27±70.60 | 474.46±47.65 |
| 4096 | 1048576 | 476.64±6.67 | 470.87±16.98 |
| 8192 | 65536 | 217.07±6.95 | 466.42±28.12 |
| 8192 | 262144 | 401.41±5.26 | 494.72±25.05 |
| 8192 | 1048576 | 432.81±19.27 | 445.06±10.18 |

## Small Files Batch (count=256, chunk=65536)

| Size | Run A enc ops/s | Run B enc ops/s | Run A dec ops/s | Run B dec ops/s |
| --- | --- | --- | --- | --- |
| 1 KiB | 7.38 | 7.25 | 7.24 | 7.31 |
| 2 KiB | 6.72 | 7.24 | 7.17 | 7.43 |
| 4 KiB | 7.34 | 7.53 | 7.13 | 7.48 |
| 8 KiB | 7.05 | 7.05 | 7.51 | 7.76 |
| 16 KiB | 7.35 | 7.71 | 7.32 | 7.72 |
| 32 KiB | 7.17 | 7.66 | 6.91 | 7.47 |
| 64 KiB | 6.41 | 7.02 | 6.76 | 7.69 |

## Stability Run (Run D, workers=16, warmup=1, runs=3)

### 64 MiB Throughput (chunk=262144)

| Metric | Result |
| --- | --- |
| encrypt MiB/s | 225.17±1.29 |
| decrypt MiB/s | 250.83±5.13 |
| encrypt ms | 284.2±1.6 |
| decrypt ms | 255.2±5.3 |

### Resource Usage (128 MiB, sample_ms=50)

| Metric | Result |
| --- | --- |
| working_set_peak_bytes | 79,192,064±0 |
| handle_peak_count | 187.7±0.6 |

### Rewrap vs Re-encrypt (128 MiB)

| Metric | Result |
| --- | --- |
| rewrap ms | 593±3 |
| re-encrypt ms | 540±1 |
| re-encrypt throughput MiB/s | 236.93±0.48 |
| ratio (reencrypt/rewrap) | 0.91±0.01 |

### Small Files Batch (count=64, chunk=65536)

| Size | Enc ops/s | Dec ops/s | Enc MiB/s | Dec MiB/s |
| --- | --- | --- | --- | --- |
| 1 KiB | 7.22±0.42 | 7.27±0.44 | 0.01±0.00 | 0.01±0.00 |
| 2 KiB | 7.48±0.03 | 7.54±0.03 | 0.01±0.00 | 0.01±0.00 |
| 4 KiB | 7.42±0.22 | 7.47±0.14 | 0.03±0.00 | 0.03±0.00 |
| 8 KiB | 7.36±0.03 | 7.34±0.24 | 0.06±0.00 | 0.06±0.00 |
| 16 KiB | 7.46±0.11 | 7.49±0.06 | 0.12±0.00 | 0.12±0.00 |
| 32 KiB | 7.37±0.38 | 7.53±0.21 | 0.23±0.01 | 0.24±0.01 |
| 64 KiB | 7.61±0.12 | 7.67±0.04 | 0.48±0.01 | 0.48±0.00 |

## Change Attribution

Primary changes between Run A and Run B:
- Encrypt: payload digest computed during streaming (no second pass).
- Decrypt: integrity verification merged into decrypt stream (no pre-pass).
- Buffer reuse for ciphertext vectors.

Primary changes between Run B and Run C:
- Parallel frame processing with bounded in-flight queue (`XUN_FILEVAULT_WORKERS=4`).
