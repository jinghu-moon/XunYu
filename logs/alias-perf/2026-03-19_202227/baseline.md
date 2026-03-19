# Alias 性能基线

- 基线目录：`D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_202227`
- Commit：`10d49d6e7136711d91578d1c2d5f2b573329ba55-dirty`
- 采集时间：`2026-03-19 20:28:51`
- 基线命令：`cargo test --test test_alias perf_ --features alias -- --nocapture --test-threads=1`
- 轮次：5 次，串行执行，避免并发抖动
- 统计口径：以 `median` 作为后续优化对比主指标，`min/mean/max` 作为波动参考

## 运行文件

- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_202227\run1.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_202227\run2.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_202227\run3.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_202227\run4.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_202227\run5.txt`

## 整体耗时

| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| total runtime (s) | 70.84 | 70.31 | 67.66 | 68.31 | 67.21 | 67.21 | 68.31 | 68.87 | 70.84 |

## 指标明细

| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| app sync 12 aliases | 25 | 25 | 25 | 25 | 25 | 25 | 25 | 25 | 25 |
| batch add 200 aliases | 6972 | 6978 | 6904 | 7191 | 6849 | 6849 | 6972 | 6978.8 | 7191 |
| batch app add 20 | 861 | 615 | 576 | 628 | 572 | 572 | 615 | 650.4 | 861 |
| export 200 aliases | 28 | 32 | 28 | 31 | 30 | 28 | 30 | 29.8 | 32 |
| import 200 aliases | 947 | 900 | 1038 | 995 | 808 | 808 | 947 | 937.6 | 1038 |
| filtered add in 64 aliases | 40 | 39 | 30 | 32 | 31 | 30 | 32 | 34.4 | 40 |
| find in 200 aliases | 34 | 29 | 28 | 28 | 29 | 28 | 29 | 29.6 | 34 |
| force overwrite in 64 aliases | 36 | 40 | 32 | 32 | 34 | 32 | 34 | 34.8 | 40 |
| import force duplicates 32 aliases | 77 | 71 | 73 | 73 | 73 | 71 | 73 | 73.4 | 77 |
| mixed import shell=64 app=12 | 254 | 398 | 378 | 290 | 234 | 234 | 290 | 310.8 | 398 |
| import skip duplicates 32 aliases | 27 | 25 | 23 | 23 | 22 | 22 | 23 | 24 | 27 |
| ls 200 aliases | 29 | 30 | 28 | 29 | 29 | 28 | 29 | 29 | 30 |
| ls --json 200 aliases | 34 | 25 | 26 | 26 | 25 | 25 | 26 | 27.2 | 34 |
| single add | 32 | 28 | 29 | 29 | 29 | 28 | 29 | 29.4 | 32 |
| rm in 64 aliases | 32 | 32 | 30 | 30 | 30 | 30 | 30 | 30.8 | 32 |
| sync 200 aliases | 290 | 285 | 287 | 282 | 284 | 282 | 285 | 285.6 | 290 |
| sync idempotent first | 286 | 280 | 289 | 278 | 286 | 278 | 286 | 283.8 | 289 |
| sync idempotent second | 279 | 278 | 280 | 277 | 282 | 277 | 279 | 279.2 | 282 |
| sync with 64 aliases and 24 orphans | 154 | 175 | 176 | 185 | 178 | 154 | 176 | 173.6 | 185 |

## 与上一版基线对比

- 上一版基线目录：`D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_200327`
- 对比口径：`median`

| 指标 | Prev Median | Current Median | Delta(ms) | Delta(%) | 趋势 |
| --- | ---: | ---: | ---: | ---: | --- |
| app sync 12 aliases | 25 | 25 | 0 | 0% | 持平 |
| batch add 200 aliases | 6837 | 6972 | 135 | 1.97% | 更慢 |
| batch app add 20 | 574 | 615 | 41 | 7.14% | 更慢 |
| export 200 aliases | 28 | 30 | 2 | 7.14% | 更慢 |
| import 200 aliases | 920 | 947 | 27 | 2.93% | 更慢 |
| filtered add in 64 aliases | 31 | 32 | 1 | 3.23% | 更慢 |
| find in 200 aliases | 29 | 29 | 0 | 0% | 持平 |
| force overwrite in 64 aliases | 33 | 34 | 1 | 3.03% | 更慢 |
| import force duplicates 32 aliases | 76 | 73 | -3 | -3.95% | 更快 |
| mixed import shell=64 app=12 | 393 | 290 | -103 | -26.21% | 更快 |
| import skip duplicates 32 aliases | 23 | 23 | 0 | 0% | 持平 |
| ls 200 aliases | 29 | 29 | 0 | 0% | 持平 |
| ls --json 200 aliases | 26 | 26 | 0 | 0% | 持平 |
| single add | 28 | 29 | 1 | 3.57% | 更慢 |
| rm in 64 aliases | 31 | 30 | -1 | -3.23% | 更快 |
| sync 200 aliases | 290 | 285 | -5 | -1.72% | 更快 |
| sync idempotent first | 292 | 286 | -6 | -2.05% | 更快 |
| sync idempotent second | 287 | 279 | -8 | -2.79% | 更快 |
| sync with 64 aliases and 24 orphans | 190 | 176 | -14 | -7.37% | 更快 |

## 后续执行约定

- 每次优化后新建一个时间戳目录，例如：`logs/alias-perf/<timestamp>`
- 在新目录中按相同命令连续执行 5 次，并分别写入 `run1.txt` 到 `run5.txt`
- 使用以下命令生成新基线并对比上一版：`powershell -ExecutionPolicy Bypass -File "tools/alias-perf-report.ps1" -Dir "logs/alias-perf/<timestamp>" -PrevDir "D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_202227"`
- 若新基线确认生效，则将该目录视为下一轮优化的对比基线
