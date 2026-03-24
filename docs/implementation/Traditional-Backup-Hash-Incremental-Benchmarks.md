# 传统 `backup` 哈希增量基线

> 日期：2026-03-24
> 代码状态：本地工作树，已包含共享哈希模块、hash manifest、hash cache、Windows 路径查找归一化
> 平台：Windows
> 二进制：`target/release/xun.exe`

---

## 1. 场景

### 1.1 diff 微基准

- 基准入口：[bak_bench_divan.rs](/D:/100_Projects/110_Daily/XunYu/benches/bak_bench_divan.rs)
- 命令：

```powershell
cargo bench --bench bak_bench_divan -- diff_metadata_500_unchanged diff_hash_500_unchanged_cold diff_hash_500_unchanged_warm
```

- 数据集：
  - `src/**` + `public/**`
  - 共 500 个文件
  - 文件大小约 500 B 到 3000 B

### 1.2 命令级 timing 基线

- 命令：`XUN_BACKUP_TIMING=1 target/release/xun.exe backup -C <fixture> -m v2`
- 数据集：
  - 首次全量后再进行一次全量备份
  - 共 500 文件
  - 其中 50 文件内容变更
  - 450 文件命中 hash cache 并被 hardlink 复用

---

## 2. 基准结果

### 2.1 diff 微基准

| 基准 | median | mean | 说明 |
|---|---:|---:|---|
| `diff_metadata_500_unchanged` | 1.449 ms | 1.465 ms | 旧 `size + mtime` 元数据 diff |
| `diff_hash_500_unchanged_cold` | 1.073 s | 1.074 s | 新 hash diff，删除 hash cache，全部重算 |
| `diff_hash_500_unchanged_warm` | 3.061 ms | 3.075 ms | 新 hash diff，hash cache 全命中 |

结论：

- 冷缓存 hash diff 明显慢于旧元数据 diff，这是预期成本。
- 热缓存 hash diff 与旧元数据 diff 已接近同一数量级。
- 当前设计的性能关键在于 **hash cache 命中率**，不是 hash diff 算法本身。

### 2.2 命令级 timing

来自一次 500 文件、50 文件修改、450 文件复用的真实运行：

| 指标 | 数值 |
|---|---:|
| `scan time` | 112 ms |
| `baseline time` | 0 ms |
| `diff time` | 0 ms |
| `copy time` | 55 ms |
| `total time` | 171 ms |
| `hash_checked_files` | 500 |
| `hash_cache_hits` | 450 |
| `hash_computed_files` | 50 |
| `reused count` | 0 |
| `hardlinked count` | 450 |
| `copied bytes` | 26,240 B |
| `modified count` | 50 |

说明：

- 当前 `reused count` 指“跨路径内容复用”的 diff 计数，本场景没有 rename/duplicate 新路径，因此为 `0`。
- `hardlinked count = 450` 说明全量模式下未变化文件已大规模走 hardlink 复用。
- 对这类“少量文件变更”的典型项目，瓶颈已从复制转移到扫描与哈希判定阶段。

---

## 3. 当前结论

### 3.1 已确认有效

- hash cache 热命中后，hash 驱动增量的 diff 成本可控制在毫秒级
- 全量模式下，未变化文件大规模 hardlink 复用是主要收益来源
- 50 / 500 文件变更场景下，实际复制量已降到 26 KB 量级

### 3.2 仍待继续优化

- `cold` 场景下的 hash 重算仍然昂贵
- `file_id` 仍未接入真实采集，rename-only 命中仍完全依赖内容 hash
- 还缺更大目录、更多层级、更多重复内容场景的长期基线

---

## 4. 后续建议

- 固定一组真实 fixture，持续记录 `warm/cold` 两档基线
- 增加“同内容新路径”与“删除/新增/修改混合”基准
- 若后续引入 `file_id`，单独记录 rename-only 场景基线
