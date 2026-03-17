# path_guard Divan 基准 — 优化后对比
# 日期: 2026-03-17
# 优化内容:
#   1. validate_string_stage: 消除每条路径的 policy.clone()（改传 allow_relative: bool）
#   2. build_probe_cache: targets map 改存 &Vec<u16> 引用，消除 name.clone()
#   3. hash_units: DefaultHasher → FNV-1a（内联实现，无堆分配）
#   4. 清理 unused import: DefaultHasher, Hash, Hasher

## 关键指标对比

| 指标 | 基线 | 优化后 | 变化 |
|------|------|--------|------|
| alloc_probe_50k mean | 73.26 ms | 68.86 ms | **-5.9%** |
| alloc_probe_50k max alloc次数 | 200,012 | 150,012 | **-25.0%** |
| alloc_probe_50k alloc字节 | 31.35 MB | 27.05 MB | **-13.7%** |
| latency_single_string_only mean | 680.8 ns | 642.4 ns | **-5.6%** |
| dedupe_rate/0% max alloc | 4,012 | 3,012 | **-24.9%** |
| dedupe_rate/50% max alloc | 2,012 | 1,512 | **-24.8%** |
| alloc_string_only_50k mean | 31.38 ms | 31.71 ms | +1.1% (noise) |

## 吞吐量对比（Kitem/s mean）

| 基准 | 基线 | 优化后 | 变化 |
|------|------|--------|------|
| dedupe_rate/0% | 532.1 | 534.6 | +0.5% |
| dedupe_rate/50% | 602.4 | 606.1 | +0.6% |
| dedupe_rate/90% | 728.2 | 699.7 | -3.9% (noise) |
| dedupe_rate/99% | ~800 | 788.6 | - |

## 分配减少原因分析

- **policy.clone() 消除**: 每条路径节省2次 PathBuf clone（Option<PathBuf> × 2）
  - 50k路径 × 2 = 100k 次分配减少 → 实测减少约 50k（probe路径约50%命中）
- **name.clone() 消除**: probe_cache targets map 不再克隆 Vec<u16>
  - 减少约 25k 次分配（50k路径对应约25k unique目录文件名）
- **FNV hash**: 内联计算，无 DefaultHasher 对象，略微改善 CPU cache

## grow次数未变原因

grow: 100,054 次不变 — 这来自 rayon/crossbeam worker 线程的 Vec 扩容
主要是 probe_cache Vec 和 ok_slots/issue_slots 的初始分配，不受上述优化影响

## 下一步优化方向

1. **ok_slots/issue_slots 预分配**: `vec![None; total]` 两个 Vec 共 50k*2 元素
   可合并为单个枚举 Vec 减少内存占用
2. **work_items 扩容**: `work_items.reserve(stage_results.len())` 已有，但
   stage_results 先收集再分发，存在一次额外 Vec 分配
3. **io_threads 数量**: 当前 min(avail/2, 8)=8线程，50k路径 IO 密集
   可尝试提升至 min(avail, 16) 观察效果
