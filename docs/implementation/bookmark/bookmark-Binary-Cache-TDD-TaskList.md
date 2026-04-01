# xun bookmark — Binary Cache TDD 开发任务清单

> **版本**：1.0 · **日期**：2026-03-31  
> **关联文档**：bookmark-Binary-Cache-Design.md · bookmark-Benchmark-Suite.md · bookmark-PRD.md  
> **范围声明**：
>
> 1. 本清单**只覆盖 binary fast-load cache**
> 2. 不包含 Dashboard
> 3. 不包含 SQLite 迁移
> 4. 不包含 `mmap` / 异步重建 / unchecked access 默认启用
> 5. **一个节点只做一件事**
> 6. 严格遵循：**Red → Green → Refactor**

---

## 0.1 当前状态

- 当前主库存储：compact JSON
- 当前主要热点：`store_load` 的 JSON 解析
- 当前已完成：持久化倒排索引、delta-based `undo / redo`
- 当前实现状态：header / 文件层 / payload / 状态层 / 消费层 / 鲁棒性层已打通
- 当前 payload 状态：已切到 `rkyv` checked 二进制 payload，读取实现对齐 `rkyv` 官方 `to_bytes / from_bytes / AlignedVec<16>` 用法
- 当前 release 结论：`Store::load(20k)` 约 `155ms -> 34ms`，`Store::load(50k)` 约 `396ms -> 86ms`
- 当前目标：增加 **JSON 主库 + 二进制 fast-load cache**

---

## 0.2 执行顺序总览

本清单按依赖顺序组织：

1. **契约层**：cache 文件名、header、版本、hash、flags
2. **纯函数层**：header 编解码、hash 计算、元数据比较
3. **文件层**：cache 读写、锁、原子替换
4. **payload 层**：`Bookmark` / `BookmarkIndex` 的缓存表示
5. **状态层**：`Store::load` / `Store::save` 接入 cache
6. **消费层**：命令、completion、debug 不感知后端差异
7. **鲁棒性层**：损坏、失效、并发踩踏、fallback
8. **性能层**：release 基准与门槛

---

## Phase A — 契约层

> 目标：先把 cache 格式、版本、文件名、失效规则定死。

### A01 cache 文件路径固定

- **Red**
  编写测试 `test_store_cache_path_uses_xun_bookmark_cache_name`
  输入：主库路径 `C:/tmp/.xun.bookmark.json`
  预期：cache 路径为 `C:/tmp/.xun.bookmark.cache`
- **Green**
  实现 `store_cache_path()`
- **Refactor**
  提取 `CACHE_FILE_NAME` 常量

### A02 cache 锁文件路径固定

- **Red**
  编写测试 `test_store_cache_lock_path_uses_lock_suffix`
  预期：锁文件名为 `.xun.bookmark.cache.lock`
- **Green**
  实现 `store_cache_lock_path()`
- **Refactor**
  提取 `CACHE_LOCK_FILE_NAME` 常量

### A03 cache 版本常量存在

- **Red**
  编写测试 `test_store_cache_version_is_one`
- **Green**
  定义 `STORE_CACHE_VERSION = 1`
- **Refactor**
  版本常量集中到 `cache.rs`

### A04 header 固定长度为 52 字节

- **Red**
  编写测试 `test_cache_header_size_is_52_bytes`
- **Green**
  定义 `CacheHeader` 与 `HEADER_SIZE = 52`
- **Refactor**
  避免魔法数字散落

### A05 header 使用固定 magic

- **Red**
  编写测试 `test_cache_header_magic_matches_xun_bookmark_cache`
- **Green**
  定义 8-byte magic
- **Refactor**
  提取 `CACHE_MAGIC`

### A06 header 不单独保留 codec 字段

- **Red**
  编写测试 `test_cache_header_has_no_payload_codec_field`
  预期：header 字段中不存在独立 codec
- **Green**
  仅保留 `flags`
- **Refactor**
  通过注释说明“换 codec => bump cache_version”

### A07 source_hash 为硬性字段

- **Red**
  编写测试 `test_cache_header_contains_source_hash`
- **Green**
  在 header 中加入 `source_hash: u64`
- **Refactor**
  提取字段读写辅助

### A08 flags 预留位定义

- **Red**
  编写测试 `test_cache_flags_encode_checked_and_embedded_index_bits`
- **Green**
  定义 bit0=`checked`，bit1=`embedded_index`
- **Refactor**
  提取 `CacheFlags`

### A09 失效判断规则固定

- **Red**
  编写测试 `test_cache_invalidated_when_version_differs`
  编写测试 `test_cache_invalidated_when_schema_differs`
  编写测试 `test_cache_invalidated_when_source_len_differs`
  编写测试 `test_cache_invalidated_when_source_mtime_differs`
  编写测试 `test_cache_invalidated_when_source_hash_differs`
- **Green**
  实现统一 `is_cache_valid(...)`
- **Refactor**
  失效原因返回枚举而不是 bool

### A10 binary cache 不改变产品边界

- **Red**
  编写测试 `test_binary_cache_is_internal_acceleration_layer_only`
  预期：命令面、query core、undo/redo 语义不变
- **Green**
  在文档/代码注释中明确约束
- **Refactor**
  添加模块级注释

---

## Phase B — 纯函数层

> 目标：先把不碰 IO 的 cache 计算逻辑做成稳定纯函数。

### B01 header 小端序编码

- **Red**
  编写测试 `test_encode_header_writes_little_endian_fields`
- **Green**
  实现 `encode_header()`
- **Refactor**
  字段写入拆成 `write_u32_le()` / `write_u64_le()`

### B02 header 小端序解码

- **Red**
  编写测试 `test_decode_header_reads_little_endian_fields`
- **Green**
  实现 `decode_header()`
- **Refactor**
  字段读取拆成 `read_u32_le()` / `read_u64_le()`

### B03 错误 magic 直接拒绝

- **Red**
  编写测试 `test_decode_header_rejects_invalid_magic`
- **Green**
  校验 magic
- **Refactor**
  返回 `CacheHeaderError::InvalidMagic`

### B04 短头部直接拒绝

- **Red**
  编写测试 `test_decode_header_rejects_short_buffer`
- **Green**
  检查 `len < HEADER_SIZE`
- **Refactor**
  统一 `CacheHeaderError::UnexpectedEof`

### B05 `xxh3_64` 计算 JSON 主库哈希

- **Red**
  编写测试 `test_source_hash_same_bytes_same_hash`
  编写测试 `test_source_hash_different_bytes_different_hash`
- **Green**
  实现 `compute_source_hash(bytes)`
- **Refactor**
  隔离 crate 依赖到单一函数

### B06 metadata 三元组提取

- **Red**
  编写测试 `test_source_fingerprint_contains_len_mtime_hash`
- **Green**
  实现 `SourceFingerprint { len, modified_ms, hash }`
- **Refactor**
  `SourceFingerprint::from_path()` 工厂方法

### B07 flags 编解码

- **Red**
  编写测试 `test_cache_flags_roundtrip`
- **Green**
  实现 `CacheFlags::bits()` 与 `from_bits()`
- **Refactor**
  `const fn` 化

### B08 cache payload 元数据判断

- **Red**
  编写测试 `test_cache_validation_returns_reason_enum`
- **Green**
  `validate_cache_header(...) -> Result<(), CacheInvalidReason>`
- **Refactor**
  失效原因实现 `Display`

---

## Phase C — 文件层

> 目标：把 cache 文件与锁文件的读写链路做对。

### C01 读取 header 不读全文件

- **Red**
  编写测试 `test_read_cache_header_reads_only_fixed_prefix`
- **Green**
  实现 `read_cache_header()`
- **Refactor**
  header 读取与 payload 读取分离

### C02 cache 原子写使用 tmp + rename

- **Red**
  编写测试 `test_write_cache_uses_tmp_then_rename`
- **Green**
  实现 `write_cache_atomic()`
- **Refactor**
  复用主库原子写模式

### C03 cache 写入失败不破坏旧文件

- **Red**
  编写测试 `test_write_cache_failure_preserves_previous_cache`
- **Green**
  失败时不覆盖现有 cache
- **Refactor**
  tmp 清理逻辑独立

### C04 cache 锁可获取

- **Red**
  编写测试 `test_cache_lock_acquire_success`
- **Green**
  实现 `CacheLock::acquire()`
- **Refactor**
  锁文件路径由 `store_cache_lock_path()` 提供

### C05 cache 锁冲突时返回非阻塞失败

- **Red**
  编写测试 `test_cache_lock_conflict_returns_none`
- **Green**
  第二个进程/句柄获取失败
- **Refactor**
  统一锁错误类型

### C06 只有写路径持锁

- **Red**
  编写测试 `test_read_cache_path_does_not_require_lock`
- **Green**
  读 cache 不加锁
- **Refactor**
  `load` 和 `rebuild` 路径分离

### C07 主库不存在时不读 cache

- **Red**
  编写测试 `test_load_cache_returns_none_when_source_missing`
- **Green**
  缺失主库直接回退空/默认
- **Refactor**
  提前返回

---

## Phase D — payload 层

> 目标：定义 cache payload 的精确内容，只保留加载态数据。

### D01 `CachedBookmark` 结构存在

- **Red**
  编写测试 `test_cached_bookmark_contains_only_load_state_fields`
- **Green**
  定义 `CachedBookmark`
- **Refactor**
  注释说明不含运行时字段

### D02 `Bookmark -> CachedBookmark`

- **Red**
  编写测试 `test_cached_bookmark_from_bookmark_roundtrip_fields`
- **Green**
  实现 `CachedBookmark::from_bookmark`
- **Refactor**
  提取 source 编码函数

### D03 `CachedBookmark -> Bookmark`

- **Red**
  编写测试 `test_cached_bookmark_into_bookmark_roundtrip`
- **Green**
  实现 `into_bookmark`
- **Refactor**
  统一 `source_code / source_from_code`

### D04 payload 支持内嵌索引

- **Red**
  编写测试 `test_cache_payload_can_embed_index`
- **Green**
  定义 `CachePayload { bookmarks, index }`
- **Refactor**
  `index` 使用 `Option`

### D05 payload 不缓存运行时字段

- **Red**
  编写测试 `test_cache_payload_excludes_dirty_count_last_save_at_oncelock`
- **Green**
  payload 只保留加载态
- **Refactor**
  模块注释声明

### D06 `rkyv` checked access 读取 payload

- **Red**
  编写测试 `test_rkyv_checked_access_reads_cache_payload`
- **Green**
  实现 `read_cache_payload_checked()`
- **Refactor**
  payload 解码与 header 校验分离

### D07 payload validation 失败回退

- **Red**
  编写测试 `test_invalid_payload_returns_none_not_error`
- **Green**
  validation 失败时返回 miss
- **Refactor**
  增加调试日志 reason

---

## Phase E — 状态层

> 目标：把 cache 正式接入 `Store::load` / `Store::save`。

### E01 `Store::load` 先做 `stat`

- **Red**
  编写测试 `test_store_load_reads_source_metadata_before_cache_match`
- **Green**
  先取 `len / mtime / hash`
- **Refactor**
  提取 `SourceFingerprint`

### E02 `Store::load` 命中 cache 直接返回

- **Red**
  编写测试 `test_store_load_uses_binary_cache_when_metadata_matches`
- **Green**
  cache hit 直接返回 Store
- **Refactor**
  提取 `load_from_cache()`

### E03 `Store::load` miss 时回退 JSON

- **Red**
  编写测试 `test_store_load_falls_back_to_json_when_cache_missing`
- **Green**
  miss 回退 JSON
- **Refactor**
  提取 `load_from_json()`

### E04 `Store::load` miss 后重建 cache

- **Red**
  编写测试 `test_store_load_rebuilds_cache_after_json_fallback`
- **Green**
  JSON load 成功后写 cache
- **Refactor**
  `rebuild_cache_after_load()`

### E05 `Store::save` 先写 JSON 再写 cache

- **Red**
  编写测试 `test_store_save_writes_json_before_cache`
- **Green**
  保存顺序固定
- **Refactor**
  `save_json_then_cache()`

### E06 `Store::save` 写 cache 时持锁

- **Red**
  编写测试 `test_store_save_acquires_cache_lock_for_cache_write`
- **Green**
  只在写 cache 时持锁
- **Refactor**
  锁作用域最小化

### E07 `Store::save` 同步写持久化索引到 cache payload

- **Red**
  编写测试 `test_store_save_embeds_persisted_index_into_cache_payload`
- **Green**
  payload 写入 `index`
- **Refactor**
  复用现有 `BookmarkIndex` 持久化表示

### E08 `Store::load` 命中 cache 时恢复索引

- **Red**
  编写测试 `test_store_load_restores_index_from_cache_payload`
- **Green**
  命中 cache 时直接装配 `OnceLock`
- **Refactor**
  提取 `store_with_index()`

### E09 cache hit 不重建索引

- **Red**
  编写测试 `test_store_index_cache_hit_skips_rebuild`
- **Green**
  直接复用 cache payload 中的 index
- **Refactor**
  标志变量改为数据驱动

### E10 环境变量关闭 binary cache

- **Red**
  编写测试 `test_store_load_skips_binary_cache_when_env_disabled`
- **Green**
  实现 `XUN_BM_DISABLE_BINARY_CACHE=1`
- **Refactor**
  提取 `binary_cache_enabled()`

---

## Phase F — 消费层

> 目标：让命令层完全不感知 binary cache 的存在。

### F01 `z` 命中 cache 行为不变

- **Red**
  编写测试 `test_cmd_z_same_result_with_cache_hit_and_json_fallback`
- **Green**
  `cmd_z` 不感知后端
- **Refactor**
  复用公共断言

### F02 `zi / oi` 命中 cache 行为不变

- **Red**
  编写测试 `test_cmd_zi_same_result_with_cache_hit_and_json_fallback`
  编写测试 `test_cmd_oi_same_result_with_cache_hit_and_json_fallback`
- **Green**
  交互命令不感知后端
- **Refactor**
  交互与非交互用同一断言样板

### F03 completion 命中 cache 行为不变

- **Red**
  编写测试 `test_completion_same_order_with_cache_hit`
- **Green**
  completion 仍走同一 query core
- **Refactor**
  helper 复用

### F04 `check / gc / dedup` 命中 cache 行为不变

- **Red**
  编写测试 `test_maintenance_commands_same_behavior_with_cache_hit`
- **Green**
  治理命令不感知后端
- **Refactor**
  批量断言 helper

### F05 `undo / redo` 后 cache 自动更新

- **Red**
  编写测试 `test_undo_redo_refreshes_binary_cache`
- **Green**
  `save_exact` 路径同样写 cache
- **Refactor**
  避免重复写路径

### F06 debug 输出显示 cache hit/miss

- **Red**
  编写测试 `test_bookmark_load_timing_emits_cache_hit_or_miss_reason`
- **Green**
  `BookmarkLoadTiming` 增加 `cache=hit|miss|disabled`
- **Refactor**
  统一 extras 输出

---

## Phase G — 鲁棒性层

> 目标：让 cache 坏了也绝不影响主流程。

### G01 header 损坏回退 JSON

- **Red**
  编写测试 `test_corrupted_header_falls_back_to_json`
- **Green**
  header 解析失败 => miss
- **Refactor**
  `CacheInvalidReason::HeaderCorrupt`

### G02 payload 损坏回退 JSON

- **Red**
  编写测试 `test_corrupted_payload_falls_back_to_json`
- **Green**
  checked access 失败 => miss
- **Refactor**
  `CacheInvalidReason::PayloadCorrupt`

### G03 source_hash 不匹配回退 JSON

- **Red**
  编写测试 `test_source_hash_mismatch_falls_back_to_json`
- **Green**
  强校验生效
- **Refactor**
  mismatch reason 统一

### G04 schema_version 不匹配回退 JSON

- **Red**
  编写测试 `test_schema_version_mismatch_falls_back_to_json`
- **Green**
  schema mismatch => miss
- **Refactor**
  reason 统一

### G05 cache_version 不匹配回退 JSON

- **Red**
  编写测试 `test_cache_version_mismatch_falls_back_to_json`
- **Green**
  cache version mismatch => miss
- **Refactor**
  reason 统一

### G06 读路径不等待锁

- **Red**
  编写测试 `test_cache_read_path_does_not_block_on_lock_holder`
- **Green**
  读 miss 直接回退 JSON
- **Refactor**
  lock 尝试逻辑隔离到写路径

### G07 并发重建仅一个进程写 cache

- **Red**
  编写测试 `test_concurrent_cache_rebuild_only_one_writer_succeeds`
- **Green**
  持锁者写 cache，其他进程仅回退 JSON
- **Refactor**
  writer election 注释清晰化

### G08 cache 写失败不影响命令成功

- **Red**
  编写测试 `test_cache_write_failure_does_not_fail_command`
- **Green**
  cache 写失败仅 warning
- **Refactor**
  统一 warning 文案

### G09 旧 cache 文件自动失效

- **Red**
  编写测试 `test_legacy_cache_file_is_ignored_and_rebuilt`
- **Green**
  v1 之前的 cache 自动忽略
- **Refactor**
  版本判断集中

---

## Phase H — 性能层

> 目标：用 release 基准证明 binary cache 值得默认开启。

### H01 `Store::load(20k)` release 对照

- **Red**
  使用 `perf_bookmark_store_load_20000_compact`
  分别在 `XUN_BM_DISABLE_BINARY_CACHE=1` 与默认环境下执行
- **Green**
  输出 compact JSON 与 warm binary cache hit 两组结果
- **Refactor**
  抽 `measure_store_load_avg_ms_*`

### H02 `Store::load(50k)` release 对照

- **Red**
  使用 `perf_bookmark_store_load_50000_compact`
  分别在 `XUN_BM_DISABLE_BINARY_CACHE=1` 与默认环境下执行
- **Green**
  输出 compact JSON 与 warm binary cache hit 两组结果
- **Refactor**
  抽复用 helper

### H03 `z --list`（20k）release 对照

- **Red**
  编写测试 `perf_bookmark_release_compare_matrix_20000_binary_cache`
- **Green**
  输出 JSON / compact JSON / binary cache 三组结果
- **Refactor**
  对照矩阵统一化

### H04 `__complete`（20k）release 对照

- **Red**
  编写测试 `perf_bookmark_release_complete_20000_binary_cache`
- **Green**
  记录 completion 提速
- **Refactor**
  结果汇总 helper

### H05 working set 归因

- **Red**
  编写测试 `perf_bookmark_memory_attribution_binary_cache`
- **Green**
  输出 empty / compact / binary cache 峰值
- **Refactor**
  统一 MiB 格式化

### H06 cache 命中率调试统计

- **Red**
  编写测试 `test_load_timing_reports_cache_hit_ratio_fields`
- **Green**
  增加命中/失效 reason 统计字段
- **Refactor**
  调试输出格式统一

### H07 默认开启门槛判断

- **Red**
  编写测试 `test_binary_cache_default_enable_only_after_release_threshold_passed`
- **Green**
  以 release 结果定义默认开启判据
- **Refactor**
  文档与门槛常量对齐

> 2026-04-01 当前进度：H01 / H02 / H03 / H04 / H06 已得到实测结果；H05 是否继续扩充为 binary-cache 专项 working-set 对照，可按后续需要补充。

---

## Phase I — 文档与发布收口

> 目标：实现完成后，文档与基准口径统一。

### I01 PRD 状态同步

- **Red**
  编写检查项 `check_prd_mentions_binary_cache_as_load_layer`
- **Green**
  更新 PRD
- **Refactor**
  链接到 Binary Cache 设计文档

### I02 Roadmap 状态同步

- **Red**
  编写检查项 `check_roadmap_marks_binary_cache_phase_status`
- **Green**
  更新路线图
- **Refactor**
  与 SQLite 路线边界明确分离

### I03 Benchmark 文档同步

- **Red**
  编写检查项 `check_benchmark_doc_uses_release_as_primary_conclusion`
- **Green**
  更新 benchmark 文档
- **Refactor**
  大库矩阵整理成统一表格

### I04 设计文档实施状态同步

- **Red**
  编写检查项 `check_binary_cache_design_marks_phase1_done_when_complete`
- **Green**
  在设计文档顶部更新实施状态
- **Refactor**
  增加“已采纳建议”摘要

---

## 1. 实施建议顺序

建议严格按以下顺序推进：

1. Phase A 契约层
2. Phase B 纯函数层
3. Phase C 文件层
4. Phase D payload 层
5. Phase E 状态层
6. Phase F 消费层
7. Phase G 鲁棒性层
8. Phase H 性能层
9. Phase I 文档与发布收口

---

## 2. 一句话版本

> **先把二进制 cache 的格式、锁和失效规则定死，再实现 `rkyv + xxh3_64` 的 payload 与 `Store::load/save` 接线，随后补并发/损坏/fallback 测试，最后用 20k / 50k 的 release 基准证明它值得默认开启。**
