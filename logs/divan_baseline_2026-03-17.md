# path_guard Divan 基准基线
# 日期: 2026-03-17
# 构建: bench profile (optimized)
# 环境: Intel i7-13xxx, 20 线程, Windows 11, Timer precision: 100 ns

## 吞吐量（50k 路径）

| 基准 | fastest | median | mean | 吞吐 |
|------|---------|--------|------|------|
| throughput_string_only_50k | - | - | ~31 ms | ~1.6M item/s |
| throughput_with_probe_50k (50% exist) | - | - | ~73 ms | ~680K item/s |
| throughput_all_existing_50k | - | - | - | - |
| throughput_all_missing_50k | - | - | - | - |
| throughput_owned_50k | - | - | - | - |

## 延迟（单条路径）

| 基准 | fastest | median | mean |
|------|---------|--------|------|
| latency_single_existing | 24.79 µs | 34.34 µs | 36.62 µs |
| latency_single_missing | 6.298 µs | 6.598 µs | 6.986 µs |
| latency_single_string_only | 654 ns | 672.7 ns | 680.8 ns |
| latency_first_call_warmup | 3.997 µs | 4.197 µs | 9.657 µs |

## 内存分配（50k 路径）

| 基准 | alloc次数 | alloc字节 | grow次数 |
|------|-----------|-----------|----------|
| alloc_probe_50k | 251,671 | 31.35 MB | 100,054 |
| alloc_string_only_50k | 50,009 | 14.01 MB | 28 |
| alloc_single_path | 5 | 697 B | 2 |

## 扩展性 - 重复率（1000路径）

| dup_pct | fastest | median | mean | 吞吐 |
|---------|---------|--------|------|------|
| 0% | 1.642 ms | 1.821 ms | 1.879 ms | 532K item/s |
| 50% | 1.433 ms | 1.585 ms | 1.659 ms | 602K item/s |
| 90% | 1.181 ms | 1.327 ms | 1.373 ms | 728K item/s |
| 99% | ~1.0 ms | - | - | ~1M item/s |

## 关键观察

1. **alloc_probe_50k: 251k 次分配，31MB** — 核心瓶颈，probe 路径分配过多
2. **grow: 100,054 次** — Vec 扩容严重，预分配不足
3. **latency_single_existing: 36µs mean** — 单条 IO 探测延迟高
4. **alloc_string_only: 50k 次分配** — 每条路径至少1次 String alloc，可优化
5. **重复率越高越快** — dedupe 有效，但 0% 重复时分配仍然高

## 优化目标

- alloc_probe_50k mean: 73ms → <50ms (目标 -30%)
- alloc次数: 251k → <150k (目标 -40%)
- grow次数: 100k → <10k (目标 -90%，预分配)
- latency_single_existing: 36µs → <25µs
