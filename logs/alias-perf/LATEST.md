# Alias 性能基线入口

- 当前基线目录：`logs/alias-perf/2026-03-19_202227`
- 当前报告文档：`logs/alias-perf/2026-03-19_202227/baseline.md`
- 当前结构化汇总：`logs/alias-perf/2026-03-19_202227/summary.json`
- 当前对比基线：`logs/alias-perf/2026-03-19_200327`
- 基线命令：`cargo test --test test_alias perf_ --features alias -- --nocapture --test-threads=1`
- 对比主指标：`median`

## 后续优化流程

1. 新建目录：`logs/alias-perf/<timestamp>`
2. 连续执行 5 次相同命令，分别输出到 `run1.txt` ~ `run5.txt`
3. 生成报告并对比上一版：`powershell -ExecutionPolicy Bypass -File "tools/alias-perf-report.ps1" -Dir "logs/alias-perf/<timestamp>" -PrevDir "logs/alias-perf/2026-03-19_202227"`
4. 审阅新目录下的 `baseline.md`
5. 若新结果确认作为新基线，则将这里更新为新的时间戳目录

## 备注

- `logs/alias-perf/2026-03-19_104821` 是早期失败目录，不作为基线使用。
- 当前有效基线以 `logs/alias-perf/2026-03-19_202227` 为准。
