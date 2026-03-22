# `.xunbak` 单文件容器 MVP 设计 — 第二轮审核建议

> 本文是对更新后文档（631 行版）的二次审核。第一轮审核的核心建议（file-end footer、`record_len`、manifest codec 预留、`win_attributes`、compact 原子替换等）已被吸收。本轮聚焦在 **第一轮未覆盖的实现细节风险**。

---

## 1. 更新后文档的评价

文档质量显著提升。主要改善点：

| 第一轮建议                             | 吸收情况                                                                 |
| -------------------------------------- | ------------------------------------------------------------------------ |
| Header 固定长度 + little-endian        | ✅ 已明确为 64 字节，并给出了字段布局表                                  |
| `min_reader_version`                   | ✅ 已补充                                                                |
| `record_len` 字段                      | ✅ 已加入 blob record                                                    |
| 去掉 `path_hash`                       | ✅ 已移除，blob 只表达"内容存储"                                         |
| manifest `codec` 预留                  | ✅ 已增加 `manifest_codec` + `manifest_version`                          |
| `win_attributes` / `created_time_unix` | ✅ 已补充到 entry                                                        |
| file-end footer                        | ✅ 已新增 §4.5，含 magic + offset                                        |
| checkpoint 增加统计字段                | ✅ 已增加 `blob_count`、`referenced_blob_bytes`、`total_container_bytes` |
| content_hash 去重                      | ✅ 已写入 §6 步骤 6                                                      |
| 并行恢复策略                           | ✅ 已新增 §7.1                                                           |
| compact Windows 原子替换               | ✅ 已新增 §9.1                                                           |
| compact 触发指标                       | ✅ 已新增 §9.2                                                           |

> [!TIP]
> 文档已经从"方向性草案"进化为"可直接指导实现的规格说明"。以下第二轮建议更偏实现层面的风险控制。

---

## 2. 第二轮具体建议

### 2.1 checkpoint_crc 算法未明确（§4.4）

当前文档只写了 `checkpoint_crc` 但没有指定算法。这会导致实现时自由裁量，未来不兼容。

**建议明确为 CRC32C（Castagnoli）**：

| 算法   | 速度                           | 碰撞率       | 理由                                                                    |
| ------ | ------------------------------ | ------------ | ----------------------------------------------------------------------- |
| CRC32C | 极快（x86 有硬件加速 SSE 4.2） | 对短数据足够 | checkpoint 本身很小（约 100 字节），CRC32C 足以检测损坏                 |
| xxHash | 快                             | 更低         | 但 checkpoint 场景下不需要这么强                                        |
| blake3 | 快                             | 加密级       | overkill，blob 的 `content_hash` 已用 blake3，checkpoint 不需要同等强度 |

- CRC32C 在 Rust 中有成熟实现：[`crc32c` crate](https://crates.io/crates/crc32c)，底层直接用 SSE 4.2 硬件指令
- 参考：RocksDB 的 block checksum 也用 CRC32C
  - [RocksDB wiki - Format](https://github.com/facebook/rocksdb/wiki/Rocksdb-BlockBasedTable-Format)

> [!IMPORTANT]
> 必须在文档中明确写 "checkpoint_crc 使用 CRC32C"，否则实现者可能选择 CRC32（不同多项式）或其他算法，导致格式不确定。

### 2.2 fsync 策略需要比 "flush / fsync" 更具体（§6）

当前文档步骤 10 只写了 `flush / fsync`。在 Windows 上需要明确：

**关键事实**：

- Windows 的 `FlushFileBuffers` 会向驱动发送 `FLUSH_CACHE` 命令，**确保数据持久化到物理磁盘**
  - 参考：[Microsoft Learn: FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)
- 但 `FlushFileBuffers` **不保证部分写入的原子性**——如果 flush 过程中断电，可能出现 checkpoint 写一半的情况

**建议明确的写入顺序**（§6 步骤 9-10 应细化为）：

```text
9a. append 所有新 blob               → 此时不 fsync
9b. append 新 manifest               → 此时不 fsync
9c. FlushFileBuffers()                → 确保 blob + manifest 落盘
9d. append 新 checkpoint              → 写入 checkpoint
9e. update file-end footer            → 覆写末尾 8+8 字节
9f. FlushFileBuffers()                → 确保 checkpoint + footer 落盘
```

**为什么要两次 flush？**

- 第一次 flush 保证 blob + manifest 已在盘上
- 然后写 checkpoint + footer
- 第二次 flush 保证 checkpoint 已在盘上
- 如果在 9c 和 9f 之间崩溃：旧 checkpoint 仍有效，新 blob/manifest 只是"未被引用的尾部垃圾"
- 这是 **write-ahead 的思路**：先保证数据，再保证指针

> 参考：SQLite WAL 也是先写 WAL 帧，再更新 WAL index
> [SQLite WAL format](https://www.sqlite.org/wal.html)

### 2.3 Manifest 尺寸膨胀风险（§4.3）

当前设计每次快照都生成一个**完整 manifest**（包含所有文件的 entry）。这引出一个问题：

**快速估算**：

- 假设备份 10,000 个文件
- 每个 entry 约 200 字节（JSON 格式，含 path、blake3 hex、offset 等）
- 10,000 × 200 = **2 MB** per manifest

对于 XunYu 的开发者工具场景（通常 < 10,000 文件），这可以接受。但需要在文档中明确上限假设。

**建议补充**：

1. 在 §3（MVP 范围）中增加一个 **规模假设**：
   ```
   MVP 设计目标文件数：≤ 50,000
   MVP 设计目标单容器大小：≤ 10 GB
   ```
2. 超过此范围需要评估是否切换到二进制 manifest 或分片 manifest

> 参考：restic 在 150M blob 时 index 达到 6-7 GB，需要 22 GB RAM
> [restic forum - Large repository RAM usage](https://forum.restic.net/t/high-memory-usage-with-large-repository/4292)

### 2.4 blob_id = content_hash 的含义需要进一步明确（§4.2 补充）

文档写了 "blob_id 可以直接等于 content_hash"。这个决定有两个隐含后果需要明确：

**后果 1：同内容文件只存一份 blob**

- 如果两个文件（不同路径）内容完全相同，它们引用同一个 `blob_id`
- 这正是文件级去重的实现基础，是好事
- 但 compact 时需要注意：只要任何 manifest entry 引用了某个 `blob_id`，该 blob 就不能被清除

**后果 2：blob record 的 `content_hash` 字段变成与 `blob_id` 冗余**

- 如果 `blob_id == content_hash`，那 blob record 内部不需要再单独存 `content_hash`
- 建议 **去掉 blob record 中的 `content_hash` 字段**，只保留 `blob_id`（它本身就是 blake3 hash）
- 验证时直接对 payload 算 blake3，与 `blob_id` 对比即可

这样可以少一个 32 字节字段，简化 record 结构。

### 2.5 Footer 覆写的异常处理（§4.5）

file-end footer 的更新是 **覆写**（不是 append），这是整个 append-only 设计中唯一的覆写操作。

需要明确：

1. **footer 损坏时的 fallback**：如果 footer 的 magic 校验失败，应从文件头开始顺序扫描所有 record，找到最后一个完整 checkpoint
2. **footer 更新失败时**：footer 更新是写入流程的最后一步（§2.2 中的 9f 之后）。如果此时崩溃，footer 指向旧 checkpoint，但新 checkpoint 已经写入——下次打开时仍然走 fallback 扫描即可从旧 checkpoint 恢复

**建议在文档中增加一句**：

> footer 仅用于加速定位。如果 footer 校验失败，reader 必须退化为顺序扫描模式。

### 2.6 首次创建流程需要单独描述（§6）

当前 §6 只描述了"更新现有 .xunbak"的流程。但 **首次创建** 有一些差异：

1. 没有旧 checkpoint → 跳过步骤 2-3
2. 没有旧 manifest → 所有文件都视为"新增"
3. 需要先写入 Header（64 字节）

建议在 §6 开头增加一个简短的首次创建分支：

```text
首次创建流程：
1. 创建新文件
2. 写入 Header（64 字节）
3. 进入后续流程（从步骤 4 开始，baseline 为空）
```

### 2.7 Phase 分期与实际依赖关系（§12）

当前分期：

- Phase 1：定义格式 + 写入 + 恢复 + 压缩
- Phase 2：文件级复用 + append-only checkpoint + verify
- Phase 3：compact + 历史快照 + chunk 评估

**建议调整**：

将 "content_hash 复用旧 blob" 从 Phase 2 提前到 **Phase 1**。

理由：

1. §6 步骤 6 已经写了 "查找相同 content_hash → 复用旧 blob"
2. 如果 Phase 1 不做这个，那 Phase 1 每次写入都是全量，容器膨胀会非常快
3. 这个功能实现成本很低（只是一次 HashMap 查找），不应推迟

同时，"append-only checkpoint 更新" 实际上是 Phase 1 就必须做的（否则第二次备份时无法读取上一次的 checkpoint）。建议检查 Phase 1 和 Phase 2 的边界是否准确。

---

## 3. 第二轮最关键的 3 条建议

| 优先级 | 建议                               | 原因                                            |
| ------ | ---------------------------------- | ----------------------------------------------- |
| **P0** | 明确 `checkpoint_crc = CRC32C`     | 避免实现歧义，格式规范必须确定性                |
| **P0** | 细化 fsync 策略为两阶段 flush      | 写入持久性保证是备份工具的生命线                |
| **P1** | 将 content_hash 复用提前到 Phase 1 | 否则首个 MVP 每次都全量写入，容器会立刻膨胀失控 |

---

## 4. 参考资料（本轮新增）

1. **CRC32C 硬件加速**
   - [crc32c crate](https://crates.io/crates/crc32c)
   - [RocksDB Block Format](https://github.com/facebook/rocksdb/wiki/Rocksdb-BlockBasedTable-Format)

2. **Windows FlushFileBuffers**
   - [Microsoft Learn: FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)
   - [MSDN blog: Durability guarantees](https://devblogs.microsoft.com/oldnewthing/20100319-00/?p=14553)

3. **manifest 规模膨胀**
   - [restic forum: High RAM with large repo](https://forum.restic.net/t/high-memory-usage-with-large-repository/4292)
   - [kopia issue: manifest size scaling](https://github.com/kopia/kopia/discussions/2632)

4. **SQLite WAL 两阶段写入**
   - [SQLite WAL format](https://www.sqlite.org/wal.html)
