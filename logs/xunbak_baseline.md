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

## 2026-03-26 Codec Expansion Baseline

运行命令：

```bash
cargo bench --bench xunbak_bench_divan --features xunbak -- --sample-count 3
```

新增结果：

| Bench | Mean |
| --- | ---: |
| `compress_lz4_1mb` | `762.3 us` |
| `compress_lzma2_1mb` | `109.8 ms` |
| `compress_deflate_1mb` | `1.077 ms` |
| `compress_bzip2_1mb` | `111.9 ms` |
| `compress_ppmd_text_corpus` | `4.669 ms` |
| `backup_100_files` | `43.79 ms` |
| `backup_100_files_lz4` | `38.94 ms` |
| `restore_100_files` | `54.89 ms` |
| `restore_100_files_lz4` | `52.92 ms` |

摘要：

1. `LZ4` create 吞吐优于默认 `zstd(1)` 基线
2. `restore_100_files_lz4` 与默认 `zstd(1)` 接近，当前样本下没有数量级优势
3. `LZMA2 / BZIP2` 明显更慢，更适合作为高压缩比选项而不是默认值
4. `PPMD` 在文本样本上的 1 MiB 压缩耗时明显低于 `LZMA2 / BZIP2`

## 2026-03-26 Large Text Baseline

运行命令：

```bash
cargo bench --bench xunbak_bench_divan --features xunbak -- --sample-count 1 large_text
```

样本说明：

1. 单文件 `large.txt`
2. 原始大小约 `33 MiB`
3. 内容为重复文本语料，主要用于观察大文件分块 / restore 流式路径
4. `sample-count = 1`，只作为 spot baseline，不用于严肃统计

| Bench | Mean | Max Alloc |
| --- | ---: | ---: |
| `backup_large_text_lz4` | `44.62 ms` | `25.66 MB` |
| `backup_large_text_ppmd` | `196.7 ms` | `34.07 MB` |
| `backup_large_text_lzma2` | `5.78 s` | `114 MB` |
| `restore_large_text_lz4` | `54.46 ms` | `8.391 MB` |
| `restore_large_text_ppmd` | `1.437 s` | `16.78 MB` |
| `restore_large_text_lzma2` | `39.94 ms` | `8.472 MB` |

摘要：

1. 大文件 create 路径中，`LZ4` 仍是吞吐优先选项
2. `PPMD` 在大文本文件上 create 明显慢于 `LZ4`，但仍远快于 `LZMA2`
3. `LZMA2` 大文件 create 成本最高，应继续定位为慢速高压缩归档模式
4. restore 路径的 `max alloc` 没有随 `33 MiB` 原始大小线性放大，和流式复制测试结论一致
