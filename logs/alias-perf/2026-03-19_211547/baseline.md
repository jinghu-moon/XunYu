# Alias 性能基线

- 基线目录：`D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_211547`
- Commit：`63d64327f6f5f6d388ca9a39977a1efbb4849a7f-dirty`
- 采集时间：`2026-03-19 21:21:03`
- 基线命令：`cargo test --test test_alias perf_ --features alias -- --nocapture --test-threads=1`
- 轮次：5 次，串行执行，避免并发抖动
- 统计口径：以 `median` 作为后续优化对比主指标，`min/mean/max` 作为波动参考

## 运行文件

- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_211547\run1.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_211547\run2.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_211547\run3.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_211547\run4.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_211547\run5.txt`

## 整体耗时

| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| total runtime (s) | 62.35 | 62.01 | 62.51 | 61.88 | 63.87 | 61.88 | 62.35 | 62.52 | 63.87 |

## 指标明细

| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| app sync 12 aliases | 29 | 24 | 23 | 24 | 24 | 23 | 24 | 24.8 | 29 |
| batch add 200 aliases | 6252 | 6239 | 6250 | 6217 | 6319 | 6217 | 6250 | 6255.4 | 6319 |
| batch app add 20 | 549 | 587 | 540 | 539 | 557 | 539 | 549 | 554.4 | 587 |
| export 200 aliases | 28 | 27 | 27 | 27 | 27 | 27 | 27 | 27.2 | 28 |
| import 200 aliases | 812 | 678 | 818 | 795 | 949 | 678 | 812 | 810.4 | 949 |
| filtered add in 64 aliases | 29 | 30 | 29 | 31 | 29 | 29 | 29 | 29.6 | 31 |
| find in 200 aliases | 28 | 27 | 28 | 28 | 29 | 27 | 28 | 28 | 29 |
| force overwrite in 64 aliases | 30 | 30 | 31 | 31 | 49 | 30 | 31 | 34.2 | 49 |
| import force duplicates 32 aliases | 72 | 71 | 73 | 74 | 80 | 71 | 73 | 74 | 80 |
| mixed import shell=64 app=12 | 310 | 308 | 294 | 227 | 248 | 227 | 294 | 277.4 | 310 |
| import skip duplicates 32 aliases | 23 | 22 | 23 | 22 | 27 | 22 | 23 | 23.4 | 27 |
| ls 200 aliases | 29 | 28 | 28 | 28 | 35 | 28 | 28 | 29.6 | 35 |
| ls --json 200 aliases | 25 | 26 | 27 | 26 | 25 | 25 | 26 | 25.8 | 27 |
| single add | 28 | 27 | 27 | 27 | 28 | 27 | 27 | 27.4 | 28 |
| rm in 64 aliases | 29 | 29 | 27 | 30 | 27 | 27 | 29 | 28.4 | 30 |
| sync 200 aliases | 274 | 280 | 278 | 283 | 281 | 274 | 280 | 279.2 | 283 |
| sync idempotent first | 276 | 277 | 280 | 276 | 279 | 276 | 277 | 277.6 | 280 |
| sync idempotent second | 271 | 278 | 276 | 278 | 277 | 271 | 277 | 276 | 278 |
| sync with 64 aliases and 24 orphans | 170 | 130 | 178 | 222 | 214 | 130 | 178 | 182.8 | 222 |

## 与上一版基线对比

- 上一版基线目录：`D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_202227`
- 对比口径：`median`

| 指标 | Prev Median | Current Median | Delta(ms) | Delta(%) | 趋势 |
| --- | ---: | ---: | ---: | ---: | --- |
| app sync 12 aliases | 25 | 24 | -1 | -4% | 更快 |
| batch add 200 aliases | 6972 | 6250 | -722 | -10.36% | 更快 |
| batch app add 20 | 615 | 549 | -66 | -10.73% | 更快 |
| export 200 aliases | 30 | 27 | -3 | -10% | 更快 |
| import 200 aliases | 947 | 812 | -135 | -14.26% | 更快 |
| filtered add in 64 aliases | 32 | 29 | -3 | -9.38% | 更快 |
| find in 200 aliases | 29 | 28 | -1 | -3.45% | 更快 |
| force overwrite in 64 aliases | 34 | 31 | -3 | -8.82% | 更快 |
| import force duplicates 32 aliases | 73 | 73 | 0 | 0% | 持平 |
| mixed import shell=64 app=12 | 290 | 294 | 4 | 1.38% | 更慢 |
| import skip duplicates 32 aliases | 23 | 23 | 0 | 0% | 持平 |
| ls 200 aliases | 29 | 28 | -1 | -3.45% | 更快 |
| ls --json 200 aliases | 26 | 26 | 0 | 0% | 持平 |
| single add | 29 | 27 | -2 | -6.9% | 更快 |
| rm in 64 aliases | 30 | 29 | -1 | -3.33% | 更快 |
| sync 200 aliases | 285 | 280 | -5 | -1.75% | 更快 |
| sync idempotent first | 286 | 277 | -9 | -3.15% | 更快 |
| sync idempotent second | 279 | 277 | -2 | -0.72% | 更快 |
| sync with 64 aliases and 24 orphans | 176 | 178 | 2 | 1.14% | 更慢 |

## 后续执行约定

- 每次优化后新建一个时间戳目录，例如：`logs/alias-perf/<timestamp>`
- 在新目录中按相同命令连续执行 5 次，并分别写入 `run1.txt` 到 `run5.txt`
- 使用以下命令生成新基线并对比上一版：`powershell -ExecutionPolicy Bypass -File "tools/alias-perf-report.ps1" -Dir "logs/alias-perf/<timestamp>" -PrevDir "D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_211547"`
- 若新基线确认生效，则将该目录视为下一轮优化的对比基线
