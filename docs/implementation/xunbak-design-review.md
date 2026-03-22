# `.xunbak` 单文件容器 MVP 设计审核建议

> 本文基于对 XunYu backup/restore 全部核心代码（约 2400 行）的深度阅读，以及 restic、borg、kopia、duplicacy、tar 等主流备份工具的公开设计文档调研，对 `Single-File-Xunbak-MVP-Design.md` 提出审核建议。
>
> **所有建议仅供参考，不修改原设计文档。**

---

## 1. 整体评价

设计文档的核心思路——"append-only 单文件容器 + blob 级 zstd 压缩 + checkpoint 定位"——是合理的。这个方向与 kopia 的 pack file 思路有相似之处，同时通过 append-only 简化了写入路径，非常适合 MVP。

> [!TIP]
> 整体方向正确，以下建议主要集中在 **具体实现层面的风险点** 和 **可借鉴的成熟工程实践**。

---

## 2. 按章节的具体建议

### 2.1 Header（§4.1）

**现状**：Header 固定长度，含 `magic`、`format_version`、`flags`、`created_at`、`reserved`。

**建议**：

| 项               | 建议                                                    | 来源参考                                                                                                                                          |
| ---------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Header 长度      | 明确固定为 **64 或 128 字节**，写入文档                 | restic pack format 用固定 header 便于工具快速识别<br>[restic design doc](https://restic.readthedocs.io/en/latest/100_references.html#pack-format) |
| 字节序           | 明确声明使用 **little-endian**（Windows 原生字节序）    | 减少歧义，`tar` 和 `zip` 因字节序问题历史上产生过兼容问题                                                                                         |
| `format_version` | 建议同时记录 **最低兼容版本号**（`min_reader_version`） | SQLite 的 [file format](https://www.sqlite.org/fileformat.html) 同时记录 `read_version` 和 `write_version`，让旧版工具能明确判断是否能读          |

### 2.2 Blob Record（§4.2）

**现状**：每个变更文件存为一个 blob，含 `blob_id`、`path_hash`、`content_hash`、`codec`、`raw_size`、`stored_size`、`payload`。

**建议**：

#### 2.2.1 blob 头部增加 `record_len` 字段

当前设计中 blob 有 `stored_size`，但没有 **整个 record（header + payload）的总长度**。

- 增加一个 `record_len` 字段（blob header 自身长度 + payload 长度），可以让读取端在不理解 blob 内部结构的情况下 **直接跳过整个 record**
- 这是 kopia pack format 和 restic pack format 都采用的做法
  - kopia: 每个 pack 尾部有 index，通过 offset + length 定位每个 content
  - restic: pack header 记录 blob 类型与长度，读取端可按偏移跳跃
  - 参考：[kopia architecture - Repository Format](https://kopia.io/docs/advanced/architecture/)

> [!IMPORTANT]
> 没有 `record_len`，在崩溃恢复场景下扫描 truncated blob 会非常困难——你必须尝试解析 blob header 每个字段来推算长度，而不是简单跳过。

#### 2.2.2 `path_hash` 的意义需要澄清

当前设计同时存了 `path_hash` 和 manifest 中的 `path`。

- 如果 `path_hash` 用于快速去重查找，那它应该放在 manifest 的 entry 里，而不是 blob record 里
- 如果 `path_hash` 纯粹用于 blob 关联，则 `blob_id` 已经承担了这个功能
- 建议 **去掉 blob record 中的 `path_hash`**，减少冗余

#### 2.2.3 小文件聚合（post-MVP 考虑）

当前设计是一个文件 = 一个 blob。对于大量小文件（如 `.gitignore`、配置文件），每个 blob 都有独立的 header 开销。

- kopia 将多个小 content 聚合到一个 20-40 MB 的 pack 里
  - 参考：[kopia architecture](https://kopia.io/docs/advanced/architecture/)
- MVP 不做很合理，但建议在文档中 **显式标注为 Phase 3+ 待评估项**，避免格式设计与未来聚合不兼容

### 2.3 Snapshot Manifest（§4.3）

**现状**：JSON 序列化，每个 entry 含 `path`、`blob_id`、`content_hash`、`size`、`mtime_unix`、`codec`、`blob_offset`、`blob_len`。

**建议**：

#### 2.3.1 Manifest 序列化格式：JSON → 考虑 MessagePack

设计文档说"MVP 可以接受 manifest 用 JSON 序列化"。我先列出代价：

| 场景            | JSON 的问题                                                              |
| --------------- | ------------------------------------------------------------------------ |
| 文件数 > 10,000 | JSON 文本体积可能达到 MB 级别，解析开销显著上升                          |
| 二进制 hash     | `content_hash`（blake3，32 bytes）做 hex 编码后变成 64 字节文本，膨胀 2x |
| 读取延迟        | JSON 解析比二进制反序列化慢一个数量级                                    |

**替代方案**：

- **MessagePack**（`rmp-serde`）：borg 所有内部元数据都用 msgpack
  - 参考：[Borg internals - Data structures](https://borgbackup.readthedocs.io/en/2.0.0b19/internals/data-structures.html)
- **bincode**（Rust 原生且零拷贝可能）：与 `serde` 集成最好
- 复杂度差异很小，因为 `serde` derive 通用，切换序列化器只是换一行 `to_vec` 调用

> [!TIP]
> 如果坚持 MVP 用 JSON，建议在 manifest 外层加一个 `codec` 字段（`json` / `msgpack` / `bincode`），让格式可演进。

#### 2.3.2 补充 entry 的 `permissions` / `attributes` 字段

当前 entry 只记录了 `mtime_unix`。在 Windows 场景下：

- 文件的 **只读属性**（`FILE_ATTRIBUTE_READONLY`）、**隐藏属性**（`FILE_ATTRIBUTE_HIDDEN`）也应该保留
- restic 在 Windows 下会记录 `GenericAttributes` 包含这些信息
  - 参考：[restic backup docs](https://restic.readthedocs.io/en/latest/040_backup.html)
- 建议至少预留一个 `attrs` 或 `win_attributes: u32` 字段（对应 `GetFileAttributesW` 返回值）

#### 2.3.3 补充 `ctime` 字段

- `mtime` 只反映"内容最后修改时间"
- `ctime`（在 Windows 上即 creation time）对于判断文件是否被 rename/move 后重建有意义
- borg 的 files cache 使用 `inode + size + mtime + ctime` 四元组做变更判定
  - 参考：[Borg internals - Files cache](https://borgbackup.readthedocs.io/en/2.0.0b19/internals/data-structures.html#files-cache)

### 2.4 Checkpoint（§4.4）

**现状**：Checkpoint 在每次写入最后追加，含 `snapshot_id`、`manifest_offset`、`manifest_len`、`container_end`、`checkpoint_crc`。

**建议**：

#### 2.4.1 Checkpoint 定位策略需要明确

设计文档说"打开容器时以最后一个完整 checkpoint 为准"。但 **如何找到最后一个 checkpoint？**

两种常见策略：

| 策略                 | 优点     | 缺点                                                       | 谁在用                             |
| -------------------- | -------- | ---------------------------------------------------------- | ---------------------------------- |
| **尾部固定偏移回扫** | 快，O(1) | 需要固定 checkpoint 大小，或在文件末尾 N 字节存一个 footer | kopia（pack 尾部嵌入 local index） |
| **从头顺序扫描**     | 简单     | 容器很大时慢                                               | tar append（需要全扫）             |

**建议**：采用 **file-end footer** 策略：

1. 在容器最末尾固定保留 **8 字节** 存最后一个 checkpoint 的 offset
2. 打开文件时 `seek(-8, SEEK_END)` 读 offset → 直接定位 checkpoint
3. 每次写入完成后更新这 8 字节

> [!CAUTION]
> 如果不做 footer，则必须从头顺序扫描所有 record 找 checkpoint。当容器达到 GB 级别时，这会严重影响打开速度。这与"速度快"的目标矛盾。

#### 2.4.2 Checkpoint 应包含 `blob_count` 和 `total_blob_bytes`

- 这样在 `compact` 操作前可以快速估算"当前容器的膨胀率"
- 不需要读完整个 manifest 就能判断是否值得 compact

### 2.5 压缩策略（§5）

**现状**：每个 blob 单独 zstd 压缩，默认 level 3。

**建议**：

#### 2.5.1 考虑 zstd seekable format

zstd 官方定义了 [Seekable Format](https://github.com/facebook/zstd/blob/dev/contrib/seekable_format/zstd_seekable_compression_format.md)，允许对大文件分 frame 压缩，支持随机访问解压。

Rust 生态中已有成熟实现：

- [`zeekstd`](https://github.com/nicktehrany/zeekstd)：完整的 Rust 实现，支持按 frame 解压
- [`zstd_framed`](https://docs.rs/zstd_framed)：支持同步和异步 I/O

**对 `.xunbak` 的意义**：

- 当前设计是每个 blob 独立压缩，这本身就是一种"frame per blob"的模式
- 如果未来考虑聚合小文件到单个 blob，seekable format 可以让聚合后的 blob 仍然支持单文件级别的随机解压
- **MVP 不需要改**，但值得在设计中提及兼容性预留

#### 2.5.2 "不压缩" 阈值建议调整

设计文档建议 `stored_size >= raw_size * 0.98` 时回退为 `none`。

- 建议改为 `0.95`（5% 收益阈值）
- 原因：zstd 压缩本身有固定 overhead（frame header 约 12-18 字节），如果只压缩了 2%，解压时的 CPU 开销可能不值得这点空间节省
- kopia 默认用 zstd-fastest（level 1），只在确认有明显收益时才保留压缩结果

#### 2.5.3 zstd dictionary（post-MVP）

- zstd 支持 [dictionary compression](https://facebook.github.io/zstd/#small-data)，对大量结构相似的小文件（如 JSON 配置、源代码）效果显著
- 可在 Phase 3 评估是否为特定项目类型预训练 dictionary

### 2.6 备份写入流程（§6）

**现状**：scan → diff → append blob → manifest → checkpoint → fsync。

**建议**：

#### 2.6.1 文件级 diff 策略需要与现有 baseline 对齐

当前 `backup` 模块的 diff 判定逻辑是 `size + mtime`（参见 `diff.rs` 中的 `compute_diff`）。新的 `.xunbak` 流程也应该沿用同样的策略。

但有一个关键差异：

- 现有 backup 的 baseline 来源是 **上一个备份目录/zip 中的实际文件元数据**
- `.xunbak` 的 baseline 应该来源于 **容器内最新 manifest 中记录的元数据**

这意味着：

1. manifest 中 **必须** 记录 `mtime` 和 `size`（当前设计已有 ✓）
2. diff 逻辑可以直接复用现有的 `compute_diff` 骨架，只需要把 `HashMap<String, FileMeta>` 的数据源从文件系统切换到 manifest 解析

#### 2.6.2 "重复内容" 检测不应只靠 mtime

当前设计步骤 6 中"计算 blake3"是对 **新增/变更** 文件做的。

建议增加一步：

1. 计算 blake3 后，先查当前 manifest 中是否已存在相同 `content_hash` 的 blob
2. 如果已存在（比如文件 rename 但内容不变），直接复用 `blob_id`，**不写新 blob**

这正是 restic 和 kopia 的核心去重逻辑，但在文件级粒度上实现成本很低。

> 参考：restic 的 content_hash 去重使用 SHA-256
> [restic design - Backups and Deduplication](https://restic.readthedocs.io/en/latest/100_references.html)

### 2.7 恢复流程（§7）

**现状**：打开 → 定位 checkpoint → 读 manifest → 筛选 entry → 读 blob → 解压 → 写文件。

**建议**：

#### 2.7.1 并行恢复

当前 `restore_core.rs` 已经使用 rayon 做并行恢复（`par_iter` 并行 copy）。`.xunbak` 恢复时同样应该并行。

但 append-only 单文件容器有一个限制：**所有 blob 在同一个文件中**，多线程读同一个文件需要注意：

- Windows 上 `ReadFile` 支持并行读（每个线程用自己的 file handle 或 `OVERLAPPED`）
- 但如果用 Rust 的 `std::fs::File`，则需要 `clone()` 或每线程 `open()` 来获取独立的 seek position
- 建议明确采用 **每线程独立 file handle** 的策略

#### 2.7.2 恢复时的文件属性还原

与 §2.3.2 对应：如果 manifest 记录了 Windows 文件属性，恢复时应该调用 `SetFileAttributesW` 还原。

### 2.8 容器膨胀与 compact（§9）

**现状**：MVP 不自动 compact，但预留接口。

**建议**：

#### 2.8.1 compact 时要注意 "原子替换" 在 Windows 上的实现

设计文档说"原子替换旧 `.xunbak`"。在 Windows 上：

- 没有 POSIX `rename` 的原子语义
- 建议使用 `MoveFileExW` + `MOVEFILE_REPLACE_EXISTING` 或 `ReplaceFileW`
  - 参考：[Microsoft Learn: ReplaceFile](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilea)
- 更安全的做法是：写新文件 → rename 旧文件为 `.xunbak.bak` → rename 新文件 → 删除 `.bak`

#### 2.8.2 compact 触发时机建议

虽然 MVP 不做自动 compact，建议预留一个简单的指标：

```
waste_ratio = 1 - (referenced_blob_bytes / total_container_bytes)
```

当 `waste_ratio > 50%` 时提示用户运行 compact。这个指标可以从 checkpoint 中的 `blob_count` + `total_blob_bytes` 快速计算。

---

## 3. 容器格式对比总结

| 维度                  | `.xunbak`（当前设计） | restic pack   | kopia pack      | borg segment      | tar               |
| --------------------- | --------------------- | ------------- | --------------- | ----------------- | ----------------- |
| 单文件                | ✅                    | ❌（多文件）  | ❌（多文件）    | ❌（多文件）      | ✅                |
| Append-only           | ✅                    | ✅            | ✅              | ✅                | ✅                |
| 增量更新              | ✅ 文件级             | ✅ chunk 级   | ✅ chunk 级     | ✅ chunk 级       | ❌ 需外部 snar    |
| 快速定位最新快照      | ⚠️ 需要扫描或 footer  | ✅ index 文件 | ✅ index 文件   | ✅ hints file     | ❌ 需全扫         |
| 崩溃安全              | ✅ checkpoint         | ✅ 只读 pack  | ✅ 只读 pack    | ✅ segment        | ❌ 可能 truncated |
| blob 级 random access | ✅                    | ✅            | ✅              | ✅                | ❌                |
| 内建压缩              | ✅ zstd/blob          | ✅ 加密       | ✅ zstd/content | ✅ lz4/zstd/chunk | ❌ 需外层         |
| Windows 优化          | 可做                  | 一般          | 一般            | 一般              | 不适合            |

---

## 4. 最关键的 3 条建议

以下是按优先级排序的最重要建议：

### 建议 1：增加 file-end footer 定位 checkpoint（**关键**）

不加 footer 的代价是每次打开容器都要从头扫描，这在容器增长到 GB 级别后会成为严重瓶颈，直接违背"速度快"的目标。

实现成本极低：每次写入结束后在文件末尾追加 8 字节 checkpoint offset。

### 建议 2：blob record 增加 `record_len` 字段（**关键**）

这是崩溃恢复和格式向前兼容的基础。没有 `record_len`，未来任何 blob header 字段的增删都会破坏旧版本的跳过能力。

### 建议 3：Manifest 序列化预留 codec 字段（**推荐**）

MVP 用 JSON 没问题，但在 manifest 外层加一个 `codec` 标记，让未来切换到 bincode/msgpack 不需要 breaking change。

---

## 5. 参考资料

1. **restic repository format**
   - [restic design doc - Pack format](https://restic.readthedocs.io/en/latest/100_references.html#pack-format)
   - [restic backup docs](https://restic.readthedocs.io/en/latest/040_backup.html)

2. **borg internals**
   - [Borg data structures](https://borgbackup.readthedocs.io/en/2.0.0b19/internals/data-structures.html)
   - [Borg FAQ - Files cache](https://borgbackup.readthedocs.io/en/stable/faq.html)

3. **kopia architecture**
   - [kopia Repository Format](https://kopia.io/docs/advanced/architecture/)

4. **duplicacy design**
   - [duplicacy Lock-Free Deduplication](https://github.com/gilbertchen/duplicacy/wiki/Lock-Free-Deduplication)
   - [duplicacy IEEE paper](https://ieeexplore.ieee.org/document/7920067)

5. **zstd seekable format**
   - [zstd seekable format spec](https://github.com/facebook/zstd/blob/dev/contrib/seekable_format/zstd_seekable_compression_format.md)
   - [zeekstd crate](https://github.com/nicktehrany/zeekstd)
   - [zstd_framed crate](https://docs.rs/zstd_framed)

6. **SQLite file format**
   - [SQLite file format spec](https://www.sqlite.org/fileformat.html)

7. **Windows 文件操作**
   - [Microsoft Learn: ReplaceFile](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilea)
   - [Microsoft Learn: MoveFileEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexa)
