# Alias 性能基线

- 基线目录：`D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_104920`
- Commit：`764426b180b8c2035f4d36cc4970c9d07ef2fef2`
- 采集时间：`2026-03-19 19:13:17`
- 基线命令：`cargo test --test test_alias perf_ --features alias -- --nocapture --test-threads=1`
- 轮次：5 次，串行执行，避免并发抖动
- 统计口径：以 `median` 作为后续优化对比主指标，`min/mean/max` 作为波动参考

## 运行文件

- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_104920\run1.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_104920\run2.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_104920\run3.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_104920\run4.txt`
- `D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_104920\run5.txt`

## 整体耗时

| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| total runtime (s) | 268.63 | 270.72 | 268.46 | 266.79 | 273.23 | 266.79 | 268.63 | 269.57 | 273.23 |

## 指标明细

| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| app sync 12 aliases | 24 | 24 | 25 | 24 | 26 | 24 | 24 | 24.6 | 26 |
| batch add 200 aliases | 32762 | 32949 | 33204 | 32696 | 33818 | 32696 | 32949 | 33085.8 | 33818 |
| batch app add 20 | 611 | 613 | 692 | 599 | 605 | 599 | 611 | 624 | 692 |
| export 200 aliases | 28 | 26 | 29 | 28 | 33 | 26 | 28 | 28.8 | 33 |
| import 200 aliases | 877 | 820 | 827 | 800 | 1038 | 800 | 827 | 872.4 | 1038 |
| filtered add in 64 aliases | 114 | 114 | 112 | 113 | 119 | 112 | 114 | 114.4 | 119 |
| find in 200 aliases | 28 | 29 | 28 | 28 | 27 | 27 | 28 | 28 | 29 |
| force overwrite in 64 aliases | 121 | 120 | 117 | 121 | 127 | 117 | 121 | 121.2 | 127 |
| import force duplicates 32 aliases | 74 | 72 | 75 | 74 | 75 | 72 | 74 | 74 | 75 |
| mixed import shell=64 app=12 | 353 | 386 | 307 | 411 | 317 | 307 | 353 | 354.8 | 411 |
| import skip duplicates 32 aliases | 68 | 71 | 68 | 68 | 69 | 68 | 68 | 68.8 | 71 |
| ls 200 aliases | 30 | 29 | 30 | 28 | 28 | 28 | 29 | 29 | 30 |
| ls --json 200 aliases | 27 | 25 | 27 | 25 | 27 | 25 | 27 | 26.2 | 27 |
| single add | 28 | 28 | 28 | 27 | 29 | 27 | 28 | 28 | 29 |
| rm in 64 aliases | 32 | 36 | 31 | 32 | 33 | 31 | 32 | 32.8 | 36 |
| sync 200 aliases | 300 | 299 | 286 | 284 | 289 | 284 | 289 | 291.6 | 300 |
| sync idempotent first | 284 | 298 | 305 | 283 | 301 | 283 | 298 | 294.2 | 305 |
| sync idempotent second | 291 | 284 | 293 | 283 | 313 | 283 | 291 | 292.8 | 313 |
| sync with 64 aliases and 24 orphans | 171 | 175 | 177 | 185 | 181 | 171 | 177 | 177.8 | 185 |

## 后续执行约定

- 每次优化后新建一个时间戳目录，例如：`logs/alias-perf/<timestamp>`
- 在新目录中按相同命令连续执行 5 次，并分别写入 `run1.txt` 到 `run5.txt`
- 使用以下命令生成新基线并对比上一版：`powershell -ExecutionPolicy Bypass -File "tools/alias-perf-report.ps1" -Dir "logs/alias-perf/<timestamp>" -PrevDir "D:\100_Projects\110_Daily\XunYu\logs\alias-perf\2026-03-19_104920"`
- 若新基线确认生效，则将该目录视为下一轮优化的对比基线
