# .xunbak 单文件容器 — TDD 分阶段任务清单

> 依据：[Single-File-Xunbak-Design.md](./Single-File-Xunbak-Design.md)
> 标记：`[ ]` 待办　`[-]` 进行中　`[x]` 完成
> 原则：**红-绿-重构**。每个任务先写失败测试，再写最小实现，最后重构。

---

## Phase 0：模块骨架与二进制常量

### 0.1 模块结构

- [ ] 新建 `src/xunbak/mod.rs`，声明子模块（`header`, `record`, `blob`, `manifest`, `checkpoint`, `footer`, `codec`, `writer`, `reader`, `verify`, `lock`）
- [ ] `src/commands/mod.rs`：注册 xunbak 相关命令路由
- [ ] `src/cli/backup.rs`：新增 `--container <path>` 参数（`.xunbak` 模式入口）
- [ ] `Cargo.toml`：feature gate `xunbak`，引入依赖（`crc32c`, `blake3`, `zstd`, `ulid`）
- [ ] 新建 `tests/test_xunbak.rs` 测试入口 + `tests/xunbak/` 子目录

### 0.2 常量与枚举

- [ ] **测试**：断言 `HEADER_MAGIC == b"XUNBAK\0\0"`，`FOOTER_MAGIC == b"XBKFTR\0\0"`
- [ ] **测试**：断言 `HEADER_SIZE == 64`，`FOOTER_SIZE == 24`，`RECORD_PREFIX_SIZE == 13`
- [ ] **测试**：`RecordType` 枚举 `u8` 编码值正确（`Blob=0x01, Manifest=0x02, Checkpoint=0x03`）
- [ ] **测试**：`Codec` 枚举 `u8` 编码值正确（`None=0x00, Zstd=0x01, Lz4=0x02, Lzma=0x03`）
- [ ] 实现 `src/xunbak/constants.rs`：所有 magic / size / 枚举定义
- [ ] **测试**：未知 `RecordType(0xFF)` 可安全构造且 `is_known()` 返回 false
- [ ] **测试**：未知 `Codec(0x80)` 可安全构造且 `is_known()` 返回 false

---

## Phase 1：Header 读写（§6）

### 1.1 Header 序列化

- [ ] **测试**：构造 `Header`，序列化为 `[u8; 64]`，验证各字段偏移与字节值
- [ ] **测试**：`write_version`, `min_reader_version` 在正确偏移（8, 12）
- [ ] **测试**：`flags` 在偏移 16，8 字节 LE
- [ ] **测试**：`created_at_unix` 在偏移 24
- [ ] **测试**：非分卷模式 reserved 区域全零
- [ ] 实现 `src/xunbak/header.rs`：`Header` 结构体 + `to_bytes()` + `from_bytes()`

### 1.2 Header 反序列化与校验

- [ ] **测试**：从合法 64 字节 buf 反序列化，字段完全匹配
- [ ] **测试**：magic 错误 → 返回 `Err(InvalidMagic)`
- [ ] **测试**：buf 长度 < 64 → 返回 `Err(HeaderTooShort)`
- [ ] **测试**：`min_reader_version` 高于当前版本 → 返回 `Err(VersionTooNew)`
- [ ] **测试**：未知 flags 不 panic，返回带警告的 `Ok`

### 1.3 分卷 Header 字段（§11.4，格式预留）

- [ ] **测试**：`FLAG_SPLIT` 启用时 `volume_index / split_size / set_id` 从 reserved 正确读写
- [ ] **测试**：`FLAG_SPLIT` 未启用时 reserved 区域全零
- [ ] 实现分卷字段的条件序列化/反序列化

---

## Phase 2：Record 前缀编解码（§5）

### 2.1 Record 前缀

- [ ] **测试**：写入 `RecordPrefix { record_type, record_len, record_crc }`，输出恰好 13 字节
- [ ] **测试**：字节序为 LE；`record_type` 在偏移 0，`record_len` 在偏移 1-8，`record_crc` 在偏移 9-12
- [ ] **测试**：从 13 字节 buf 反序列化，字段完全匹配
- [ ] 实现 `src/xunbak/record.rs`：`RecordPrefix` 结构体 + 序列化/反序列化

### 2.2 CRC 计算

- [ ] **测试**：blob record CRC 覆盖范围 = `record_type(1) + record_len(8) + blob 固定头部(50)`，不含 CRC 自身 4 字节，不含 data payload
- [ ] **测试**：manifest record CRC 覆盖范围 = `record_type(1) + record_len(8) + 全 payload`，不含 CRC 自身
- [ ] **测试**：checkpoint record CRC 覆盖同 manifest（全 payload）
- [ ] **测试**：空 payload 的 CRC 计算不 panic
- [ ] 实现 `compute_record_crc(record_type, record_len_bytes, payload_for_crc) -> u32`

### 2.3 顺序扫描辅助

- [ ] **测试**：给定一段包含 3 条连续 record 的字节流，`scan_records()` 返回 3 个 `(offset, type, len)`
- [ ] **测试**：第 2 条 record CRC 损坏 → 扫描在第 2 条终止，返回前 1 条
- [ ] **测试**：`record_len` 导致越界 → 返回 `Err(TruncatedRecord)`
- [ ] 实现 `scan_records(reader) -> Vec<ScannedRecord>` 或迭代器

---

## Phase 3：Footer 读写（§10）

### 3.1 Footer 序列化

- [ ] **测试**：构造 `Footer { checkpoint_offset }`，序列化为 `[u8; 24]`
- [ ] **测试**：`footer_magic` 在偏移 0-7，`checkpoint_offset` 在偏移 8-15，`footer_crc32c` 在偏移 16-19，padding 在偏移 20-23
- [ ] **测试**：`footer_crc32c` 覆盖 `magic(8) + checkpoint_offset(8)`
- [ ] 实现 `src/xunbak/footer.rs`：`Footer` + `to_bytes()` + `from_bytes()`

### 3.2 Footer 反序列化与校验

- [ ] **测试**：合法 24 字节 → 反序列化成功
- [ ] **测试**：magic 错误 → `Err(InvalidFooterMagic)`
- [ ] **测试**：CRC 不匹配 → `Err(FooterCrcMismatch)`
- [ ] **测试**：`checkpoint_offset` 超出文件大小 → `Err(OffsetOutOfRange)`（需文件大小参数）

---

## Phase 4：Blob Record 读写（§7）

### 4.1 Blob 固定头部

- [ ] **测试**：构造 blob 固定头部（50 字节），序列化后各字段偏移正确
- [ ] **测试**：`blob_id` 偏移 0-31，`blob_flags` 偏移 32，`codec` 偏移 33，`raw_size` 偏移 34-41，`stored_size` 偏移 42-49
- [ ] **测试**：反序列化后字段完全匹配
- [ ] 实现 `src/xunbak/blob.rs`：`BlobHeader` + 序列化/反序列化

### 4.2 Blob Record 完整写入

- [ ] **测试**：写入一个小文件（如 "hello world"），`record_type=0x01`，`record_len = 50 + stored_size`
- [ ] **测试**：`blob_id` == `blake3("hello world")`
- [ ] **测试**：`codec=None` 时 `stored_size == raw_size`，data payload == 原始内容
- [ ] **测试**：`codec=Zstd` 时 `stored_size <= raw_size`，解压后 == 原始内容
- [ ] **测试**：`record_crc` 仅覆盖 `record_type + record_len + blob 固定头部`，篡改 data payload 不影响 `record_crc`
- [ ] 实现 `write_blob_record(writer, content, codec) -> BlobWriteResult`

### 4.3 Blob Record 读取

- [ ] **测试**：从合法字节流读取 blob record，解压后内容 == 原始
- [ ] **测试**：`record_crc` 校验失败 → `Err(BlobCrcMismatch)`
- [ ] **测试**：`blob_id` 校验失败（内容被篡改） → `Err(BlobHashMismatch)`
- [ ] **测试**：未知 `codec` → `Err(UnsupportedCodec)`
- [ ] 实现 `read_blob_record(reader) -> BlobReadResult`

---

## Phase 5：压缩层（§12）

### 5.1 Codec 抽象

- [ ] **测试**：`compress(Codec::None, data)` → 返回原始数据
- [ ] **测试**：`decompress(Codec::None, data)` → 返回原始数据
- [ ] **测试**：`compress(Codec::Zstd, data)` → 返回有效 zstd frame
- [ ] **测试**：`decompress(Codec::Zstd, compressed)` → 返回原始数据
- [ ] **测试**：空数据 compress/decompress 正确往返
- [ ] **测试**：1 MB 随机数据往返正确
- [ ] 实现 `src/xunbak/codec.rs`：`compress(codec, data, level) -> Vec<u8>` + `decompress(codec, data) -> Vec<u8>`

### 5.2 不压缩规则

- [ ] **测试**：`.jpg` / `.zip` / `.mp4` 等扩展名 → `should_skip_compress()` 返回 true
- [ ] **测试**：`.rs` / `.txt` / `.json` → 返回 false
- [ ] **测试**：收益阈值：`stored_size >= raw_size * 0.95` → 回退 `codec=None`
- [ ] 实现 `should_skip_compress(ext) -> bool` + 压缩收益检查

### 5.3 Zstd 等级

- [ ] **测试**：默认 zstd level 1 输出合法且可解压
- [ ] **测试**：`--compression zstd:9` 解析为 level 9
- [ ] 实现压缩参数解析（`none / zstd / zstd:N / lz4 / lzma / auto`）

---

## Phase 6：Manifest 读写（§8）

### 6.1 Manifest 固定前缀

- [ ] **测试**：manifest 固定前缀 4 字节：`manifest_codec(1) + manifest_type(1) + manifest_version(2 LE)`
- [ ] **测试**：从 4 字节 buf 反序列化，字段匹配
- [ ] 实现 `ManifestPrefix` 结构体

### 6.2 Manifest Body — JSON 序列化

- [ ] **测试**：构造完整 `ManifestBody`（含 `snapshot_id`, `entries`, `file_count` 等），JSON 序列化 → 反序列化往返正确
- [ ] **测试**：entry 包含所有必需字段（`path, blob_id, content_hash, size, mtime_ns, created_time_ns, win_attributes, codec, blob_offset, blob_len, volume_index`）
- [ ] **测试**：`ext` 字段为 `None` 时 JSON 中省略（`#[serde(skip_serializing_if)]`）
- [ ] **测试**：`parts` 字段为 `None` 时 JSON 中省略
- [ ] **测试**：`removed` 为空数组时序列化为 `[]`
- [ ] **测试**：entry 中 `blob_id` 以 hex 编码，32 字节 → 64 hex chars
- [ ] **测试**：`snapshot_id` 以 ULID 格式序列化（26 字符 Crockford Base32）
- [ ] 实现 `src/xunbak/manifest.rs`：`ManifestBody`, `ManifestEntry`, `ManifestPart` + serde

### 6.3 Manifest Record 写入

- [ ] **测试**：写入 manifest record：`record_type=0x02`，payload = `prefix(4) + json_body`
- [ ] **测试**：`record_len` == `4 + json_body.len()`
- [ ] **测试**：`record_crc` 覆盖 `record_type + record_len + 全 payload`
- [ ] 实现 `write_manifest_record(writer, manifest) -> ManifestWriteResult`

### 6.4 Manifest Record 读取

- [ ] **测试**：从合法字节流读取 manifest record，解析出 `ManifestPrefix` + `ManifestBody`
- [ ] **测试**：`manifest_codec` 不是 JSON → `Err(UnsupportedManifestCodec)`
- [ ] **测试**：JSON 解析失败 → `Err(ManifestParseError)`
- [ ] **测试**：`record_crc` 不匹配 → `Err(ManifestCrcMismatch)`
- [ ] 实现 `read_manifest_record(reader) -> ManifestReadResult`

### 6.5 路径规范化（§8.3）

- [ ] **测试**：`C:\Users\foo\bar` → 相对路径 `foo/bar`（分隔符转换）
- [ ] **测试**：大小写不敏感去重：`Foo/Bar.txt` 与 `foo/bar.txt` → 冲突检测报错
- [ ] **测试**：空路径 → `Err(EmptyPath)`
- [ ] 实现 `normalize_path()` + `detect_case_conflicts()`

### 6.6 时间戳转换（§8.4）

- [ ] **测试**：Windows FILETIME → Unix epoch nanoseconds 往返正确
- [ ] **测试**：已知时间点精确值对照（如 `2026-01-01 00:00:00 UTC`）
- [ ] **测试**：边界值：`FILETIME = 0`（1601-01-01）→ 负 Unix ns
- [ ] 实现 `filetime_to_unix_ns()` + `unix_ns_to_filetime()`

---

## Phase 7：Checkpoint 读写（§9）

### 7.1 Checkpoint Payload

- [ ] **测试**：构造 `CheckpointPayload`（128 字节），序列化后各字段偏移正确
- [ ] **测试**：`snapshot_id` 偏移 0-15（16 字节 ULID），`manifest_offset` 偏移 16-23，`manifest_len` 偏移 24-31
- [ ] **测试**：`manifest_hash` 偏移 32-63（32 字节 blake3）
- [ ] **测试**：`container_end` 偏移 64-71，`blob_count` 偏移 72-79
- [ ] **测试**：`referenced_blob_bytes` 偏移 80-87，`total_container_bytes` 偏移 88-95
- [ ] **测试**：`prev_checkpoint_offset` 偏移 96-103（当前始终为 0）
- [ ] **测试**：`total_volumes` 偏移 104-105（u16 LE，默认 1）
- [ ] **测试**：reserved 偏移 106-123（18 字节全零）
- [ ] **测试**：`checkpoint_crc` 偏移 124-127，覆盖 payload 前 124 字节（不含自身）
- [ ] 实现 `src/xunbak/checkpoint.rs`：`CheckpointPayload` + `to_bytes()` + `from_bytes()`

### 7.2 Checkpoint Record

- [ ] **测试**：完整 checkpoint record = 13（前缀）+ 128（payload）= 141 字节
- [ ] **测试**：`record_type = 0x03`
- [ ] **测试**：`record_crc` 覆盖 `record_type + record_len + 全 payload`
- [ ] **测试**：`checkpoint_crc` 与 `record_crc` 独立校验，互不替代
- [ ] **测试**：`checkpoint_crc` 不匹配 → `Err(CheckpointCrcMismatch)`
- [ ] **测试**：`manifest_offset` 超出 `container_end` → `Err(ManifestOffsetOutOfRange)`
- [ ] 实现 checkpoint record 读写

### 7.3 Manifest Hash 校验

- [ ] **测试**：构造 manifest payload → 计算 blake3 → 存入 checkpoint → 校验通过
- [ ] **测试**：篡改 manifest 一个字节 → `manifest_hash` 校验失败
- [ ] **测试**：`manifest_hash` 覆盖 manifest record payload（含 4 字节固定前缀，不含 13 字节 record 前缀）
- [ ] 实现 `compute_manifest_hash(manifest_payload) -> [u8; 32]`

---

## Phase 8：Lock 文件（§19）

### 8.1 独占写锁

- [ ] **测试**：获取写锁 → 锁文件创建，包含 `pid / hostname / username / command / started_at / write_start_offset`
- [ ] **测试**：已有写锁 → 第二次获取返回 `Err(ContainerLocked)`
- [ ] **测试**：释放写锁 → 锁文件删除
- [ ] **测试**：lockfile 路径 = `{container_path}.lock`
- [ ] 实现 `src/xunbak/lock.rs`：`LockFile` + `acquire_write_lock()` + `release()`

### 8.2 锁文件内容

- [ ] **测试**：锁文件为 JSON，可反序列化回 `LockInfo` 结构体
- [ ] **测试**：`tool_version` 字段存在且非空
- [ ] 实现 `LockInfo` 结构体 + serde

---

## Phase 9：Writer — 首次创建（§13.1）

### 9.1 创建空容器

- [ ] **测试**：调用 `create_container(path)` → 文件存在，大小 >= `64 + 13 + ? + 141 + 24`（Header + 空 Manifest Record + Checkpoint + Footer）
- [ ] **测试**：读取 Header → magic 正确，`write_version = 1`，`min_reader_version = 1`
- [ ] **测试**：读取 Footer → `checkpoint_offset` 指向合法 checkpoint
- [ ] **测试**：读取 Checkpoint → `blob_count = 0`，`file_count` in manifest = 0
- [ ] **测试**：读取 Manifest → `entries = []`，`removed = []`
- [ ] **测试**：verify quick 通过
- [ ] 实现 `src/xunbak/writer.rs`：`ContainerWriter::create(path) -> Result<Self>`

### 9.2 单文件写入

- [ ] **测试**：备份一个含 3 个小文件的目录 → 容器包含 3 个 blob + 1 manifest + 1 checkpoint + footer
- [ ] **测试**：manifest 中 3 个 entry，`path / blob_id / size / mtime_ns` 正确
- [ ] **测试**：每个 entry 的 `blob_offset` 指向正确的 record 起始位置
- [ ] **测试**：每个 entry 的 `blob_len == 13 + record_len`
- [ ] **测试**：`content_hash == blob_id`（单 blob 文件）
- [ ] **测试**：checkpoint 统计字段正确（`blob_count = 3`, `total_container_bytes` 等）
- [ ] 实现 `ContainerWriter::backup(source_dir, options) -> BackupResult`

### 9.3 快照上下文元数据（§26）

- [ ] **测试**：manifest 中包含 `snapshot_context`，含 `hostname / username / os / xunyu_version`
- [ ] **测试**：`snapshot_id` 为合法 ULID
- [ ] 实现 `SnapshotContext` 收集

---

## Phase 10：Writer — 增量更新（§13.2）

### 10.1 content_hash 去重

- [ ] **测试**：首次备份 3 文件 → 更新时 1 文件内容不变、1 文件修改、1 文件新增 → 容器只新增 2 个 blob
- [ ] **测试**：未变化文件复用旧 blob，manifest entry 的 `blob_offset / blob_len` 指向旧位置
- [ ] **测试**：rename 但内容不变 → 不写新 blob，新 manifest 中路径更新、`blob_id` 复用
- [ ] 实现 `build_content_hash_index(manifest) -> HashMap<[u8;32], BlobLocator>`

### 10.2 文件级 diff 集成

- [ ] **测试**：复用 `scan.rs` 扫描目录 → 与 manifest baseline diff → 产出 `New / Modified / Unchanged / Deleted` 列表
- [ ] **测试**：`size + mtime_ns` 未变 → `Unchanged`
- [ ] **测试**：文件被删除 → 不出现在新 manifest 中
- [ ] 实现 `diff_against_manifest(scan_result, manifest) -> Vec<DiffEntry>`

### 10.3 增量追加写入

- [ ] **测试**：打开已有容器 → seek 到旧 footer 位置 → 追加新 blob + manifest + checkpoint + footer
- [ ] **测试**：旧 footer 被覆盖，不残留在文件中间
- [ ] **测试**：新 checkpoint 的 `prev_checkpoint_offset = 0`（当前阶段不链接）
- [ ] **测试**：顺序扫描能找到所有 record（旧 + 新），无间隙
- [ ] 实现 `ContainerWriter::update(container_path, source_dir) -> UpdateResult`

### 10.4 两阶段 flush（§13.3）

- [ ] **测试**：模拟第一阶段 flush 后、第二阶段 flush 前崩溃 → 旧 checkpoint 仍有效
- [ ] **测试**：旧 footer 仍指向旧 checkpoint → 恢复到旧快照完整性正确
- [ ] 实现两阶段 flush 逻辑（blob+manifest → fsync → checkpoint+footer → fsync）

---

## Phase 11：Reader — 容器打开与定位

### 11.1 Footer 定位

- [ ] **测试**：打开合法容器 → `read_footer()` 成功 → 定位 checkpoint
- [ ] **测试**：文件 < 88 字节（`HEADER + FOOTER` 最小值不成立）→ `Err(ContainerTooSmall)`
- [ ] 实现 `ContainerReader::open(path) -> Result<Self>`

### 11.2 Footer Fallback

- [ ] **测试**：破坏 footer magic → fallback 顺序扫描 → 找到最后一个合法 checkpoint
- [ ] **测试**：footer CRC 损坏 → 同上
- [ ] **测试**：footer + 所有 record CRC 均损坏 → `Err(UnrecoverableContainer)`
- [ ] 实现 `fallback_scan(reader) -> Option<CheckpointPayload>`

### 11.3 Manifest 加载

- [ ] **测试**：从 checkpoint 定位 manifest → 读取并校验 `manifest_hash` → 解析成功
- [ ] **测试**：`manifest_hash` 不匹配 → `Err(ManifestHashMismatch)`
- [ ] 实现 `load_manifest(reader, checkpoint) -> Result<ManifestBody>`

---

## Phase 12：Restore — 恢复流程（§14）

### 12.1 全量恢复

- [ ] **测试**：备份 3 文件 → 恢复到新目录 → 文件内容、大小完全一致
- [ ] **测试**：恢复后 `mtime` 精度到 100ns（Windows FILETIME 精度）
- [ ] **测试**：恢复后 `win_attributes` 还原（readonly / hidden）
- [ ] **测试**：恢复后 `created_time` 还原
- [ ] **测试**：恢复目录结构正确（嵌套子目录）
- [ ] 实现 `restore_all(reader, target_dir) -> RestoreResult`

### 12.2 单文件恢复

- [ ] **测试**：指定路径恢复单个文件 → 只写出该文件
- [ ] **测试**：路径不存在 → `Err(PathNotFound)`
- [ ] **测试**：大小写不敏感匹配（Windows 语义）
- [ ] 实现 `restore_file(reader, path, target_dir) -> RestoreResult`

### 12.3 Glob 恢复

- [ ] **测试**：`*.rs` → 只恢复 `.rs` 文件
- [ ] **测试**：`src/**/*.rs` → 只恢复 src 下的 `.rs` 文件
- [ ] **测试**：无匹配 → 返回 0 文件恢复 + 警告
- [ ] 实现 `restore_glob(reader, pattern, target_dir) -> RestoreResult`

### 12.4 Blob 读取与校验

- [ ] **测试**：按 `blob_offset` seek → 读取 → 解压 → `blake3(content) == content_hash`
- [ ] **测试**：blob 数据被篡改 → hash 不匹配 → `Err(BlobIntegrityError)`
- [ ] **测试**：恢复读取计划按 `blob_offset` 升序（顺序 I/O 优化）
- [ ] 实现 `read_and_verify_blob(reader, entry) -> Vec<u8>`

### 12.5 空容器恢复（§13.6）

- [ ] **测试**：空容器恢复 → 产出空目录，无报错
- [ ] 实现空容器恢复路径

---

## Phase 13：Verify — 完整性校验（§15）

### 13.1 Quick Verify

- [ ] **测试**：合法容器 → quick verify 通过
- [ ] **测试**：footer 损坏 → quick verify 失败（或 fallback 后通过）
- [ ] **测试**：checkpoint CRC 损坏 → quick verify 失败
- [ ] **测试**：manifest 无法解析 → quick verify 失败
- [ ] **测试**：`manifest_hash` 不匹配 → quick verify 失败
- [ ] **测试**：容器过小 → quick verify 早期拒绝
- [ ] 实现 `verify_quick(reader) -> VerifyReport`

### 13.2 Full Verify

- [ ] **测试**：所有 blob 解压 + blake3 校验通过
- [ ] **测试**：某个 blob 数据损坏 → full verify 报告该 entry 的 `path / blob_id / offset`
- [ ] **测试**：codec 不匹配 → full verify 报告
- [ ] 实现 `verify_full(reader) -> VerifyReport`

### 13.3 Paranoid Verify

- [ ] **测试**：全容器顺序扫描，逐条 `record_crc` 校验通过
- [ ] **测试**：record 边界连续性：`offset_i + 13 + record_len_i == offset_{i+1}`
- [ ] **测试**：某条 record CRC 损坏 → paranoid verify 报告 offset 和 record_type
- [ ] 实现 `verify_paranoid(reader) -> VerifyReport`

### 13.4 Verify 报告

- [ ] **测试**：`VerifyReport` 包含 `level / passed / errors / stats(blob_count, manifest_entries, elapsed)`
- [ ] **测试**：JSON 输出格式正确
- [ ] 实现 `VerifyReport` 结构体 + 格式化输出

---

## Phase 14：并行压缩流水线（§13.5）

### 14.1 单线程基线

- [ ] **测试**：单线程顺序写入 100 个小文件 → 容器正确
- [ ] 确认单线程基线可用作对照

### 14.2 多线程流水线

- [ ] **测试**：并行压缩 100 个文件 → 容器与单线程结果逻辑等价（manifest entries 一致，blob_id 一致）
- [ ] **测试**：写入仍为单线程顺序 append（blob_offset 单调递增）
- [ ] **测试**：线程数 = 1 时退化为顺序模式
- [ ] 实现 `parallel_compress_pipeline(files, codec, num_threads) -> Vec<CompressedBlob>`

### 14.3 流式 blake3 + compress

- [ ] **测试**：流式处理 10 MB 文件 → 内存峰值不超过 2 * chunk_size
- [ ] **测试**：流式 blake3 结果 == 一次性 blake3 结果
- [ ] 实现流式 hasher+compressor 组合

---

## Phase 15：CLI 集成（§18）

### 15.1 backup --container

- [ ] **测试**：`xun backup --container project.xunbak` → 创建合法容器
- [ ] **测试**：第二次执行 → 增量更新
- [ ] **测试**：`--compression none / zstd / zstd:9` 参数解析正确
- [ ] 实现 CLI 入口，调用 writer

### 15.2 restore

- [ ] **测试**：`xun restore project.xunbak` → 全量恢复
- [ ] **测试**：`xun restore project.xunbak --file path/to/file` → 单文件恢复
- [ ] **测试**：`xun restore project.xunbak --glob "*.rs"` → glob 恢复
- [ ] 实现 CLI 入口，调用 reader

### 15.3 verify

- [ ] **测试**：`xun verify project.xunbak` → 默认 quick
- [ ] **测试**：`xun verify project.xunbak --level full` → full verify
- [ ] **测试**：`xun verify project.xunbak --level paranoid` → paranoid verify
- [ ] 实现 CLI 入口

### 15.4 进度输出

- [ ] **测试**：长任务（>100 文件）输出进度信息（已处理字节 / 文件数 / 吞吐）
- [ ] 实现进度回调机制

---

## Phase 16：端到端集成测试

### 16.1 往返正确性

- [ ] **测试**：备份 → 恢复 → 逐文件 blake3 对比 → 全部一致
- [ ] **测试**：含中文路径、空格、长路径（> 260 字符）
- [ ] **测试**：含 readonly / hidden 属性文件
- [ ] **测试**：含空文件（0 字节）
- [ ] **测试**：含已压缩文件（.zip / .jpg）→ codec=None 不二次压缩

### 16.2 增量正确性

- [ ] **测试**：首次备份 → 修改 1 文件 → 增量更新 → 恢复 → 修改的文件内容正确，未修改的也正确
- [ ] **测试**：删除 1 文件 → 增量更新 → 恢复 → 该文件不存在
- [ ] **测试**：新增 1 文件 → 增量更新 → 恢复 → 新文件存在
- [ ] **测试**：rename 文件（内容不变）→ 增量更新 → blob 不重复写入

### 16.3 崩溃恢复

- [ ] **测试**：写入中间截断容器（模拟崩溃） → 旧 checkpoint 仍可用 → 恢复到上一快照
- [ ] **测试**：footer 被截断 → fallback 扫描成功

### 16.4 边界场景

- [ ] **测试**：空目录备份 → 空容器 → verify 通过 → 恢复产出空目录
- [ ] **测试**：单文件目录备份 → 正确
- [ ] **测试**：10,000 文件目录备份 → 正确（规模验证）

---

## Phase 17：基准测试

### 17.1 Divan 基准

- [ ] 新建 `benches/xunbak_bench_divan.rs`
- [ ] **bench**：`header_roundtrip` — Header 序列化/反序列化
- [ ] **bench**：`blob_write_1kb / 1mb / 10mb` — 不同大小 blob 写入
- [ ] **bench**：`compress_zstd_1mb` — 1 MB 数据 zstd 压缩
- [ ] **bench**：`backup_100_files` — 100 个小文件完整备份
- [ ] **bench**：`backup_incremental_10pct` — 10% 变更增量更新
- [ ] **bench**：`restore_100_files` — 100 个文件全量恢复
- [ ] **bench**：`verify_quick / verify_full` — 校验耗时
- [ ] 记录基线到 `logs/xunbak_baseline.md`

---

## 依赖关系

```text
Phase 0 ──→ Phase 1 ──→ Phase 2 ──→ Phase 3
                                       │
Phase 5（codec）─────────────────→ Phase 4（blob 依赖 codec）
                                       │
Phase 6（manifest）─── Phase 7（checkpoint 依赖 manifest hash）
     │                      │
     └────────┬─────────────┘
              ↓
Phase 8（lock）──→ Phase 9（writer create）──→ Phase 10（writer update）
                                                    │
Phase 11（reader）──→ Phase 12（restore）            │
     │                                              │
     └──→ Phase 13（verify）                        │
                                                    │
Phase 14（并行）←───────────────────────────────────┘
     │
Phase 15（CLI）──→ Phase 16（E2E）──→ Phase 17（bench）
```

---

## 测试运行命令

```bash
# 单元 + 集成测试
cargo test --test test_xunbak --features xunbak

# 基准测试
cargo bench --bench xunbak_bench_divan --features xunbak

# 快速编译验证
cargo build --lib --features xunbak
```
