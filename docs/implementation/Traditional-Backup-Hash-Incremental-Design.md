# 传统 `backup` 哈希增量重构设计方案

生成时间：2026-03-24

> 本文面向 XunYu 现有传统 `backup` 目录备份链路。
> 本方案**允许破坏性改动**，不考虑与旧版传统 backup baseline/manifest 的兼容性。
> 设计以当前 `.xunbak` 的 `content_hash + content_index + path_index` 思路为参考，把传统目录/zip 备份升级为**哈希驱动增量**。

---

## 1. 目标

把传统 `backup` 从当前的：

1. `size + mtime` 元数据驱动增量
2. 备份后再写校验哈希
3. 无 rename/content reuse 语义

升级为：

1. **`content_hash` 作为增量判定的主真相**
2. **`path_index + content_index` 作为 diff 的核心索引**
3. **同内容复用**
4. **rename 识别**
5. **统一 manifest 既服务 verify，也服务下次增量**

一句话：

> 传统 `backup` 不再把哈希当“备份完成后的附加校验”，而是把哈希变成“增量决策的核心数据结构”。

---

## 2. 设计边界

### 2.1 适用对象

本方案覆盖：

1. `backup`
2. `backup create --format dir`
3. `backup create --format zip`

说明：

1. `7z` / `.xunbak` 已经有各自独立的实现模型
2. 本方案关注的是“传统目录版本备份”这条线

### 2.2 非目标

本方案**不**做：

1. pack/chunk 仓库
2. 全局对象存储
3. `.xunbak` 容器替代
4. 与旧 `.bak-manifest.json` 结构兼容
5. 与旧 baseline 行为兼容

### 2.3 破坏性策略

允许以下破坏性变化：

1. 直接替换当前 `baseline.rs` 的数据结构
2. 直接替换 `.bak-manifest.json` 的语义
3. 旧传统 backup 的 manifest 不再作为可持续增量 baseline
4. 首次升级后可要求重新生成基线

---

## 3. 当前问题

当前传统 `backup` 的核心问题：

1. `src/backup/legacy/diff.rs` 只比较 `size + modified`
2. `src/backup/legacy/baseline.rs` 不记录 `content_hash`
3. `src/backup/legacy/checksum.rs` 的哈希只服务 verify
4. `src/backup/artifact/sidecar.rs` 的 `export_manifest.json` 只是导出说明，不参与增量

结果：

1. 内容不变但 `mtime` 变了，会被误判为 `Modified`
2. rename 但内容不变，无法识别为复用
3. 同内容跨路径无法参与传统 backup 的去重/复用
4. 校验数据与增量数据割裂

---

## 4. 参考模型：`.xunbak`

本方案直接参考 `.xunbak` 当前的核心思路：

1. `content_hash` 是内容级身份
2. `path_index` 解决“同路径旧文件”
3. `content_index` 解决“同内容旧文件”
4. 先看路径，再看内容
5. 内容相同即可复用

相关实现位于：

1. [writer.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/writer.rs)
2. [manifest.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/manifest.rs)
3. [reader.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/reader.rs)

本方案不照搬容器格式，但照搬以下思想：

1. `content_hash` 必须进入 manifest
2. `build_content_hash_index()` 必须存在
3. `build_path_index()` 必须存在
4. diff 不能只靠元数据

### 4.1 可直接复用的 `.xunbak` 哈希能力

应明确复用以下现有能力，而不是重新发明：

1. `compute_file_content_hash(path)`
   - 当前实现位于 `src/xunbak/writer.rs`
   - 已采用流式 `blake3::Hasher`
   - 可直接作为传统 `backup` 的文件内容哈希实现基线
2. `build_content_hash_index(manifest)`
   - 当前实现位于 `src/xunbak/writer.rs`
   - 思路完全可迁移到传统 backup 的 snapshot manifest
3. `build_path_index(manifest)`
   - 当前实现位于 `src/xunbak/writer.rs`
   - 可直接复用索引构建策略
4. `diff_against_manifest(...)`
   - 不直接复用容器字段，但可复用“先 path，再 hash”的 diff 结构

必须强调：

1. 复用的是**哈希与索引思想/实现**
2. 不是复用 `.xunbak` 的 `BlobLocator / blob_offset / blob_len / codec / volume_index`
3. 传统 `backup` 不应依赖容器层字段

### 4.2 不应直接复用的 `.xunbak` 容器代码

以下内容不应直接拉进传统 `backup`：

1. `BlobLocator`
2. `ManifestEntry` 中的 blob 定位字段
3. checkpoint / footer / record 体系
4. append-only writer/update 路径

原因：

1. 这些字段服务于单文件容器
2. 传统 `backup` 仍然是目录/zip 结果导向
3. 如果直接耦合容器层，会让传统 backup 的实现边界被污染

### 4.3 推荐：抽共享哈希模块

最佳方案不是让传统 `backup` 直接 import `src/xunbak/writer.rs` 中的内部函数，
而是抽出共享模块。

建议新增：

```text
src/backup/common/hash.rs
```

或：

```text
src/hash_identity.rs
```

建议该模块承载：

1. `compute_file_content_hash(path) -> [u8; 32]`
2. `hash_bytes(bytes) -> [u8; 32]`
3. `encode_hash_hex(hash) -> String`
4. `decode_hash_hex(s) -> [u8; 32]`
5. `build_path_index(entries)`
6. `build_content_hash_index(entries)`

分层要求：

1. **共享哈希模块**
   - 只依赖通用数据结构或 trait
   - 不依赖 `.xunbak` 容器字段
2. **传统 `backup`**
   - 用共享模块做 snapshot manifest diff
3. **`.xunbak`**
   - 用共享模块做 content identity
   - 容器专有的 `BlobLocator / parts / offsets` 仍保留在容器层

结论：

1. 应复用 `.xunbak` 的哈希代码
2. 但复用方式应是**抽公共模块**
3. 不应让传统 `backup` 直接耦合 `.xunbak` writer/container 细节

---

## 5. 新总模型

传统 backup 升级后的核心流程：

```text
扫描源目录元数据
    ↓
加载上一版本哈希 manifest
    ↓
构建 path_index / content_index
    ↓
为当前文件生成或读取 content_hash
    ↓
基于 path + hash 判定 New / Modified / Reused / Deleted
    ↓
生成新版本目录
    ↓
写出新的 .bak-manifest.json
    ↓
verify / restore 继续使用该 manifest
```

核心变化：

1. 不再需要额外 `.bak-baseline.json`
2. `.bak-manifest.json` 直接升级为**权威 snapshot manifest**
3. manifest 同时承担：
   - verify 基线
   - 下次增量 baseline

---

## 6. 权威数据文件

### 6.1 `.bak-manifest.json` 升级为唯一真相

新设计里：

1. `.bak-manifest.json` 是每个传统 backup 的唯一权威元数据文件
2. 不再拆分为“校验 manifest”和“增量 baseline”两个文件
3. sidecar `export_manifest.json` 继续只做导出说明，不参与传统 backup 增量

### 6.2 文件位置

目录备份：

```text
backup-dir/
  .bak-meta.json
  .bak-manifest.json
  src/...
  docs/...
```

zip 备份：

```text
backup.zip
  .bak-manifest.json
  .bak-meta.json
  src/...
```

说明：

1. ZIP 中必须包含 `.bak-manifest.json`
2. 读取 zip baseline 时，不再只看 entry mtime/size，而是优先读内部 manifest

---

## 7. Manifest 结构

### 7.1 顶层结构

建议直接引入 snapshot 语义：

```rust
pub(crate) struct BackupSnapshotManifest {
    pub version: u32,
    pub snapshot_id: String,
    pub created_at_ns: u64,
    pub source_root: String,
    pub file_count: u64,
    pub total_raw_bytes: u64,
    pub entries: Vec<BackupSnapshotEntry>,
    pub removed: Vec<String>,
}
```

说明：

1. `version` 从 `2` 开始
2. `snapshot_id` 建议直接采用 ULID
3. `removed` 明确表达相对上一版本删除的路径

### 7.2 Entry 结构

```rust
pub(crate) struct BackupSnapshotEntry {
    pub path: String,
    pub content_hash: [u8; 32],
    pub size: u64,
    pub mtime_ns: u64,
    pub created_time_ns: Option<u64>,
    pub win_attributes: u32,
    pub file_id: Option<u128>,
}
```

要求：

1. `content_hash` **必须存在**
2. 不允许像旧版一样只存 size/mtime
3. 时间统一使用 Unix epoch nanoseconds

### 7.3 JSON 示例

```json
{
  "version": 2,
  "snapshot_id": "01JQ7J3N4QCG7N2T1DJ7C9ZK4Q",
  "created_at_ns": 1774320000000000000,
  "source_root": "D:\\project",
  "file_count": 3,
  "total_raw_bytes": 18432,
  "entries": [
    {
      "path": "src/main.rs",
      "content_hash": "3b8e4f62e0d8c2e5d3b8f0b3a8c64c4df55b8cb24a8f7b9ff04d7b9fd8a9f4d2",
      "size": 1234,
      "mtime_ns": 1774319900000000000,
      "created_time_ns": 1774319000000000000,
      "win_attributes": 32,
      "file_id": "0000000000000000000000000000abcd"
    },
    {
      "path": "README.md",
      "content_hash": "a9f75d6cbb6c4f8e1d3db2b67a11de7c6b7e97bde5960e7b7c10dc64e3b1f2ab",
      "size": 2048,
      "mtime_ns": 1774319999000000000,
      "created_time_ns": 1774319100000000000,
      "win_attributes": 32,
      "file_id": null
    }
  ],
  "removed": [
    "docs/old.txt"
  ]
}
```

---

## 8. 索引模型

### 8.1 Path Index

基于上一版本 manifest 构建：

```rust
HashMap<&str, &BackupSnapshotEntry>
```

用途：

1. 判断同路径文件是否存在
2. 比较同路径新旧 hash

### 8.2 Content Index

基于上一版本 manifest 构建：

```rust
HashMap<[u8; 32], Vec<&BackupSnapshotEntry>>
```

用途：

1. 识别 rename
2. 识别 same-content reuse
3. 为全量模式 hardlink 复用提供候选

### 8.3 Current Scan Index

当前扫描结果也需要有：

```rust
HashMap<String, ScannedFile>
```

但 `ScannedFile` 必须升级为包含：

1. `size`
2. `mtime_ns`
3. `created_time_ns`
4. `win_attributes`
5. `file_id`
6. `content_hash`

注意：

1. `content_hash` 在最终 diff 前必须补齐
2. 不允许留空进入最终比较阶段

---

## 9. 新的 diff 模型

### 9.1 DiffKind

建议直接定义：

```rust
pub enum DiffKind {
    New,
    Modified,
    Reused,
    Unchanged,
    Deleted,
}
```

说明：

1. 不再保留旧的 `MetadataOnlyChanged`
2. 因为主语义从“元数据驱动”切换到了“内容驱动”
3. 若 hash 相同，则逻辑上就是 `Unchanged` 或 `Reused`

### 9.2 判定规则

对每个当前文件：

1. 若 `path_index` 命中：
   - `old.content_hash == new.content_hash`
     - `Unchanged`
   - 否则
     - `Modified`
2. 若 `path_index` 未命中，但 `content_index` 命中：
   - `Reused`
3. 两者都未命中：
   - `New`

最后对旧 manifest 中未被匹配的路径：

1. 标记 `Deleted`

### 9.3 关键变化

旧模型：

1. 先看 `size + mtime`
2. 再决定是否复制

新模型：

1. **先拿到 `content_hash`**
2. 再做 diff

也就是说：

> 哈希不是辅助字段，而是 diff 的核心输入。

---

## 10. 哈希获取策略

### 10.1 逻辑真相

逻辑上，`content_hash` 是唯一真相。

### 10.2 性能优化

性能上，仍允许本地缓存避免重复算 hash。

建议新增：

```text
.xun-bak-hash-cache.json
```

或放到 `%LOCALAPPDATA%/xun/cache/`

缓存项建议：

```rust
pub struct HashCacheEntry {
    pub size: u64,
    pub mtime_ns: u64,
    pub created_time_ns: Option<u64>,
    pub win_attributes: u32,
    pub file_id: Option<u128>,
    pub content_hash: [u8; 32],
}
```

### 10.3 缓存命中规则

对于当前文件：

1. 若缓存中存在该路径
2. 且 `size + mtime_ns + file_id` 未变化
3. 则直接复用缓存中的 `content_hash`

否则：

1. 流式读取文件
2. 重新计算 `blake3`
3. 回写缓存

### 10.4 重要约束

1. 缓存只是性能优化
2. manifest 才是上一版本的权威 baseline
3. 缓存丢失不影响正确性
4. 缓存损坏时直接全量重算 hash

---

## 11. 写入策略

### 11.1 增量模式

建议：

1. `New`
   - 复制文件
2. `Modified`
   - 复制文件
3. `Reused`
   - 若前一版本目录还在，优先 hardlink
   - 否则复制
4. `Unchanged`
   - 不复制
5. `Deleted`
   - 记录到 `removed`

### 11.2 全量模式

建议：

1. `New / Modified`
   - 正常复制
2. `Reused / Unchanged`
   - 优先 hardlink 到前一版本同 hash 文件
3. `Deleted`
   - 不进入新版本目录

### 11.3 同内容跨路径复用

这是新模型的核心收益之一。

例子：

```text
old: docs/a.txt  (hash = H1)
new: docs/b.txt  (hash = H1)
```

新模型会判定：

1. `docs/b.txt` = `Reused`
2. 可直接 hardlink 或复制已有内容
3. 不需要把它当作全新内容

---

## 12. verify 与 manifest 的关系

### 12.1 `.bak-manifest.json` 的新职责

升级后 `.bak-manifest.json` 同时承担：

1. verify 权威清单
2. 下次增量 baseline

### 12.2 verify 行为

`backup verify` 继续：

1. 遍历 manifest entries
2. 对每个目标文件重新计算 `blake3`
3. 比较 `content_hash`

### 12.3 结果

这样不会再出现：

1. verify 用一套哈希
2. diff 用另一套逻辑

所有备份判断都会围绕同一个 `content_hash` 模型。

---

## 13. sidecar 的定位

`export_manifest.json` 继续保留，但明确降级为：

1. 导出说明文件
2. 仅服务 `dir / zip / 7z` artifact 使用者
3. 不进入传统 `backup` 增量判定链路

所以要明确：

1. `sidecar != baseline`
2. `sidecar != verify manifest`
3. `sidecar` 只是一份人类/工具可读导出摘要

---

## 14. 代码落点

### 14.1 必改文件

1. [baseline.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/legacy/baseline.rs)
   - 改为读取/写入新的 manifest 结构，或至少读取新的 hash manifest
2. [diff.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/legacy/diff.rs)
   - 改成基于 `content_hash` 的 diff
3. [scan.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/legacy/scan.rs)
   - 补充 `mtime_ns / created_time_ns / win_attributes / file_id`
4. [checksum.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/legacy/checksum.rs)
   - 可与新 manifest 结构合并或重构
5. [backup.rs](/D:/100_Projects/110_Daily/XunYu/src/commands/backup.rs)
   - 接入新 diff 统计和复制策略

### 14.2 新增建议

1. `src/backup/common/hash.rs`
   - 共享哈希基础设施
   - 从 `.xunbak` 中抽出 `compute_file_content_hash / build_path_index / build_content_hash_index`
2. `src/backup/legacy/hash_cache.rs`
   - 本地 hash cache
3. `src/backup/legacy/hash_diff.rs`
   - 若想降低侵入，可先新增再并回 `diff.rs`

### 14.3 推荐改造顺序

建议先做共享抽象，再做传统 backup 改造：

1. 从 `.xunbak` 抽出共享哈希模块
2. 让 `.xunbak` 自己先切到共享模块
3. 再让传统 `backup` 使用同一套 `compute_file_content_hash / path_index / content_index`
4. 最后重写传统 `diff.rs`

这样可以保证：

1. 哈希算法与索引构建逻辑在两条链路中完全一致
2. 后续修 bug 只修一处
3. 不会让传统 backup 直接依赖容器内部结构

---

## 15. 新的阶段计划

### Phase 1：破坏性切换

1. 引入新 `BackupSnapshotManifest`
2. `content_hash` 变成必填字段
3. 写出新的 `.bak-manifest.json`
4. 旧 manifest 不再读取

### Phase 2：哈希驱动 diff

1. 所有 current 文件都拿到 `content_hash`
2. 构建 `path_index + content_index`
3. 输出 `New / Modified / Reused / Unchanged / Deleted`

### Phase 3：性能优化

1. 引入本地 hash cache
2. 大文件流式 BLAKE3
3. 并行 hash
4. `Reused` 场景下 hardlink 优化

---

## 16. TDD 测试建议

建议新增/重写测试：

1. 新 manifest 结构读写
2. manifest 中 `content_hash` 必填
3. 同路径同 hash -> `Unchanged`
4. 同路径不同 hash -> `Modified`
5. 新路径同 hash -> `Reused`
6. baseline 中存在但当前没有 -> `Deleted`
7. hash cache 命中 -> 不读文件内容
8. hash cache 未命中 -> 流式 BLAKE3
9. 全量模式下 `Reused` 走 hardlink
10. 增量模式下 `Reused` 默认复用已有内容

---

## 17. 风险

### 17.1 首次升级会变慢

因为：

1. 旧 baseline 不再可用
2. 首次需要建立新的完整哈希 manifest

这是允许的，因为本方案明确允许破坏性切换。

### 17.2 大目录首次全量哈希成本高

规避：

1. 流式 BLAKE3
2. 并行 hash
3. 后续通过本地 cache 降低成本

### 17.3 旧校验链路会失效

这是设计选择：

1. 不做兼容保留
2. 统一成 hash-first 模型

---

## 18. 结论

在允许破坏性改动、不要求兼容的前提下，传统 `backup` 最合理的升级方向不是：

1. 继续给 `size + mtime` 打补丁
2. 再叠一个 baseline 文件
3. 让 verify、sidecar、增量各自维护不同哈希来源

而是：

1. 直接把 `.bak-manifest.json` 升级为**权威 hash manifest**
2. 让 `content_hash` 成为增量判定主真相
3. 采用 `.xunbak` 同款的 `path_index + content_index` 思路
4. 用本地 hash cache 解决性能问题，而不是继续让元数据担任语义真相

一句话总结：

> 传统 `backup` 在破坏性升级后，应改造成“以 `content_hash` 为核心、以 snapshot manifest 为唯一真相、以 cache 为性能优化”的哈希驱动增量系统。
