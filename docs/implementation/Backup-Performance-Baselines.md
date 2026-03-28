# Backup 性能基线

> 目标：把 `backup / restore / verify / sidecar` 的性能采集固定成可重复、可比对、可回写的流程。

---

## 1. 基准入口

### 1.1 共享热点

- bench: [backup_perf_bench_divan.rs](/D:/100_Projects/110_Daily/XunYu/benches/backup_perf_bench_divan.rs)
- 关注项：
  - `hash_file_content_64mb`
  - `sidecar_build_missing_hash_1000_files`
  - `sidecar_build_prehash_1000_files`
  - `verify_entries_content_dir_1000_files`
  - `verify_entries_content_xunbak_1000_files`
  - `verify_full_xunbak_1000_files`
  - `xunbak_restore_all_1000_files`
  - `xunbak_restore_incremental_1000_files`

### 1.2 导出链路

- bench: [backup_export_bench_divan.rs](/D:/100_Projects/110_Daily/XunYu/benches/backup_export_bench_divan.rs)
- 关注项：
  - `create_zip_200_files`
  - `create_7z_200_files`
  - `restore_zip_200_files`
  - `restore_7z_200_files`
  - `convert_xunbak_to_zip_200_files`
  - `convert_xunbak_to_7z_200_files`

### 1.3 `.xunbak` 低层基线

- bench: [xunbak_bench_divan.rs](/D:/100_Projects/110_Daily/XunYu/benches/xunbak_bench_divan.rs)
- 关注项：
  - `backup_100_files`
  - `restore_100_files`
  - `verify_quick`
  - `verify_full`

---

## 2. 固化脚本

- 脚本：[backup_perf_baseline.ps1](/D:/100_Projects/110_Daily/XunYu/scripts/backup_perf_baseline.ps1)
- 用途：
  - 顺序运行关键 `backup_perf_bench_divan` 基准
  - 可选补跑导出基准
  - 追加一次 `.xunbak restore_all` 单样本阶段 timing
  - 输出 Markdown 日志到 `logs/`

### 2.1 用法

```powershell
.\scripts\backup_perf_baseline.ps1
.\scripts\backup_perf_baseline.ps1 -IncludeExport
```

### 2.2 输出

- 默认输出：`logs/backup_perf_baseline_YYYYMMDD_HHMMSS.md`
- 内容包含：
  - benchmark median / mean / fastest / slowest
  - `XUN_XUNBAK_RESTORE_TIMING=1` 的单样本阶段统计

---

## 3. 当前重点指标

### 3.1 `.xunbak restore`

当前已经形成两条稳定基线：

- `xunbak_restore_all_1000_files`
  - 反映空目录 / 全量恢复的吞吐
- `xunbak_restore_incremental_1000_files`
  - 反映已有目标目录、命中“未变化跳过”时的收益

这两项需要一起看，不能只看其中之一。

### 3.2 `sidecar`

`sidecar_build_missing_hash_1000_files` 是 sidecar 热点的主指标。  
若它退化，而 `sidecar_build_prehash_1000_files` 没退化，通常说明“缺失 hash 回退路径”又变慢了。

### 3.3 `verify`

- `verify_entries_content_xunbak_1000_files`
- `verify_full_xunbak_1000_files`

这两项分别对应：

- 内容校验热路径
- `.xunbak` 全链路 full verify

若要优化 `verify`，必须同时看这两项；只优化其中一个容易出现“看起来快了、整体没变甚至变慢”的假象。

---

## 4. 判定规则

### 4.1 保留优化

满足以下任一条件即可保留：

- 关键 benchmark 中位数稳定下降
- 波动范围不增大，且单样本阶段 timing 明确显示热点被压缩
- 用户可见语义更完整，且性能无显著回退

### 4.2 回退优化

应回退的典型情况：

- benchmark 中位数稳定变差
- low-level bench 与 high-level bench 同时退化
- 复杂度显著上升，但收益只停留在“可能更好”

---

## 5. 当前经验

- `.xunbak restore_all` 的优化不能只看 syscall 层，要同时看：
  - 归档读锁范围
  - 文件系统创建/元数据成本
  - 并行调度策略
  - 增量跳过命中率
- `verify_full` 的并行化并不一定有收益；若读 locality 被破坏，很容易比串行更慢。
- `sidecar` 最有价值的优化不是 JSON 序列化本身，而是缺失 `content_hash` 时的回退路径。
