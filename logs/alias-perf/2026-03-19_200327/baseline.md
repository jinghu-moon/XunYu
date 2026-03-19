# Alias 性能基线

- 基线目录：`D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_200327`
- Commit：`a4e640597a37d0f66a6154b2c5efdf996ac9c684-dirty`
- 采集时间：`2026-03-19 20:09:57`
- 基线命令：`cargo test --test test_alias perf_ --features alias -- --nocapture --test-threads=1`
- 轮次：5 次，串行执行，避免并发抖动
- 统计口径：以 `median` 作为后续优化对比主指标，`min/mean/max` 作为波动参考

## 运行文件

- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_200327\run1.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_200327\run2.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_200327\run3.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_200327\run4.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_200327\run5.txt`

## 整体耗时

| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| total runtime (s) | 68.67 | 75.56 | 68.66 | 67.61 | 67.52 | 67.52 | 68.66 | 69.6 | 75.56 |

## 指标明细

| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| app sync 12 aliases | 26 | 27 | 25 | 25 | 24 | 24 | 25 | 25.4 | 27 |
| batch add 200 aliases | 6819 | 7379 | 6970 | 6837 | 6828 | 6819 | 6837 | 6966.6 | 7379 |
| batch app add 20 | 573 | 632 | 565 | 574 | 576 | 565 | 574 | 584 | 632 |
| export 200 aliases | 28 | 29 | 28 | 27 | 28 | 27 | 28 | 28 | 29 |
| import 200 aliases | 875 | 1171 | 985 | 920 | 862 | 862 | 920 | 962.6 | 1171 |
| filtered add in 64 aliases | 31 | 32 | 36 | 31 | 31 | 31 | 31 | 32.2 | 36 |
| find in 200 aliases | 29 | 32 | 28 | 29 | 28 | 28 | 29 | 29.2 | 32 |
| force overwrite in 64 aliases | 31 | 43 | 33 | 31 | 34 | 31 | 33 | 34.4 | 43 |
| import force duplicates 32 aliases | 76 | 76 | 75 | 78 | 85 | 75 | 76 | 78 | 85 |
| mixed import shell=64 app=12 | 416 | 383 | 393 | 507 | 281 | 281 | 393 | 396 | 507 |
| import skip duplicates 32 aliases | 23 | 24 | 25 | 23 | 23 | 23 | 23 | 23.6 | 25 |
| ls 200 aliases | 28 | 30 | 29 | 29 | 28 | 28 | 29 | 28.8 | 30 |
| ls --json 200 aliases | 26 | 33 | 26 | 25 | 26 | 25 | 26 | 27.2 | 33 |
| single add | 29 | 33 | 28 | 28 | 28 | 28 | 28 | 29.2 | 33 |
| rm in 64 aliases | 30 | 34 | 31 | 29 | 31 | 29 | 31 | 31 | 34 |
| sync 200 aliases | 295 | 294 | 285 | 289 | 290 | 285 | 290 | 290.6 | 295 |
| sync idempotent first | 333 | 295 | 292 | 285 | 291 | 285 | 292 | 299.2 | 333 |
| sync idempotent second | 323 | 285 | 282 | 287 | 288 | 282 | 287 | 293 | 323 |
| sync with 64 aliases and 24 orphans | 190 | 121 | 202 | 178 | 225 | 121 | 190 | 183.2 | 225 |

## 与上一版基线对比

- 上一版基线目录：`D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_193029`
- 对比口径：`median`

| 指标 | Prev Median | Current Median | Delta(ms) | Delta(%) | 趋势 |
| --- | ---: | ---: | ---: | ---: | --- |
| app sync 12 aliases | 26 | 25 | -1 | -3.85% | 更快 |
| batch add 200 aliases | 6939 | 6837 | -102 | -1.47% | 更快 |
| batch app add 20 | 579 | 574 | -5 | -0.86% | 更快 |
| export 200 aliases | 28 | 28 | 0 | 0% | 持平 |
| import 200 aliases | 1004 | 920 | -84 | -8.37% | 更快 |
| filtered add in 64 aliases | 31 | 31 | 0 | 0% | 持平 |
| find in 200 aliases | 28 | 29 | 1 | 3.57% | 更慢 |
| force overwrite in 64 aliases | 33 | 33 | 0 | 0% | 持平 |
| import force duplicates 32 aliases | 79 | 76 | -3 | -3.8% | 更快 |
| mixed import shell=64 app=12 | 338 | 393 | 55 | 16.27% | 更慢 |
| import skip duplicates 32 aliases | 71 | 23 | -48 | -67.61% | 更快 |
| ls 200 aliases | 29 | 29 | 0 | 0% | 持平 |
| ls --json 200 aliases | 26 | 26 | 0 | 0% | 持平 |
| single add | 28 | 28 | 0 | 0% | 持平 |
| rm in 64 aliases | 30 | 31 | 1 | 3.33% | 更慢 |
| sync 200 aliases | 304 | 290 | -14 | -4.61% | 更快 |
| sync idempotent first | 304 | 292 | -12 | -3.95% | 更快 |
| sync idempotent second | 295 | 287 | -8 | -2.71% | 更快 |
| sync with 64 aliases and 24 orphans | 180 | 190 | 10 | 5.56% | 更慢 |

## 后续执行约定

- 每次优化后新建一个时间戳目录，例如：`logs/alias-perf/<timestamp>`
- 在新目录中按相同命令连续执行 5 次，并分别写入 `run1.txt` 到 `run5.txt`
- 使用以下命令生成新基线并对比上一版：`powershell -ExecutionPolicy Bypass -File "tools/alias-perf-report.ps1" -Dir "logs/alias-perf/<timestamp>" -PrevDir "D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_200327"`
- 若新基线确认生效，则将该目录视为下一轮优化的对比基线
