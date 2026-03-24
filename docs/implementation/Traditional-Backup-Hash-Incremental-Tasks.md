# 传统 `backup` 哈希增量重构 — TDD 分阶段任务清单

> 依据：[Traditional-Backup-Hash-Incremental-Design.md](./Traditional-Backup-Hash-Incremental-Design.md)
> 标记：`[ ]` 待办　`[-]` 进行中　`[x]` 完成
> 原则：**红-绿-重构**。每个任务先写失败测试，再写最小实现，最后重构。
> 说明：本任务清单按**破坏性改动**前提设计，不考虑旧传统 backup manifest/baseline 兼容。
> 额外原则：**先抽共享哈希模块，再让传统 `backup` 接入**，避免传统 backup 直接耦合 `.xunbak` 容器层代码。

---

## Phase 0：共享哈希模块抽取

### 0.1 共享模块骨架

- [x] 新建 `src/backup/common/mod.rs`
- [x] 新建 `src/backup/common/hash.rs`
- [x] `src/backup/mod.rs`：注册 `common` 模块
- [x] 明确共享模块只承载“内容身份”能力，不承载 `.xunbak` 容器字段

### 0.2 从 `.xunbak` 抽公共哈希能力

- [x] **测试**：共享模块中的 `compute_file_content_hash(path)` 与当前 `.xunbak` 结果完全一致
- [x] **测试**：共享模块中的 `build_content_hash_index(entries)` 行为与 `.xunbak` 当前实现一致
- [x] **测试**：共享模块中的 `build_path_index(entries)` 行为与 `.xunbak` 当前实现一致
- [x] 从 `src/xunbak/writer.rs` 抽出以下公共能力到 `src/backup/common/hash.rs`
  - `compute_file_content_hash`
  - `build_content_hash_index`
  - `build_path_index`
  - `hash hex encode/decode`（若需要）

### 0.3 先让 `.xunbak` 回接共享模块

- [x] **测试**：`.xunbak` 现有测试在切换到共享哈希模块后全部通过
- [x] `src/xunbak/writer.rs`：改为调用共享哈希模块
- [x] 确保 `.xunbak` 的 `content_hash`、`path_index`、`content_index` 语义不变

---

## Phase 1：模块骨架与数据边界重定义

### 1.1 代码边界确认

- [x] 新建 `src/backup/legacy/hash_manifest.rs` 或直接扩展 `baseline.rs`，作为传统 backup 的新 hash manifest 模型载体
- [x] `src/backup/legacy/mod.rs`：导出新的 hash-manifest / hash-diff 能力
- [x] `src/commands/backup.rs`：为新哈希增量主流程预留入口
- [x] 明确移除旧“baseline 仅 size+mtime”的假设

### 1.2 新核心类型

- [x] **测试**：`BackupSnapshotManifest` 可构造，包含 `version / snapshot_id / created_at_ns / source_root / file_count / total_raw_bytes / entries / removed`
- [x] **测试**：`BackupSnapshotEntry` 包含 `path / content_hash / size / mtime_ns / created_time_ns / win_attributes / file_id`
- [x] **测试**：`content_hash` 必填，不允许 `None`
- [x] **测试**：`snapshot_id` 使用 ULID 字符串格式
- [x] 实现 manifest 核心结构体

---

## Phase 2：Hash Manifest 读写替换旧 baseline

### 2.1 顶层 manifest 序列化

- [x] **测试**：`BackupSnapshotManifest` JSON 序列化 -> 反序列化往返一致
- [x] **测试**：`version = 2`
- [x] **测试**：`removed` 为空时仍序列化为 `[]`
- [x] **测试**：`content_hash` 以 64 位小写 hex 输出
- [x] 实现 manifest serde

### 2.2 文件路径与时间字段

- [x] **测试**：路径统一使用相对路径 + `/`
- [x] **测试**：`mtime_ns` 使用 Unix epoch nanoseconds
- [x] **测试**：`created_time_ns` 缺失时可为空
- [x] **测试**：`win_attributes` 正确保留 `u32`

### 2.3 Hash Manifest 持久化

- [x] **测试**：目录备份完成后生成 `.bak-manifest.json`
- [x] **测试**：zip 备份内部包含 `.bak-manifest.json`
- [x] **测试**：读取目录 backup 时优先读取 `.bak-manifest.json`
- [x] **测试**：读取 zip backup 时优先读取内部 `.bak-manifest.json`
- [x] 实现 `read_backup_snapshot_manifest()` / `write_backup_snapshot_manifest()`
- [x] 复用共享模块提供的 hash 编解码能力，不在 manifest 层重复实现

### 2.4 破坏性切换

- [x] **测试**：旧版仅 `size + mtime` baseline 不再被读取为增量真相
- [x] **测试**：缺少新 `.bak-manifest.json` 时，增量流程返回明确错误或触发全量初始化
- [x] 移除旧 baseline 作为权威增量来源的逻辑

---

## Phase 3：扫描结果升级为哈希驱动输入

### 3.1 `ScannedFile` 扩展

- [x] **测试**：`ScannedFile` 包含 `size / mtime_ns / created_time_ns / win_attributes / file_id / content_hash`
- [x] **测试**：扫描阶段至少能正确填充 `size / mtime_ns / win_attributes`
- [x] **测试**：Windows 下 `created_time_ns` 和 `file_id` 可选填充
- [x] 改造 `src/backup/legacy/scan.rs`

### 3.2 流式哈希接入共享模块

- [x] **测试**：小文件 `blake3` 结果稳定
- [x] **测试**：10 MB 文件流式 `blake3` 与一次性读取结果一致
- [x] **测试**：空文件 hash 正确
- [x] `scan.rs` 与传统 backup 流程统一调用 `src/backup/common/hash.rs` 中的 `compute_file_content_hash(path)`

### 3.3 当前快照扫描

- [x] **测试**：`scan_files_with_hash()` 返回的每个文件都带 `content_hash`
- [x] **测试**：同内容不同路径的文件 hash 相同
- [x] **测试**：中文路径、空格路径、深层路径全部正确输出
- [x] 实现 `scan_files_with_hash()`

---

## Phase 4：索引模型（参考 xunbak）

### 4.1 Path Index

- [x] **测试**：从上一版本 manifest 构建 `path_index`
- [x] **测试**：路径查找大小写策略与 Windows 语义一致
- [x] 直接接入共享模块的 `build_path_index(manifest)`

### 4.2 Content Index

- [x] **测试**：从上一版本 manifest 构建 `content_index`
- [x] **测试**：同 hash 多路径时返回 `Vec<EntryRef>`
- [x] **测试**：单 hash 命中旧路径可用于 rename/reuse
- [x] 直接接入共享模块的 `build_content_hash_index(manifest)`

### 4.3 Current Index

- [x] **测试**：当前扫描结果可构建 `current_path_index`
- [x] **测试**：当前扫描结果全部有 `content_hash`
- [x] 实现 `build_current_path_index(scanned)`

---

## Phase 5：哈希驱动 diff

### 5.1 DiffKind 重定义

- [x] **测试**：新 `DiffKind` 包含 `New / Modified / Reused / Unchanged / Deleted`
- [x] 删除旧 `MetadataOnlyChanged / RenamedOrReused` 语义设计分支

### 5.2 同路径比较

- [x] **测试**：同路径、同 hash -> `Unchanged`
- [x] **测试**：同路径、不同 hash -> `Modified`
- [x] 实现同路径优先比较

### 5.3 跨路径复用

- [x] **测试**：新路径、旧路径 hash 相同 -> `Reused`
- [x] **测试**：rename 场景被识别为 `Reused`
- [x] **测试**：同内容跨目录移动也能命中 `Reused`
- [x] 实现基于 `content_index` 的复用判定

### 5.4 删除判定

- [x] **测试**：旧 manifest 中存在但当前扫描不存在 -> `Deleted`
- [x] **测试**：删除文件会进入 `removed`
- [x] 实现 `Deleted` 路径

### 5.5 总体 diff 入口

- [x] **测试**：`diff_against_hash_manifest(current, previous)` 输出完整 diff 结果
- [x] **测试**：新/改/复用/删除数量统计正确
- [x] 实现新的 hash-diff 主函数

---

## Phase 6：写入策略重构

### 5.1 增量模式写入

- [x] **测试**：`New` 文件被复制
- [x] **测试**：`Modified` 文件被复制
- [x] **测试**：`Unchanged` 文件不复制
- [x] **测试**：`Deleted` 文件不进入新版本目录
- [x] 实现增量模式写入调度

### 5.2 `Reused` 写入

- [x] **测试**：`Reused` 在有前一版本目录时优先 hardlink
- [x] **测试**：无法 hardlink 时回退复制
- [x] **测试**：同内容不同路径时不重复读取旧内容作为“新内容”
- [x] 实现 `Reused` 的 hardlink/复制策略

### 5.3 全量模式写入

- [x] **测试**：全量模式下 `Unchanged` 和 `Reused` 都能按内容复用 hardlink
- [x] **测试**：`Modified/New` 正常复制
- [x] 实现全量模式的 hash-based hardlink 复用

---

## Phase 7：新 manifest 生成

### 6.1 写出 snapshot manifest

- [x] **测试**：完成 backup 后写出新的 `.bak-manifest.json`
- [x] **测试**：manifest 中 `entries` 与最终目录内容一致
- [x] **测试**：`removed` 正确记录删除路径
- [x] **测试**：`file_count / total_raw_bytes` 正确
- [x] 实现 manifest 生成

### 6.2 Hash 一致性

- [x] **测试**：manifest 中的 `content_hash` 与实际文件内容一致
- [x] **测试**：恢复后重新计算 hash 与 manifest 一致
- [x] 实现最终一致性校验

---

## Phase 8：verify 链路重构

### 7.1 verify 输入改造

- [x] **测试**：`backup verify` 优先使用新的 `.bak-manifest.json`
- [x] **测试**：缺少 manifest 时给出明确错误
- [x] 实现 verify 读取新 manifest

### 7.2 verify 哈希校验

- [x] **测试**：所有文件内容 hash 与 manifest 匹配 -> verify 通过
- [x] **测试**：任一文件内容被篡改 -> verify 失败并定位路径
- [x] **测试**：manifest 缺失 entry -> verify 失败
- [x] 改造 `verify_manifest()`

---

## Phase 9：zip / dir 结果接入

### 8.1 目录备份

- [x] **测试**：目录备份结果包含新的 `.bak-manifest.json`
- [x] **测试**：下一次增量从该 manifest 构建 baseline
- [x] 实现目录备份链路接入

### 8.2 zip 备份

- [x] **测试**：zip 内部包含 `.bak-manifest.json`
- [x] **测试**：读取 zip backup 做 baseline 时优先读取内部 manifest，而不是直接遍历 zip entry 元数据
- [x] 实现 zip baseline 接入

---

## Phase 10：本地 hash cache（性能优化）

### 10.1 缓存结构

- [x] **测试**：`HashCacheEntry` JSON 序列化/反序列化正确
- [x] **测试**：包含 `size / mtime_ns / created_time_ns / win_attributes / file_id / content_hash`
- [x] 实现 `hash_cache.rs`

### 10.2 缓存命中

- [x] **测试**：元数据未变时直接复用缓存 hash
- [x] **测试**：元数据变化时重新计算 hash
- [x] **测试**：缓存损坏时回退全量重算，不影响正确性
- [x] 实现缓存读取/写回

### 10.3 大目录性能

- [x] **测试**：第二次运行相同目录时，hash 计算次数显著减少
- [x] 记录性能基线

---

## Phase 11：CLI / 统计 / 报告

### 11.1 报告字段

- [x] **测试**：输出包含 `new / modified / reused / unchanged / deleted`
- [x] **测试**：输出包含 `hash_checked_files`
- [x] 实现新报告字段

### 11.2 JSON 输出

- [x] **测试**：JSON 输出包含新 diff 统计
- [x] **测试**：`reused` 字段稳定存在
- [x] 实现 JSON 报告扩展

### 11.3 可选 diff-mode（如启用）

- [x] **测试**：`--diff-mode meta|hash|auto` 解析正确
- [x] **测试**：不同 mode 下行为符合预期
- [x] 若决定保留 CLI 开关，则实现

---

## Phase 12：端到端测试

### 12.1 内容不变但 mtime 变化

- [x] **测试**：只改 mtime，不改内容 -> `Unchanged`
- [x] **测试**：不会重复复制

### 12.2 rename

- [x] **测试**：路径变化、内容不变 -> `Reused`
- [x] **测试**：增量结果正确，恢复结果正确

### 12.3 同内容跨路径

- [x] **测试**：新增一个内容完全相同的新文件 -> `Reused`
- [x] **测试**：全量模式可 hardlink 复用

### 12.4 删除/新增/修改混合

- [x] **测试**：一次变更同时包含 `New / Modified / Reused / Deleted`
- [x] **测试**：最终 manifest 与恢复结果正确

### 12.5 zip 基线

- [x] **测试**：从 zip 备份继续做下一次增量，hash baseline 正常工作

---

## Phase 13：基准测试

### 13.1 基线对比

- [x] **bench**：旧 `size + mtime` diff 模式
- [x] **bench**：新 hash-driven diff（首次）
- [x] **bench**：新 hash-driven diff（缓存命中）

### 13.2 关键指标

参考基线记录：

- [Traditional-Backup-Hash-Incremental-Benchmarks.md](./Traditional-Backup-Hash-Incremental-Benchmarks.md)

- [x] 记录：
  - `scan time`
  - `hash time`
  - `copy time`
  - `reused count`
  - `hardlinked count`

---

## 依赖关系

```text
Phase 0（共享哈希模块） ─→ Phase 1 ─→ Phase 2 ─→ Phase 3 ─→ Phase 4 ─→ Phase 5
                                                                         │
                                                                         └→ Phase 6 ─→ Phase 7 ─→ Phase 8 ─→ Phase 9
Phase 10（cache）←──────────────────────────────────────────────────────────────┘
Phase 11（CLI/report）←──────────────────────────────────────────── Phase 5/6/7/9/10
Phase 12（E2E）←────────────────────────────────────────────────── Phase 6/7/8/9/10/11
Phase 13（bench）←──────────────────────────────────────────────── Phase 12
```

---

## 测试运行命令建议

```bash
# 传统 backup 相关单元/集成测试
cargo test --test module_backup_restore

# 全量库测试
cargo test --lib

# 关键编译验证
cargo check --lib
```
