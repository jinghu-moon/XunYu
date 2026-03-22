# 单文件 `.xunbak` 容器完整设计方案

生成时间：2026-03-22

> 本文是 XunYu backup/restore 组件的增强设计方案，不是冻结的外部公开格式规范。
> 当前目标是单仓库单实现，不承诺第三方 reader/writer 兼容。
> 精确二进制布局在实现阶段补齐；本文定义的是格式结构、约束与演进边界。

---

## 1. 目标

`.xunbak` 的目标是提供一条新的备份主路线，满足以下要求：

1. **默认只有一个备份文件**
2. **必要时支持分卷**
3. **支持增量更新**
4. **支持可选压缩算法**
5. **压缩速度快**
6. **恢复速度快**
7. **占用尽量小**
8. **整体性能高**
9. **默认只保留最新快照；格式层预留多版本能力**
10. **支持完整性校验与 compact**

换句话说，`.xunbak` 不是"当前目录/zip 备份的附属导出格式"，而是未来应可独立承担主备份职责的容器格式。

重要说明：

1. `.xunbak` **不是 git**
2. 默认行为是 **only latest**：只维护最新快照
3. 多版本快照浏览与恢复是 **格式预留能力**，不是当前必做能力
4. compact 默认只看最新 manifest

---

## 2. 设计原则

### 2.1 单文件优先

默认产物形态必须是：

```text
project.xunbak
```

不采用以下主模式：

1. 多目录快照
2. `v1.zip / v2.zip / v3.zip`
3. 外层 zip + 内层 delta 链

### 2.2 分卷是工程保护，不是否定单文件

分卷并不与"单文件优先"冲突。

分卷只用于：

1. 文件系统单文件大小限制
2. 传输介质 / 网盘单文件限制
3. 容器体积过大

因此：

1. 默认不分卷
2. 分卷是可选能力
3. `.xunbak` 的主心智模型仍然是"一个备份文件"

### 2.3 append-only 优先

写入路径以 append-only 为主：

1. 新数据追加
2. 新 manifest 追加
3. 新 checkpoint 追加
4. footer 只做最后一步覆写

不把"原地改 blob / 原地改 manifest"作为主路径。

### 2.4 文件级增量优先；超大文件允许物理分段例外

完整设计必须允许未来进入 chunk/pack 模式，但当前基线设计仍以文件级 blob 为核心：

1. 默认一个文件 = 一个 blob
2. 未变化文件复用旧 blob
3. 变化文件写入新 blob
4. 仅当单个文件过大，无法装入单卷或受文件系统硬上限约束时，允许把同一个文件拆成有序 `parts[]`
5. 这种 `parts[]` 只是物理存储分段，不改变文件级 diff / restore / verify 的语义

这样既符合当前 XunYu 的工程节奏，也保留了向 chunk 级演进的空间。

### 2.5 压缩必须内建在容器内部

压缩策略不应再依赖"外层 zip"。

采用：

1. 每个 blob 独立记录 codec
2. 容器整体不再二次 zip

这样才能同时兼容：

1. 单文件
2. 增量更新
3. 快速恢复

### 2.6 默认 only latest

`.xunbak` 不做 git 式历史系统：

1. 默认只维护最新快照
2. compact 只保留最新 manifest 引用的 blob
3. 不实现多版本浏览、快照列表、快照裁剪
4. 格式层预留 `prev_checkpoint_offset`，允许未来按需演进

### 2.7 格式层先预留，再按阶段实现

完整设计文档必须明确：

1. 哪些能力在格式层必须现在预留
2. 哪些能力可以在实现阶段延后

例如：

1. `FLAG_SPLIT`
2. `FLAG_ALIGNED`
3. `manifest_type`
4. codec 枚举
5. `prev_checkpoint_offset`

这些都应进入格式层，不应等到实现时再补。

---

## 3. 功能范围

## 3.1 当前目标能力

1. 创建 / 更新 `.xunbak`
2. 最新快照恢复
3. 单文件恢复
4. glob 恢复
5. verify（分级）
6. compact（latest-only）
7. 分卷写入 / 读取
8. 可选压缩算法
9. 元数据导出

## 3.2 格式可演进能力（非当前实现）

以下能力在格式层已预留，但不在当前阶段实现：

1. 指定快照恢复（依赖 checkpoint chain）
2. 快照列表（依赖 checkpoint chain）
3. 快照裁剪 / prune（依赖 checkpoint chain）
4. delta manifest（依赖 `manifest_type`）

## 3.3 规模假设

当前格式设计目标：

1. 目标文件数：`<= 50,000`
2. 目标单容器大小：`<= 10 GB`

说明：

1. 在该范围内，完整 manifest 采用文本 JSON 仍可接受
2. 若明显超出该范围，应优先评估：
   - 二进制 manifest
   - manifest 分片
   - 小文件聚合
   - chunk/pack 模式

---

## 4. 容器逻辑结构

默认容器：

```text
project.xunbak
```

逻辑布局：

```text
[Header]
[Blob Record...]
[Blob Record...]
[Manifest Record]
[Checkpoint Record]
[Footer]
```

多次更新后：

```text
[Header]
[Blob...]
[Manifest 1]
[Checkpoint 1]

[Blob...]
[Manifest 2]
[Checkpoint 2]

[Blob...]
[Manifest 3]
[Checkpoint 3]
[Footer -> Checkpoint 3]
```

读取策略：

1. 优先从 footer 定位最后一个 checkpoint
2. footer 失效时退化为顺序扫描
3. 以最后一个完整 checkpoint 为准
4. 分卷模式下，先按 `set_id + volume_index` 组装卷集，再从最高 `volume_index` 的卷进入

---

## 5. 统一 Record 编码

为了支持：

1. 未知 record 向前兼容
2. footer fallback 顺序扫描
3. 分卷场景的结构恢复
4. paranoid verify

建议所有 record 统一采用固定前缀：

```text
offset  size  field
0       1     record_type (u8)
1       8     record_len  (u64, little-endian, payload length only)
9       4     record_crc  (CRC32C，覆盖范围按 record_type 定义)
13      ...   payload
```

建议 `record_type` 编码：

```text
0x01 = blob
0x02 = manifest
0x03 = checkpoint
0x04 = reserved
0x05 = pack       (预留给小文件聚合)
0x06 = index      (预留给显式索引)
0x07 = tombstone  (预留给在线清理/逻辑删除)
0x08-0xFF = reserved
```

读取端规则：

1. 先读 `record_type`
2. 再读 `record_len`
3. 校验 `record_crc`
4. 已知类型则解析 payload
5. 未知类型则跳过 `record_len`

统一约定：

1. `record_len` = payload 长度，**不含** 13 字节 record 前缀
2. 整条 record 的总长度 = `13 + record_len`
3. 顺序扫描时，读完前缀后前进 `record_len` 即可到达下一条 record

`record_crc` 的意义：

1. 顺序扫描时如果 `record_crc` 校验失败，可以确定该 record 已损坏
2. 没有 `record_crc`，一旦任意 record 的 `record_len` 被破坏，顺序扫描会失步
3. 使 paranoid verify 有能力逐条验证 record 结构完整性

`record_crc` 覆盖范围约定：

1. `blob record`：覆盖 `record_type + record_len + blob 固定头部`，**不覆盖压缩后的 data payload**
2. `manifest / checkpoint / future small record`：覆盖 `record_type + record_len + 全 payload`
3. blob 的大数据体完整性由 `blob_id / content_hash` 保证，不要求 `record_crc` 再覆盖一遍
4. `record_crc` 的计算始终**不包含自身这 4 字节**

这样设计的原因：

1. blob record 可能很大，若 `record_crc` 覆盖整个压缩 payload，则必须先完整缓冲 payload 才能回填前缀 CRC
2. 当前设计优先支持 append-only 与流式写入，因此 blob record 的 `record_crc` 只负责结构校验，不负责数据体校验
3. manifest/checkpoint 体积很小，继续覆盖全 payload，便于 detect corruption

注意：

1. **Header 不走统一 record 编码**
2. Header 始终固定 64 字节，以 `magic` 开头
3. 统一 record 编码从 Header 之后的第一个 record 开始
4. **Footer 也不走统一 record 编码**（Footer 固定 24 字节，有自己的 magic 和 CRC）

---

## 6. Header

Header 建议：

1. 固定长度：**64 字节**
2. 字节序：**little-endian**

字段建议：

1. `magic = XUNBAK\0\0`
2. `write_version`
3. `min_reader_version`
4. `flags`
5. `created_at_unix`
6. `reserved`

说明：

1. magic 固定，不承载版本路由
2. 版本兼容完全由 `write_version / min_reader_version` 决定
3. 这样未来 reader 只需要识别一个 magic，再按版本字段分流解析逻辑
4. `created_at_unix` 表示**容器首次创建时间**，不随增量更新变化

推荐布局：

```text
offset  size  field
0       8     magic
8       4     write_version
12      4     min_reader_version
16      8     flags
24      8     created_at_unix
32      32    reserved
```

分卷字段（`FLAG_SPLIT` 启用时从 `reserved` 中分配）：

```text
offset  size  field
32      2     volume_index
34      6     reserved
40      8     split_size
48      8     set_id
56      8     reserved（余量）
```

非分卷模式下 `reserved` 全部为 0。Header 不存权威 `total_volumes`，避免增量更新扩卷时回写所有旧卷。精确偏移在实现阶段确定。

### 6.1 Header Flags

建议至少预留：

1. `0x01 = FLAG_SPLIT`
2. `0x02 = FLAG_ALIGNED`

说明：

1. `FLAG_SPLIT` 表示该容器使用分卷
2. `FLAG_ALIGNED` 表示 blob payload 对齐到 4 KB 或更高扇区边界，用于未来的 Windows unbuffered I/O
3. 若启用 `FLAG_ALIGNED`，Phase 2 可评估 `FILE_FLAG_NO_BUFFERING | FILE_FLAG_SEQUENTIAL_SCAN`

要求：

1. reader 遇到未知 flag 不得 panic
2. 必须返回明确"不支持"或"需要更高版本 reader"

---

## 7. Blob Record

每个变更文件存为一个 blob record。

建议字段：

1. `record_type = blob`
2. `record_len`
3. `blob_id`
4. `blob_flags`
5. `codec`
6. `raw_size`
7. `stored_size`
8. `payload`

推荐 payload 固定头部布局：

```text
offset  size  field
0       32    blob_id         (blake3, 固定 32 字节)
32      1     blob_flags      (u8)
33      1     codec           (u8)
34      8     raw_size        (u64, little-endian)
42      8     stored_size     (u64, little-endian)
50      ...   data payload    (stored_size 字节)
```

说明：

1. blob payload 固定头部为 **50 字节**
2. 加上 13 字节 record 前缀，单个 blob 的固定元数据开销为 **63 字节**
3. `record_len` = `50 + stored_size + padding`

`blob_flags` 预留建议：

1. `0x01 = FLAG_ALIGNED`
2. `0x02 = FLAG_ENCRYPTED`
3. `0x04 = FLAG_DICT`
4. `0x08-0xFF = reserved`

### 7.1 关键约束

1. `blob_id = blake3(content)`
2. 不存 `path_hash`
3. 不在 blob record 中重复存 `content_hash`
4. `record_len` 必须覆盖 blob 固定头部 + data payload + padding（若启用对齐）
5. `record_crc` 仅覆盖 blob 固定头部，不覆盖 data payload

### 7.2 这样设计的意义

1. 同内容、不同路径只存一份 blob
2. compact 时只要仍有 manifest entry 引用该 `blob_id`，就必须保留
3. reader 可在不理解 payload 的情况下安全跳过整条 record

### 7.3 超大文件的 multipart 例外

默认仍是"一个文件 = 一个 blob"。但当单个文件满足以下任一条件时：

1. `raw_size > split_size`
2. 压缩后 `stored_size` 仍可能超过单卷硬上限
3. 目标文件系统或传输介质存在硬单文件限制

允许该文件拆成有序 `parts[]` 写入多个 blob record：

1. 每个 part 仍是普通 blob record，`blob_id = blake3(part_content)`
2. manifest entry 通过 `parts[]` 顺序引用这些 part
3. restore / verify 按 `parts[]` 顺序读取、解压并拼接回原文件
4. 这不是 chunk 级去重；diff、snapshot、restore 的语义仍以整个文件为单位
5. 当前 multipart 只解决**物理跨卷存储**问题，不引入 part 级增量更新语义
6. multipart 文件只要任一 part 对应的源文件内容发生变化，默认重写该文件的全部 parts

---

## 8. Manifest

Manifest 表达当前快照的完整文件视图。

建议字段：

1. `manifest_codec`
2. `manifest_type`
3. `manifest_version`
4. `snapshot_id`
5. `base_snapshot_id`
6. `created_at`
7. `source_root`
8. `snapshot_context`
9. `file_count`
10. `total_raw_bytes`
11. `entries`
12. `removed`

manifest record payload 固定前缀建议：

```text
offset  size  field
0       1     manifest_codec    (u8)
1       1     manifest_type     (u8)
2       2     manifest_version  (u16, little-endian)
4       ...   manifest body     (按 manifest_codec 序列化)
```

这样 reader 先读 4 字节固定前缀，即可确定如何解析后续 manifest body。

约定：

1. `manifest_codec / manifest_type / manifest_version` 位于 manifest record 的固定前缀中
2. manifest body 本身不重复存储这 3 个字段
3. manifest body 的逻辑内容从 `snapshot_id` 开始

每个 entry 建议包含：

1. `path`
2. `blob_id`
3. `content_hash`
4. `size`
5. `mtime_ns`
6. `created_time_ns`
7. `win_attributes`
8. `codec`
9. `blob_offset`
10. `blob_len`
11. `volume_index`
12. `parts`（可选，仅超大文件时）
13. `ext`（可选，未来扩展字段）

`parts[]` 中每个 part 必须包含：

1. `part_index`
2. `blob_id`
3. `codec`
4. `raw_size`
5. `stored_size`
6. `blob_offset`
7. `blob_len`
8. `volume_index`

说明：

1. 普通文件场景下，`content_hash` 等同于 `blob_id`
2. 超大文件场景下，`content_hash` 表示整个文件内容的 `blake3`，而 `parts[]` 中每个 part 的 `blob_id` 表示对应 part 的哈希
3. entry 中保留 `content_hash` 是为了让 manifest 在语义上自包含（不需要交叉引用 blob record 即可完成 verify）
4. 如果后续 `blob_id` 从 `content_hash` 进一步解耦（如 chunk/pack 阶段），两者可以不同
5. `blob_offset` 是**卷内偏移**（相对于该卷文件开头），不是全局偏移
6. `blob_offset` 指向 **blob record 起始位置**（即 `record_type` 所在字节）
7. `blob_len` 表示 **整条 blob record 的总字节数**（即 `13 + record_len`）
8. 普通文件使用单值字段：`blob_id / codec / blob_offset / blob_len / volume_index`
9. 超大文件使用 `parts[]`
10. `total_raw_bytes` 表示该 manifest 覆盖文件视图的原始总字节数，可用于恢复前磁盘空间预估和压缩率统计

权威来源约定：

1. `manifest` 是恢复时的权威定位源：`path / blob_offset / blob_len / codec / volume_index`
2. `blob record` 固定头部是交叉校验源：`blob_id / blob_flags / codec / raw_size / stored_size`
3. restore 时优先按 manifest 定位，读到 blob record 后必须校验两者一致
4. verify 时，manifest 与 blob record 若不一致，应直接报告 corruption

### 8.1 Manifest Codec

完整设计允许：

1. `json`
2. `msgpack`
3. `bincode`

要求：

1. `manifest_codec` 必须写入 manifest record 的固定前缀
2. `manifest_type` 必须写入 manifest record 的固定前缀
3. reader 必须先读固定前缀，再按 `manifest_codec` 选择反序列化器

建议：

1. 第一阶段可先用 `json`
2. 但格式层绝不能把 JSON 写死

### 8.2 Manifest Type（格式预留）

格式层预留 `manifest_type`：

1. `full`
2. `delta`

当前行为：

1. 始终写 `manifest_type = full`
2. 不实现 delta manifest

格式可演进：

1. 未来按需启用 `delta` manifest
2. delta manifest 需记录 `base_snapshot_id`、`entries`（新增/变更）、`removed`（删除路径列表）

`removed` 字段适用条件：

1. `manifest_type = full` 时，`removed` 可为空数组或省略，reader 不依赖它
2. `manifest_type = delta` 时，`removed` 是必需字段
3. 当前 Phase 1 始终写 `full`，因此 `removed` 默认写空数组即可

### 8.3 路径编码与规范化

路径建议统一为：

1. 编码固定为 UTF-8
2. 容器内分隔符固定为 `/`
3. 写入时保留原始大小写
4. Windows 上的路径比较与去重使用**大小写不敏感**语义

实现要求：

1. writer 构建 manifest 时，必须拒绝仅大小写不同但逻辑上冲突的路径
2. restore 到 Windows 时，把 `/` 转换为 `\`
3. 长路径恢复应使用长路径兼容方式处理，不得假设 `MAX_PATH = 260`

### 8.4 时间戳精度与纪元

时间字段统一为：

1. `mtime_ns` = Unix epoch 纳秒
2. `created_time_ns` = Unix epoch 纳秒

说明：

1. 它们都以 `1970-01-01 00:00:00 UTC` 为基准
2. writer 负责把 Windows `FILETIME`（1601 epoch, 100ns 单位）转换为 Unix epoch nanoseconds
3. reader 恢复到 Windows 时，再按平台 API 需要反向转换

### 8.5 Entry 扩展机制

为避免每次增加元数据都提升整个 manifest 主版本，entry 应预留：

1. `ext`：可选对象 / map
2. 未知 key 由旧 reader 忽略
3. 新元数据（如 ACL / ADS / symlink_target）优先放入 `ext`

### 8.6 Windows 元数据

由于 XunYu 是 Windows-first，manifest 应明确预留：

1. `win_attributes: u32`
2. `created_time_ns`

原因：

1. 只读、隐藏等属性有恢复价值
2. creation time 在部分恢复和比较场景中有价值

补充说明：

1. `win_attributes` 直接存储 `GetFileAttributesW` 返回的 `DWORD`
2. 恢复时优先还原 `readonly / hidden / archive / system`
3. `compressed / encrypted` 等文件系统级行为不保证恢复

---

## 9. Checkpoint

Checkpoint 是容器中的"最新入口"。

建议字段：

1. `record_type = checkpoint`
2. `snapshot_id`
3. `manifest_offset`
4. `manifest_len`
5. `manifest_hash`
6. `container_end`
7. `checkpoint_crc`
8. `blob_count`
9. `referenced_blob_bytes`
10. `total_container_bytes`
11. `prev_checkpoint_offset`
12. `total_volumes`

推荐 payload 固定布局：

```text
offset  size  field
0       16    snapshot_id
16      8     manifest_offset
24      8     manifest_len
32      32    manifest_hash
64      8     container_end
72      8     blob_count
80      8     referenced_blob_bytes
88      8     total_container_bytes
96      8     prev_checkpoint_offset
104     2     total_volumes
106     18    reserved
124     4     checkpoint_crc
```

说明：

1. checkpoint payload 固定大小建议为 **128 字节**
2. 加上 13 字节 record 前缀，整条 checkpoint record 约 **141 字节**
3. `reserved` 为未来统计字段预留扩展空间

字段语义明确：

1. `manifest_hash` = manifest record **整个 payload**（含 4 字节 manifest 固定前缀，不含 13 字节 record 前缀）的 `blake3` 哈希，用于校验 manifest 本身没有被损坏（见 §15.3）
2. `manifest_offset` = 卷内偏移，且指向 **manifest record 起始位置**（即 `record_type` 所在字节；分卷时指最后一卷内偏移，因为 manifest 始终在最后一卷）
3. `manifest_len` = manifest record 的总字节数（即 `13 + record_len`）
4. `container_end` = **最后一卷的文件长度**，不是逻辑总长度（因 checkpoint 始终在最后一卷，该值可用于检测最后一卷是否被截断）
5. `total_volumes` = 分卷集的总卷数；不分卷时为 `1`
6. `snapshot_id` 建议采用 **ULID**（16 字节，128 位），兼顾唯一性与按时间排序；高位时间戳有利于未来 `snapshot list` 的自然排序

精确二进制布局在实现阶段补齐。本文定义的是字段语义与约束。

`total_volumes` 放在 checkpoint 而非各卷 Header 中的原因：

1. 避免增量更新扩卷时需要回填所有旧卷 Header（违反 append-only）
2. checkpoint 始终在最后一卷，写入完成时卷数已确定
3. 打开时：读最后一卷 footer → checkpoint → `total_volumes`，然后校验同目录下是否有 `0..total_volumes-1` 所有卷

### 9.1 Checkpoint 校验

`checkpoint_crc` 建议明确为：

1. **CRC32C（Castagnoli）**

覆盖范围建议：

1. 覆盖 checkpoint record 的 **payload 部分**
2. 不含 13 字节 record 前缀
3. 不含 `checkpoint_crc` 字段自身

职责边界：

1. `record_crc` 负责检测整条 checkpoint record 的结构损坏
2. `checkpoint_crc` 负责检测 checkpoint payload 内部字段是否被破坏
3. `manifest_hash` 负责检测 manifest payload 是否被破坏
4. `record_crc + checkpoint_crc` 的双重存在是**有意冗余设计**，不是重复定义

原因：

1. checkpoint 很小
2. CRC32C 足以检测损坏
3. 实现简单，性能高

### 9.2 prev_checkpoint_offset（格式预留）

当前行为：

1. `prev_checkpoint_offset` 始终写 `0`
2. 不遍历 checkpoint chain

格式预留意义：

1. 允许未来按需启用多版本快照浏览与恢复
2. 字段成本：仅一个 `u64 = 0`
3. 如果未来启用，形成反向链表：`Footer -> CP3 -> CP2 -> CP1 -> 0`

### 9.3 为什么带统计字段

`blob_count / referenced_blob_bytes / total_container_bytes` 可直接服务：

1. compact 提示
2. 状态展示
3. 容器膨胀率估算

### 9.4 compact 后 snapshot_id 稳定性

compact 后，保留快照的 `snapshot_id` 必须保持不变。用户可能已经通过 `snapshot_id` 建立了外部引用（如日志、配置）。

compact 后字段变化约定：

1. **保持不变**：`snapshot_id`、逻辑文件视图（`path / content_hash / size / mtime_ns / created_time_ns / win_attributes`）
2. **允许变化**：`blob_offset / blob_len / volume_index`
3. **必然变化**：`manifest_hash`、`blob_count`、`referenced_blob_bytes`、`total_container_bytes`

---

## 10. Footer

Footer 只承担一个职责：

1. 快速定位最后一个 checkpoint

Footer 固定长度：**24 字节**

推荐布局：

```text
offset  size  field
0       8     footer_magic = XBKFTR\0\0
8       8     checkpoint_offset (u64, little-endian)
16      4     footer_crc32c
20      4     reserved / padding
```

`footer_crc32c` 覆盖 `footer_magic + checkpoint_offset`。

Reader 打开时固定 `seek(-24, SEEK_END)` 即可。

### 10.1 读取规则

分卷模式下的入口：

1. 先按文件名序号选择编号最大的卷作为候选最后一卷
2. 从该卷 `seek(-24, SEEK_END)` 读取 footer
3. 再通过 checkpoint 中的 `total_volumes` 反向校验卷集完整性

读取步骤：

1. `seek` 到文件尾
2. 读取 24 字节 footer
3. 校验 `footer_magic`
4. 校验 `footer_crc32c`
5. 校验 offset 是否在文件范围内
6. 合法则跳到 checkpoint

### 10.2 fallback 规则

如果 footer 不合法：

1. reader 应**尝试顺序扫描**，利用每条 record 的 `record_crc` 逐条校验并跳过
2. 分卷模式下扫描必须是**按卷感知**的：按 `volume_index` 从小到大逐卷扫描，每卷从 Header 后第一条 record 扫到卷尾
3. 对 blob record，扫描时还应交叉校验 `record_len == 50 + stored_size + padding`
4. 若 `record_len` 与固定头部中的 `stored_size` 明显不一致，应视为结构损坏并终止扫描
5. 遇到 `record_crc` 校验失败的 record 时扫描终止，以已扫到的最后一个完整 checkpoint 为准
6. 如果连第一条 record 就校验失败，reader 应直接报错

局限性说明：

1. 顺序扫描的可靠性依赖 `record_crc`
2. 当前设计没有 sync magic 可重同步标记，因此**顺序扫描不保证在任意损坏场景下恢复**
3. footer 是正常运行的关键入口；fallback 是尽力恢复机制，不是等价替代

### 10.3 增量更新时的 footer 处理

**单文件和分卷统一规则**：增量更新时，始终覆盖旧 footer 后继续追加。

单文件场景：

1. 更新时 `seek` 到 `文件长度 - 24`（旧 footer 起始位置）
2. 从该位置开始追加新 blob / manifest / checkpoint
3. 旧 footer 被新数据覆盖，不会残留在文件中间
4. 最后写入新 footer

分卷场景：

1. 打开旧最后一卷，`seek` 到 `该卷文件长度 - 24`（旧 footer 位置）
2. 从该位置开始追加新数据
3. 如果该卷剩余空间足够：所有新 blob 继续写入该卷
4. 如果该卷写满（接近 `split_size`）：在 record 边界切到新卷
5. checkpoint + 新 footer 始终写在最终的最后一卷末尾

关键约束：**旧卷中绝不会残留 footer**。因为增量更新始终从旧最后一卷的旧 footer 位置开始覆盖写入。这保证了：

1. 顺序扫描（卷 1 → 卷 N）不会遇到非 record 字节
2. paranoid verify 不会在卷中间被旧 footer 干扰
3. 所有卷（除最后一卷末尾 footer 外）的内容都是纯粹的 Header + record 序列

---

## 11. 分卷设计

### 11.1 目标

分卷是工程退化保护，不是主模式。

适用场景：

1. FAT32 单文件上限
2. NAS / 网盘单文件限制
3. 容器体积过大

### 11.2 命名建议

推荐：

```text
project.xunbak           ← 不分卷时
project.xunbak.001       ← 第 1 卷
project.xunbak.002       ← 第 2 卷
...
```

### 11.3 实现原则

1. 默认不分卷
2. 只在 **record 边界** 切卷
3. **不允许在单个 blob record 的 payload 中间切卷**；超大文件通过 `parts[]` 拆成多个 blob record（见 §7.3）
4. checkpoint + footer 始终在最后一卷
5. 每卷都带自己的 header
6. 增量更新时始终从旧最后一卷的旧 footer 位置覆盖续写（见 §10.3）

推荐布局：

```text
卷 1: [Header(vol=0)] [Blob...] [Blob...]
卷 2: [Header(vol=1)] [Blob...] [Manifest] [Checkpoint] [Footer]
```

每卷内容（除最后一卷末尾 footer 外）都是纯粹的 `Header + record 序列`，顺序扫描时按卷序号依次读取即可。

### 11.4 分卷元数据

元数据分布原则：**稳定字段放 Header，可变字段放 Checkpoint。**

Header 中（写入后不再修改）：

1. `FLAG_SPLIT`
2. `volume_index: u16` — 当前卷序号（从 0 开始）
3. `split_size: u64` — 每卷目标大小
4. `set_id: u64` — 分卷集标识

**`total_volumes` 不放在 Header 中**，而是放在 Checkpoint（见 §9）。原因：

1. 增量更新可能将分卷集从 N 卷扩展到 N+1 卷
2. 如果 `total_volumes` 在每卷 Header 中，扩卷时必须回填所有旧卷 Header — 违反 append-only 原则
3. 放在 Checkpoint 中，只需要在最后一卷写入一次，天然与 append-only 兼容

`split_size` 更新规则：

1. 同一分卷集的增量更新默认**不允许修改** `split_size`
2. writer 打开已有分卷集时，必须校验用户输入的 `split_size` 与最后一卷 Header 中记录的值一致
3. 不一致则直接报错，不做隐式迁移
4. 若用户需要更改 `split_size`，应通过 `compact/rewrite` 生成新的分卷集

Manifest entry 需要：

1. `volume_index`

`set_id` 说明：

1. 防止不同备份的分卷文件混在同一目录下被误识别为同一组
2. 打开时校验所有卷的 `set_id` 一致
3. 不一致则报错

### 11.5 分卷完整性校验

打开分卷集时的校验流程：

1. 读取最后一卷末尾的 footer → 定位 checkpoint
2. 从 checkpoint 读取 `total_volumes`
3. 扫描同目录下同名分卷文件，检查 `0..total_volumes-1` 是否齐全
4. 校验每卷 Header 的 `set_id` 一致
5. 校验每卷 Header 的 `volume_index` 与文件名序号匹配
6. 任一校验失败则报错

### 11.6 分卷场景下的崩溃恢复

分卷下与单文件相同：

1. 只认最后一个完整 checkpoint
2. checkpoint + footer 始终在最后一卷

典型故障：

```text
卷 1: [Header] [Blob 1] [Blob 2]           <- 已完成
卷 2: [Header] [Blob 3] [Manifest ...]     <- 写到一半崩溃
```

恢复规则：

1. 若最后一卷没有完整 checkpoint，则回退到前一个完整 checkpoint
2. 最后一卷未完成部分视为无效尾部
3. 卷级校验可作为附加安全措施，但不应替代 checkpoint 语义

---

## 12. 压缩设计

### 12.1 codec 策略

建议 codec 定位：

1. `none`
   - 已压缩文件
   - 不可压缩文件
2. `zstd`
   - 默认平衡模式
3. `lz4`
   - 速度优先模式
4. `lzma`
   - 空间优先模式

### 12.2 codec 编码

建议使用 `u8`：

```text
0x00 = none
0x01 = zstd
0x02 = lz4
0x03 = lzma
0x04-0xFF = reserved
```

### 12.3 默认算法

默认：

1. `zstd`

默认等级建议：

1. `zstd level 1`
2. 若未来验证表明 CPU 仍是主要瓶颈，可进一步评估 `zstd` 负级别快速模式

### 12.4 压缩粒度

压缩粒度不是整个容器，而是单个 blob：

1. 文件变化
2. 选择 codec
3. 压缩该文件内容
4. append 到容器

### 12.5 不压缩规则

默认跳过：

1. `.zip`
2. `.7z`
3. `.rar`
4. `.gz`
5. `.xz`
6. `.zst`
7. `.lz4`
8. `.bz2`
9. `.br`
10. `.jpg`
11. `.jpeg`
12. `.png`
13. `.webp`
14. `.mp4`
15. `.mkv`

建议把上述列表作为**内置默认值**，并允许通过配置项覆盖：

1. `skip_compress_extensions`
2. `compression_min_size`
3. `compression_max_size`

收益阈值建议：

1. 若 `stored_size >= raw_size * 0.95`
2. 回退 `codec = none`

### 12.6 auto 模式

建议 CLI：

```text
--compression none
--compression zstd
--compression zstd:9
--compression lz4
--compression lzma
--compression auto
```

`auto` 语义建议：

1. 先用 `lz4` 对文件前若干 KB 样本做快速可压缩性探测
2. 若样本收益明显不足，则直接回退 `none`
3. 若样本显示可压缩，再用主算法（默认 `zstd`）压缩完整文件
4. 样本大小和阈值属于实现参数，但必须保持轻量，不能退化成“整文件试压后再丢弃”

### 12.7 关于 `lz4`

如果优先追求恢复速度：

1. `lz4` 是最值得优先加入的第二阶段 codec
2. 它对"恢复快"帮助比 `lzma` 更直接

### 12.8 关于 `lzma`

`lzma` 只适合：

1. 长期归档
2. 空间优先

不应成为默认策略。

### 12.9 关于 zstd seekable

当前设计不要求直接采用 zstd seekable format。

但需要保留兼容性：

1. 当前设计本质上已经接近 frame-per-blob
2. 若未来引入多文件聚合 blob，应优先考虑 seekable zstd 或等价可定位 frame 布局

---

## 13. 写入流程

### 13.1 首次创建

首次创建流程：

1. 创建新文件
2. 写入 Header（64 字节）
3. baseline 为空
4. 进入常规写入流程

### 13.2 更新现有容器

更新流程：

1. 打开 `.xunbak`
2. 通过 footer 定位最新 checkpoint
3. 读取最新 manifest
4. 基于最新 manifest 一次性构建 `content_hash -> blob 定位信息` 的内存索引
5. 扫描当前源目录
6. 以 manifest 为 baseline 做文件级 diff
7. 对新增/变更文件：
   - 计算 `content_hash`
   - 查步骤 4 构建的 `content_hash` 内存索引
   - 若已有：复用旧 blob
   - 若没有：压缩并 append 新 blob
   - 若单个文件超过单卷硬限制：按 `parts[]` 规则写入多个 blob record
8. 对未变化文件：
   - 复用旧 blob 引用
9. 生成新 manifest
10. append 新 checkpoint
11. 更新 footer

### 13.3 持久化顺序

建议两阶段 flush：

1. append 所有新 blob
2. append 新 manifest
3. `FlushFileBuffers` / `fsync`
   - 保证数据先落盘
4. append 新 checkpoint
5. 覆写 footer
6. `FlushFileBuffers` / `fsync`
   - 保证入口指针落盘

这样即使在第二次 flush 前崩溃：

1. 旧 checkpoint 仍有效
2. 新 blob / manifest 只是未引用尾部

### 13.4 去重规则

必须明确：

1. 普通单 blob 文件：`blob_id = content_hash`
2. multipart 文件：每个 part 的 `blob_id = blake3(part_content)`，而 entry 的 `content_hash = blake3(whole_file)`
3. 同内容、不同路径只存一份 blob / part
4. rename 但内容不变时不写新 blob

### 13.5 并行压缩

建议采用"读取 -> hash + compress 流水线 -> 顺序写入"模式：

1. 读取阶段负责顺序读取源文件内容
2. 计算阶段使用流式 `blake3` hasher 边读边更新
3. 压缩阶段把同一批数据块持续送入 codec encoder
4. 多个文件可并行执行上述流水线
5. 压缩结果通过 channel 或 buffer 汇聚
6. 单线程按序 append 到容器文件

这样压缩吞吐不受 I/O 串行化限制，写入仍然保证 append-only 顺序。

multipart 文件的 `content_hash` 计算建议：

1. 普通单 blob 文件可采用单遍流式 `blake3 + compress`
2. multipart 文件优先采用**两阶段**策略：
   - 第一遍：流式读取整个文件，计算 `content_hash = blake3(whole_file)`
   - 第二遍：按 `parts[]` 边界切分、为每个 part 计算 `part blob_id` 并压缩写入
3. 若未来需要进一步优化，再评估单遍同时维护 whole-file hasher 与 part-level hasher/encoder 的实现

### 13.6 空容器与最小合法大小

以下状态是合法的：

1. 源目录为空
2. 所有文件都被 exclude 规则过滤

此时容器仍应包含：

1. Header
2. 一个空 manifest record（`entries = []`, `file_count = 0`, `total_raw_bytes = 0`）
3. 一个 checkpoint record
4. 一个 footer

设计要求：

1. verify 应通过
2. restore 应产出空目录
3. compact 对该容器是 no-op

实现提示：

1. 单文件容器的最小合法大小约为 `Header(64) + Manifest Record + Checkpoint Record + Footer(24)`
2. quick verify 可用一个保守最小值做早期拒绝；小于该值的文件直接判定为“不完整容器”

---

## 14. 恢复流程

恢复流程：

1. 打开 `.xunbak`
2. 通过 footer 定位最新 checkpoint
3. 读取 manifest
4. 选择恢复模式：
   - all
   - file
   - glob
5. 恢复前先生成读取计划
   - 全量恢复时，优先按 `blob_offset` 升序安排读取任务，尽量接近顺序 I/O
   - 定向恢复时，优先保证目标选择正确，再做局部排序
6. 按 entry 读取 blob
   - 单 blob entry：直接读取
   - multipart entry：按 `parts[]` 顺序读取并拼接
7. 按 codec 解压
8. 写回目标目录
9. 恢复文件属性 / 时间

### 14.1 并行恢复

由于所有 blob 位于同一容器文件中：

1. 每个线程必须使用独立 file handle
2. 每个线程按 `offset + length` 读取自己的 blob
3. 不允许多个线程共享同一个 seek cursor

### 14.2 文件属性恢复

若 manifest 中记录：

1. `win_attributes`
2. `created_time_ns`

恢复后应尝试：

1. 还原 Windows 文件属性
2. 还原创建时间

### 14.3 历史快照恢复（格式预留，当前不实现）

格式层已预留 `prev_checkpoint_offset`。

未来按需启用后可支持：

1. `snapshot restore --id <snapshot_id>`
2. 按"最近第 N 个快照"恢复

当前 Phase 1 只支持 latest snapshot 恢复。

---

## 15. 完整性与 verify

完整设计至少包含：

1. checkpoint 完整性
2. manifest 完整性（通过 `manifest_hash`）
3. blob 内容完整性
4. record 结构完整性（通过 `record_crc`）

### 15.1 verify 分级

建议：

```text
xun verify project.xunbak
xun verify project.xunbak --level quick
xun verify project.xunbak --level full
xun verify project.xunbak --level paranoid
```

语义建议：

1. `quick`
   - footer 合法性
   - checkpoint CRC
   - manifest 可解析
   - manifest blake3 == checkpoint 中的 `manifest_hash`
2. `full`
   - quick
   - 所有 blob 解压 + blake3 校验
3. `paranoid`
   - full
   - 全容器顺序扫描，逐条验证 `record_crc` + record 边界连续性
   - 对 blob record 来说，`record_crc` 只验证固定头部；data payload 仍由 `full` 级别的内容哈希校验负责

输出建议：

1. verify 默认输出结构化摘要：footer、checkpoint、manifest、blob、耗时
2. 失败时必须输出首个错误的 `path / blob_id / offset / volume_index`

### 15.2 checkpoint 校验

1. 校验 `checkpoint_crc`
2. 校验 `manifest_offset/len` 在文件范围内
3. 校验 `container_end`

### 15.3 manifest 校验

1. 读取 manifest record 的 **payload**（含 4 字节 manifest 固定前缀，不含 13 字节 record 前缀）
2. 计算 `blake3`
3. 与 checkpoint 中的 `manifest_hash` 对比
4. 不一致则 manifest 不可信

这解决了 manifest 元数据发生可解析比特翻转但 verify 仍通过的问题。

### 15.4 blob 校验

恢复或 verify 时：

1. 单 blob 文件：读 blob -> 解压 -> 计算 `blake3`
2. multipart 文件：按 `parts[]` 顺序读取并解压各 part，再对拼接后的完整文件计算 `blake3`
3. 与 entry 中的 `content_hash` 对比

### 15.5 文件对象语义

`.xunbak` 当前只记录**普通文件**：

1. 不保留空目录
2. 不保留符号链接
3. 不保留硬链接关系

这与当前 XunYu backup 扫描行为一致。如果未来需要保留空目录或链接对象，应在 manifest entry 中增加 `entry_type` 字段。

---

## 16. 崩溃恢复

append-only 的恢复原则必须简单且稳定：

1. 只认最后一个完整 checkpoint
2. checkpoint 之后的残留数据视为尾部垃圾
3. footer 损坏时尝试顺序扫描（依赖 `record_crc` 逐条校验，见 §10.2）
4. 顺序扫描是尽力恢复机制，不保证在任意损坏场景下恢复

要求：

1. 不依赖外部 journal
2. 单个 `.xunbak` 文件本身必须足以恢复到最近一个稳定状态

---

## 17. compact 与膨胀控制

append-only 的代价是容器膨胀，因此完整设计必须包含 compact。

### 17.1 compact 流程（latest-only）

默认 compact 只保留最新快照：

1. 读取最新 checkpoint
2. 读取最新 manifest
3. 收集该 manifest 引用的 `blob_id` 集合（包含 `parts[]` 中各 part 的 `blob_id`）
4. 只重写这些 blob
5. 重写该 manifest
6. 写入新 checkpoint
7. 写入新 footer
8. 原子替换旧容器

compact 后 `snapshot_id` 必须保持不变。

### 17.2 Windows 替换策略

在 Windows 上不应只依赖普通 rename。

建议：

1. 写 `project.xunbak.tmp`
2. 视需要保留 `project.xunbak.bak`
3. 使用 `ReplaceFileW` 或 `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`
4. 若遇到临时锁定（如杀毒/索引器占用），按短退避重试 3 次
5. 失败时输出明确的 Win32 error code
6. 替换成功后清理 `.bak`

### 17.3 compact 触发指标

建议：

```text
waste_ratio = 1 - (referenced_blob_bytes / total_container_bytes)
```

用途：

1. 提示用户执行 compact
2. 未来自动 compact 策略

### 17.4 多快照保留（格式预留，当前不实现）

如果未来启用 `prev_checkpoint_offset` 实现多版本快照，compact 必须调整为：

1. 沿 checkpoint chain 收集所有保留快照的 manifest
2. 收集这些 manifest 引用的 `blob_id` 联合集
3. 只重写联合集中的 blob
4. 重建 checkpoint chain

当前不实现此逻辑；默认 compact 只看最新 manifest。

---

## 18. 与现有 backup/restore 的关系

`.xunbak` 不是当前目录/zip 备份的小补丁，而是新的主容器路线。

建议关系：

1. 当前目录/zip 备份继续保留为兼容模式
2. `.xunbak` 作为新一代单文件容器模式
3. 长期由 `.xunbak` 成为主路径

当前命令形态建议：

```text
xun backup --container project.xunbak
xun restore project.xunbak
xun verify project.xunbak
xun compact project.xunbak
```

实现建议：

1. `.xunbak` 模式优先复用现有 `scan.rs` 与 `diff.rs` 的扫描、过滤和差异检测逻辑
2. 目录模式与 `.xunbak` 模式应共享 scan/diff 阶段，只在写入目标和恢复目标层分叉
3. 长任务应输出实时进度：已处理字节、吞吐、文件数、ETA

格式可演进命令（当前不实现）：

```text
xun snapshot list project.xunbak
xun snapshot restore project.xunbak --id <snapshot_id>
xun snapshot prune project.xunbak --keep-last 5
```

如果强调低风险演进：

1. 先作为 `backup/restore` 的一种新模式接入
2. 成熟后再决定是否上升为默认路径

---

## 19. 锁与并发模型

`.xunbak` / `.xunbak.*` 是可变容器，必须定义明确的并发边界。

锁模型建议：

1. 同一容器集（单文件或同一 `set_id` 分卷集）同一时刻只允许一个**写者**
2. `backup/update/compact/upgrade/repair` 必须获取**独占写锁**
3. `restore/list/quick verify` 获取**共享读锁**
4. `full/paranoid verify` 默认获取共享读锁；若检测到写锁存在，应拒绝运行或显式 `--force-readonly`
5. 分卷模式下锁作用域是整个卷集，不是单卷

锁载体建议：

1. 使用容器旁路锁文件，如 `project.xunbak.lock`
2. 分卷模式仍使用一把逻辑锁，避免多卷分别加锁导致状态分裂
3. 锁文件记录：`pid / hostname / username / command / started_at / heartbeat_at / tool_version / write_start_offset`
4. 锁文件只用于并发控制，不参与崩溃恢复语义

陈旧锁处理建议：

1. 进程正常退出时主动释放锁
2. 长任务定期刷新 `heartbeat_at`
3. 若 heartbeat 超过阈值未更新，标记为 stale
4. stale lock 不自动删除，只允许显式 `force-unlock`
5. `force-unlock` 前必须提示用户确认当前无存活写者

`write_start_offset` 的用途：

1. 写入前记录本次追加的起始位置
2. 若进程异常退出且未形成新 checkpoint，可辅助判断尾部垃圾范围
3. 恢复工具可在只读检查后，选择性将最后一卷或单文件 truncate 回 `write_start_offset`

---

## 20. 存储与文件系统假设

当前设计强依赖本地文件系统语义，不是对任意底层存储都等价成立。

推荐支持层级：

1. **优先支持**：本地 `NTFS / ReFS`
2. **谨慎支持**：同局域网内语义稳定的 `SMB / NAS`
3. **不建议作为主仓库**：云同步目录、对象存储挂载盘、语义不明的第三方虚拟文件系统

容器正确性依赖以下底层能力：

1. 支持随机读写与 `seek`
2. 支持 `truncate`
3. 支持 `FlushFileBuffers` / `fsync`
4. 支持原子替换或接近原子的替换语义
5. 支持稳定的目录枚举与文件存在性检查
6. 支持进程级排他锁或等价锁文件语义

额外约束：

1. FAT32 仅建议用于分卷模式，且每卷必须严格小于 `4 GiB`
2. 若底层不保证 `flush + truncate + rename` 的可预期语义，应拒绝 `compact / upgrade / repair`
3. 若底层目录枚举存在明显延迟，不应把该路径作为分卷集的唯一存放位置

---

## 21. 安全模型

当前设计默认面向**受信任的本地备份位置**，重点解决的是完整性与可恢复性，不是零信任仓库问题。

当前阶段明确：

1. 默认**不提供加密**
2. 默认**不提供抗恶意篡改认证**（当前 `record_crc / manifest_hash / checkpoint_crc` 主要防误损坏，不防主动攻击）
3. 默认假设执行环境对容器文件拥有本地文件系统级控制

因此：

1. `.xunbak` 适合作为本地/私有 NAS 备份容器
2. 不应把它直接等同于“可存放到不可信远端的安全归档格式”
3. 若用于重要数据，仍建议搭配只读副本、异地副本或不可变存储策略

格式演进预留建议：

1. 若未来引入加密，应优先加密 `blob / manifest / checkpoint payload`
2. `Header / Footer` 可保留最小明文以支持容器定位
3. 若未来引入认证，应使用 MAC 或签名覆盖逻辑快照元数据，而不是仅依赖 CRC

---

## 22. 元数据支持矩阵

当前设计必须明确“保留什么，不保留什么”，避免恢复语义被误解。

当前保留：

1. 相对路径
2. 文件内容
3. `size`
4. `mtime_ns`
5. `created_time_ns`
6. `win_attributes`
7. `codec`
8. blob / volume 定位信息

当前不保留：

1. ACL / owner / SID / 完整安全描述符
2. ADS（Alternate Data Streams）
3. 符号链接 / junction / reparse point
4. 硬链接拓扑
5. 稀疏文件布局
6. 文件系统级压缩/加密状态
7. `atime`
8. EA / xattrs

设计要求：

1. 文档、CLI 帮助和恢复提示都应把这些限制讲清楚
2. 若未来要补齐 ACL / ADS / links，必须先扩展 manifest entry 语义，而不是在恢复阶段临时猜测
3. 当前 `.xunbak` 与现有 `scan.rs` 的普通文件语义保持一致，不额外承诺目录对象和特殊对象恢复

---

## 23. 本地缓存与变更检测策略

要满足“速度快、性能高”，不能把每次备份都退化成全量哈希。

变更检测建议分两层：

1. **快路径预筛**：默认以 `size + mtime` 作为变更预判，与当前 `backup` 行为保持一致
2. **慢路径确认**：仅对新增/变更/可疑文件计算完整 `blake3`

baseline 来源：

1. 权威 baseline 来自最新 manifest
2. 本地缓存只是性能优化，不得成为正确性的唯一依据

本地缓存建议：

1. 使用容器外部缓存文件或本机缓存目录，如 `%LOCALAPPDATA%/xun/xunbak-cache/`
2. 以 `container path / set_id / source_root` 作为缓存命名空间
3. 记录 `path / size / mtime_ns / created_time_ns / optional file_id / last_content_hash`
4. 命中缓存且指纹未变时，可跳过重复哈希
5. 缓存丢失、损坏或版本不兼容时，应直接回退到 manifest + 文件系统扫描，不影响正确性

补充建议：

1. Windows 本地文件系统可通过 `GetFileInformationByHandle` 读取 `VolumeSerialNumber + FileIndex` 作为 `file_id`
2. 当 `size + mtime` 匹配但路径发生变化时，可用 `file_id` 辅助识别 rename
3. 网络文件系统默认不信任 `file_id`，优先使用 `size + mtime`
4. 不应因缓存命中而跳过 verify；缓存只优化 backup，不参与恢复可信性判断

---

## 24. 修复、回滚与运维建议

完整设计不应只定义“如何写”，还要定义“坏了以后怎么办”。

基本原则：

1. 默认不做自动破坏性 repair
2. verify 与 recover 应优先走**只读路径**
3. repair 必须显式触发，并在执行前提示风险

建议运维流程：

1. 日常更新后默认做 `quick verify`
2. 周期性执行 `full verify`
3. 在容器迁移、升级、长期归档前执行 `paranoid verify`
4. 周期性执行一次测试恢复（test restore），不要只依赖校验哈希
5. `compact` 默认支持 `--dry-run`，先展示可回收空间、预计收益与预计耗时

故障处理建议：

1. footer 损坏：先尝试 fallback；成功则进入只读模式并提示用户尽快 compact/rewrite
2. manifest_hash 不匹配：该 checkpoint 不可信；若未来启用 checkpoint chain，可回退到前一 checkpoint
3. compact 中断：旧容器必须保留到新容器完成原子替换之后
4. stale lock：仅允许人工确认后 `force-unlock`
5. 若 lockfile 中存在 `write_start_offset` 且确认没有形成新 checkpoint，可选择性回退到该偏移

---

## 25. 格式升级与兼容策略

当前文档虽然不是冻结格式规范，但仍需有清晰的升级语义。

版本字段语义：

1. `write_version` 表示写入端使用的格式版本
2. `min_reader_version` 表示可安全读取该容器的最低 reader 版本
3. reader 若低于 `min_reader_version`，必须直接拒绝读取

升级原则：

1. 不做大规模原地格式迁移
2. 升级通过“读取旧容器 -> 写新容器 -> verify -> 原子替换”完成
3. `compact` 可以作为格式重写载体，但不得静默改变恢复语义
4. 降级不保证支持；若容器已写入新字段或新 codec，旧版 reader 应明确报错

兼容策略：

1. 新 reader 应尽量保持对旧容器只读兼容
2. 未知 flag / codec / manifest_type 不能 panic，必须返回明确错误
3. manifest codec 的切换应通过 `manifest_codec` 显式声明，而不是靠版本猜测

---

## 26. 快照上下文元数据

为了审计、排障和长期维护，快照除了文件视图，还应记录执行上下文。

建议随 manifest 或 checkpoint 记录：

1. `hostname`
2. `username`
3. `os / arch`
4. `xunyu_version`
5. `command_mode`（如 backup / compact / upgrade）
6. `compression_profile`
7. `include/exclude` 规则摘要或摘要哈希
8. 用户描述、标签、备注

作用：

1. 帮助判断某个快照是在哪台机器、以什么参数生成的
2. 便于排查“为什么这次文件数/体积变化异常”
3. 为未来 dashboard / snapshot list 提供可展示元数据

---

## 27. 分阶段落地

### Phase 1

1. 定义 `.xunbak` header / blob / manifest / checkpoint / footer
2. 实现单文件 latest snapshot 写入
3. 实现 latest snapshot 恢复
4. 支持 `none / zstd`
5. 实现 `content_hash` 复用旧 blob
6. 实现两阶段 flush 写入
7. 实现流式 `blake3 + compress` 流水线与并行压缩
8. 实现基础 verify + 独占写锁
9. 写入基础快照上下文元数据
10. 明确当前元数据支持矩阵与非目标项
11. 复用现有 `scan.rs / diff.rs` 基础设施
12. 固定 Header magic，并由版本字段负责兼容路由
13. 为 Footer 增加 CRC32C
14. 预留 `FLAG_SPLIT`
15. 预留 `FLAG_ALIGNED`
16. 预留 `manifest_type`
17. 预留 `prev_checkpoint_offset`（写 0）

### Phase 2

1. 实现分卷写入/读取
2. 实现 `lz4`
3. 实现 `auto`
4. 实现 compact（latest-only）
5. 实现共享读锁、stale lock 检测与 `force-unlock`
6. 加入本地缓存与更快的变更检测
7. 实现 `compact --dry-run`、进度与结构化 verify 报告
8. 加入更多诊断和完整性工具
9. 评估 MessagePack manifest

### Phase 3

1. 按需评估是否启用 checkpoint chain 与多版本快照
2. 按需评估 delta manifest
3. 评估小文件聚合
4. 评估 `lzma/xz`
5. 评估 zstd dictionary
6. 评估 ACL / ADS / link / 特殊文件元数据扩展
7. 评估加密、认证与不可变存储集成
8. 再决定是否进入 chunk/pack 路线

---

## 28. 结论

基于六轮审核意见，完整设计应坚持：

1. 默认单文件
2. 必要时分卷
3. append-only
4. **默认 only latest，不做 git**
5. footer 快速定位
6. 格式层预留 checkpoint chain
7. 文件级增量
8. blob 级 codec
9. 可选压缩算法
10. 流式 hash+压缩流水线 + 顺序写入
11. 分级 verify
12. compact 默认 latest-only

一句话结论：

> 对 XunYu 来说，`.xunbak` 的正确方向不是"继续改 zip"，也不是"做 git 式历史系统"，而是"单文件容器 + append-only + 默认 only latest + blob 级 codec + 可选分卷"的高性能备份格式。

---

## 29. 设计依据

本方案吸收并参考了：

1. restic 的 pack / snapshot 思路
2. kopia 的 content 定位与 pack 组织
3. borg 的压缩、校验与 auto 模式
4. Delta Lake 的增量 manifest 思路
5. SQLite 的格式版本和两阶段写入思路
6. RAR 5 的分卷 volume 标记与 total_volumes 回填机制
7. Windows 平台上的 `FlushFileBuffers`、`ReplaceFileW`、`MoveFileExW`
8. restic 的仓库锁、change detection 与 check 思路
9. kopia 的 maintenance、consistency、caching 与 compatibility 思路
10. borg 的 files cache、append-only 与元数据建模思路
11. borg 的 auto compression 探测思路
12. Kopia 的按扩展名/大小配置压缩策略思路
13. zstd 官方基准中的默认级别权衡
14. SQLite 的固定 magic + 版本字段路由思路
15. BLAKE3 的流式与并行哈希设计
16. Windows `CreateFileW` 的 unbuffered I/O 能力与 `ReplaceFileW` 的替换/错误语义
17. ULID 的可排序 128 位标识设计
18. Windows `FILETIME` 的 100ns 精度与 `GetFileAttributesW` / `GetFileInformationByHandle` 元数据语义

同时保留了 XunYu 的现实约束：

1. Windows-first
2. 单文件优先
3. 默认 only latest，不做 git
4. 工程可实现性优先
5. 先保证速度和稳定，再追求极限压缩率
