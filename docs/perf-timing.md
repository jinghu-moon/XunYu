# FileVault v13 Timing & Performance Baseline (Release)

## Timing (Release build, 64 MiB, chunk=262144, workers=1)

Command:
- XUN_FILEVAULT_TIMING=1
- XUN_FILEVAULT_WORKERS=1
- xun vault enc/dec (64 MiB)

Results:
- encrypt total_ms=514
  - validate options: 0
  - prepare sidecars: 0
  - build headers: 106
  - open/write header: 1
  - stream frames: 405
  - payload digest: 0
  - footer write: 0
  - commit rename: 0
- decrypt total_ms=448
  - parse: 0
  - unlock: 105
  - open output: 0
  - stream frames: 341
  - verify footer: 0
  - commit rename: 0

## Timing (Release build, 64 MiB, chunk=262144, workers=4)

Command:
- XUN_FILEVAULT_TIMING=1
- XUN_FILEVAULT_WORKERS=4
- xun vault enc/dec (64 MiB)

Results:
- encrypt total_ms=285
  - validate options: 0
  - prepare sidecars: 0
  - build headers: 103
  - open/write header: 1
  - stream frames: 180
  - payload digest: 0
  - footer write: 0
  - commit rename: 0
- decrypt total_ms=239
  - parse: 0
  - unlock: 102
  - open output: 0
  - stream frames: 135
  - verify footer: 0
  - commit rename: 0

## Performance (Release, --ignored)

Note: unstated worker counts use default `XUN_FILEVAULT_WORKERS=16`.

### Throughput (64 MiB, chunk=262144, warmup=1, runs=3, mean±std)
- workers=1: encrypt 123.26±0.19 MiB/s (519.2±0.8 ms), decrypt 137.49±0.52 MiB/s (465.5±1.8 ms)
- workers=4: encrypt 215.27±1.03 MiB/s (297.3±1.4 ms), decrypt 248.34±3.72 MiB/s (257.7±3.8 ms)
- workers=16 (default): encrypt 225.17±1.29 MiB/s (284.2±1.6 ms), decrypt 250.83±5.13 MiB/s (255.2±5.3 ms)

### Resource Usage (128 MiB, sample_ms=50, warmup=1, runs=3, mean±std)
- working_set_peak_bytes: 79,192,064±0 (~75.6 MiB)
- handle_peak_count: 187.7±0.6

### Rewrap vs Re-encrypt (128 MiB, warmup=1, runs=3, mean±std)
- rewrap: 593±3 ms
- re-encrypt: 540±1 ms (236.93±0.48 MiB/s)
- ratio (reencrypt/rewrap): 0.91±0.01

### Large Files (chunk=262144, last full run with auto workers)
- 2048 MiB: enc 121.56 MiB/s (16847 ms), dec 149.92 MiB/s (13660 ms)
- 4096 MiB: enc 114.56 MiB/s (35752 ms), dec 132.20 MiB/s (30983 ms)
- 8192 MiB: enc 146.47 MiB/s (55930 ms), dec 192.52 MiB/s (42552 ms)

### Large Files (parallel comparison, chunk=262144)
- workers=1: 2048 MiB enc 152.44 MiB/s (13434 ms), dec 188.16 MiB/s (10884 ms)
- workers=1: 4096 MiB enc 153.60 MiB/s (26666 ms), dec 162.88 MiB/s (25147 ms)
- workers=1: 8192 MiB enc 138.85 MiB/s (59000 ms), dec 191.43 MiB/s (42794 ms)
- workers=4: 2048 MiB enc 285.96 MiB/s (7161 ms), dec 450.67 MiB/s (4544 ms)
- workers=4: 4096 MiB enc 269.76 MiB/s (15183 ms), dec 380.88 MiB/s (10754 ms)
- workers=4: 8192 MiB enc 330.04 MiB/s (24821 ms), dec 368.95 MiB/s (22203 ms)

### Large Files (chunk sweep, warmup=1, runs=3, mean±std)

Workers=1:

| Size (MiB) | Chunk | Enc MiB/s | Dec MiB/s | Enc ms | Dec ms |
| --- | --- | --- | --- | --- | --- |
| 2048 | 65536 | 100.96±31.40 | 167.50±26.81 | 22014.6±8343.9 | 12461.2±2194.5 |
| 2048 | 262144 | 136.48±18.63 | 171.27±21.57 | 15185.6±1974.8 | 12095.4±1635.8 |
| 2048 | 1048576 | 122.57±14.42 | 144.54±28.33 | 16854.5±1858.5 | 14571.2±3086.9 |
| 4096 | 65536 | 99.28±13.44 | 149.09±1.01 | 41755.6±5540.5 | 27474.9±187.4 |
| 4096 | 262144 | 131.97±33.59 | 147.83±37.58 | 32641.8±9494.7 | 28898.8±7095.0 |
| 4096 | 1048576 | 152.15±24.03 | 167.95±21.74 | 27407.6±4638.7 | 24681.8±3408.6 |
| 8192 | 65536 | 104.62±6.40 | 168.25±23.24 | 78501.0±4904.7 | 49356.9±7254.8 |
| 8192 | 262144 | 139.51±6.86 | 172.76±23.35 | 58814.1±2908.4 | 48051.0±7011.1 |
| 8192 | 1048576 | 151.59±20.66 | 153.02±13.84 | 54674.1±6998.9 | 53822.7±4757.6 |

Workers=4:

| Size (MiB) | Chunk | Enc MiB/s | Dec MiB/s | Enc ms | Dec ms |
| --- | --- | --- | --- | --- | --- |
| 2048 | 65536 | 221.95±4.24 | 484.54±0.67 | 9229.7±177.6 | 4226.7±5.9 |
| 2048 | 262144 | 395.52±3.04 | 508.63±1.54 | 5178.2±39.8 | 4026.5±12.2 |
| 2048 | 1048576 | 467.45±2.14 | 476.85±2.66 | 4381.2±20.0 | 4294.9±24.0 |
| 4096 | 65536 | 219.26±8.99 | 491.50±4.11 | 18701.4±761.5 | 8334.1±69.4 |
| 4096 | 262144 | 318.27±70.60 | 474.46±47.65 | 13255.8±2612.8 | 8694.3±917.9 |
| 4096 | 1048576 | 476.64±6.67 | 470.87±16.98 | 8594.7±121.2 | 8706.5±317.5 |
| 8192 | 65536 | 217.07±6.95 | 466.42±28.12 | 37764.3±1217.1 | 17605.0±1026.9 |
| 8192 | 262144 | 401.41±5.26 | 494.72±25.05 | 20410.5±265.2 | 16587.9±856.2 |
| 8192 | 1048576 | 432.81±19.27 | 445.06±10.18 | 18953.0±864.6 | 18413.0±417.2 |

### Small Files Batch (count=64, chunk=65536, warmup=1, runs=3, mean±std)
- 1 KiB: enc 7.22±0.42 ops/s (0.01±0.00 MiB/s), dec 7.27±0.44 ops/s (0.01±0.00 MiB/s)
- 2 KiB: enc 7.48±0.03 ops/s (0.01±0.00 MiB/s), dec 7.54±0.03 ops/s (0.01±0.00 MiB/s)
- 4 KiB: enc 7.42±0.22 ops/s (0.03±0.00 MiB/s), dec 7.47±0.14 ops/s (0.03±0.00 MiB/s)
- 8 KiB: enc 7.36±0.03 ops/s (0.06±0.00 MiB/s), dec 7.34±0.24 ops/s (0.06±0.00 MiB/s)
- 16 KiB: enc 7.46±0.11 ops/s (0.12±0.00 MiB/s), dec 7.49±0.06 ops/s (0.12±0.00 MiB/s)
- 32 KiB: enc 7.37±0.38 ops/s (0.23±0.01 MiB/s), dec 7.53±0.21 ops/s (0.24±0.01 MiB/s)
- 64 KiB: enc 7.61±0.12 ops/s (0.48±0.01 MiB/s), dec 7.67±0.04 ops/s (0.48±0.00 MiB/s)
