# xunbak Baseline

生成时间：2026-03-22
运行命令：

```bash
cargo bench --bench xunbak_bench_divan --features xunbak -- --sample-count 3
```

说明：

1. 当前结果仅作为初始对照基线，不代表最终发布性能
2. 样本数为 `3`，优先用于回归对比，不用于严肃统计分析
3. 单位保留 benchmark 原始输出

## Results

| Bench | Mean |
| --- | ---: |
| `header_roundtrip` | `22.59 ns` |
| `blob_write_1kb` | `1.075 us` |
| `blob_write_1mb` | `610.7 us` |
| `blob_write_10mb` | `5.055 ms` |
| `compress_zstd_1mb` | `388.7 us` |
| `backup_100_files` | `8.091 ms` |
| `backup_incremental_10pct` | `5.7 ms` |
| `restore_100_files` | `47.91 ms` |
| `verify_quick` | `324.7 us` |
| `verify_full` | `2.537 ms` |

## Raw Summary

```text
backup_100_files: mean 8.091 ms
backup_incremental_10pct: mean 5.7 ms
blob_write_1kb: mean 1.075 us
blob_write_1mb: mean 610.7 us
blob_write_10mb: mean 5.055 ms
compress_zstd_1mb: mean 388.7 us
header_roundtrip: mean 22.59 ns
restore_100_files: mean 47.91 ms
verify_full: mean 2.537 ms
verify_quick: mean 324.7 us
```
