# Alias 性能基线

- 基线目录：`D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_193029`
- Commit：`5b6fc4372ca57985651a96bc8b76d2ba37dca80a-dirty`
- 采集时间：`2026-03-19 19:39:20`
- 基线命令：`cargo test --test test_alias perf_ --features alias -- --nocapture --test-threads=1`
- 轮次：5 次，串行执行，避免并发抖动
- 统计口径：以 `median` 作为后续优化对比主指标，`min/mean/max` 作为波动参考

## 运行文件

- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_193029\run1.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_193029\run2.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_193029\run3.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_193029\run4.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_193029\run5.txt`

## 整体耗时

| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| total runtime (s) | 70.11 | 70.25 | 70.99 | 73.67 | 68.57 | 68.57 | 70.25 | 70.72 | 73.67 |

## 指标明细

| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| app sync 12 aliases | 26 | 26 | 25 | 26 | 27 | 25 | 26 | 26 | 27 |
| batch add 200 aliases | 6856 | 6889 | 7362 | 7359 | 6939 | 6856 | 6939 | 7081 | 7362 |
| batch app add 20 | 577 | 579 | 576 | 892 | 588 | 576 | 579 | 642.4 | 892 |
| export 200 aliases | 42 | 28 | 27 | 29 | 28 | 27 | 28 | 30.8 | 42 |
| import 200 aliases | 1155 | 895 | 1062 | 881 | 1004 | 881 | 1004 | 999.4 | 1155 |
| filtered add in 64 aliases | 31 | 32 | 31 | 33 | 31 | 31 | 31 | 31.6 | 33 |
| find in 200 aliases | 28 | 30 | 27 | 42 | 28 | 27 | 28 | 31 | 42 |
| force overwrite in 64 aliases | 32 | 33 | 32 | 33 | 35 | 32 | 33 | 33 | 35 |
| import force duplicates 32 aliases | 82 | 74 | 79 | 86 | 76 | 74 | 79 | 79.4 | 86 |
| mixed import shell=64 app=12 | 226 | 514 | 251 | 372 | 338 | 226 | 338 | 340.2 | 514 |
| import skip duplicates 32 aliases | 71 | 71 | 69 | 84 | 72 | 69 | 71 | 73.4 | 84 |
| ls 200 aliases | 31 | 34 | 29 | 29 | 28 | 28 | 29 | 30.2 | 34 |
| ls --json 200 aliases | 28 | 25 | 26 | 26 | 26 | 25 | 26 | 26.2 | 28 |
| single add | 28 | 28 | 28 | 29 | 28 | 28 | 28 | 28.2 | 29 |
| rm in 64 aliases | 29 | 32 | 30 | 29 | 30 | 29 | 30 | 30 | 32 |
| sync 200 aliases | 304 | 311 | 331 | 299 | 299 | 299 | 304 | 308.8 | 331 |
| sync idempotent first | 300 | 298 | 320 | 351 | 304 | 298 | 304 | 314.6 | 351 |
| sync idempotent second | 301 | 304 | 295 | 295 | 295 | 295 | 295 | 298 | 304 |
| sync with 64 aliases and 24 orphans | 151 | 188 | 187 | 180 | 180 | 151 | 180 | 177.2 | 188 |

## 与上一版基线对比

- 上一版基线目录：`D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_104920`
- 对比口径：`median`

| 指标 | Prev Median | Current Median | Delta(ms) | Delta(%) | 趋势 |
| --- | ---: | ---: | ---: | ---: | --- |
| app sync 12 aliases | 24 | 26 | 2 | 8.33% | 更慢 |
| batch add 200 aliases | 32949 | 6939 | -26010 | -78.94% | 更快 |
| batch app add 20 | 611 | 579 | -32 | -5.24% | 更快 |
| export 200 aliases | 28 | 28 | 0 | 0% | 持平 |
| import 200 aliases | 827 | 1004 | 177 | 21.4% | 更慢 |
| filtered add in 64 aliases | 114 | 31 | -83 | -72.81% | 更快 |
| find in 200 aliases | 28 | 28 | 0 | 0% | 持平 |
| force overwrite in 64 aliases | 121 | 33 | -88 | -72.73% | 更快 |
| import force duplicates 32 aliases | 74 | 79 | 5 | 6.76% | 更慢 |
| mixed import shell=64 app=12 | 353 | 338 | -15 | -4.25% | 更快 |
| import skip duplicates 32 aliases | 68 | 71 | 3 | 4.41% | 更慢 |
| ls 200 aliases | 29 | 29 | 0 | 0% | 持平 |
| ls --json 200 aliases | 27 | 26 | -1 | -3.7% | 更快 |
| single add | 28 | 28 | 0 | 0% | 持平 |
| rm in 64 aliases | 32 | 30 | -2 | -6.25% | 更快 |
| sync 200 aliases | 289 | 304 | 15 | 5.19% | 更慢 |
| sync idempotent first | 298 | 304 | 6 | 2.01% | 更慢 |
| sync idempotent second | 291 | 295 | 4 | 1.37% | 更慢 |
| sync with 64 aliases and 24 orphans | 177 | 180 | 3 | 1.69% | 更慢 |

## 后续执行约定

- 每次优化后新建一个时间戳目录，例如：`logs/alias-perf/<timestamp>`
- 在新目录中按相同命令连续执行 5 次，并分别写入 `run1.txt` 到 `run5.txt`
- 使用以下命令生成新基线并对比上一版：`powershell -ExecutionPolicy Bypass -File "tools/alias-perf-report.ps1" -Dir "logs/alias-perf/<timestamp>" -PrevDir "D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_193029"`
- 若新基线确认生效，则将该目录视为下一轮优化的对比基线
