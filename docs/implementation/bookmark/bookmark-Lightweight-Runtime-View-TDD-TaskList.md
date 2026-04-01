# xun bookmark — Lightweight Runtime View TDD 开发任务清单

> **版本**：1.0 · **日期**：2026-04-01  
> **关联文档**：bookmark-Lightweight-Runtime-View-Evaluation.md · bookmark-Benchmark-Suite.md · bookmark-Binary-Cache-Design.md  
> **范围声明**：
>
> 1. 本清单**只覆盖 lightweight runtime view 第一阶段**
> 2. 只覆盖 **cache-hit + 只读热路径**
> 3. 不替换主存储
> 4. 不替换 owned `Store`
> 5. 不改 mutation 路径语义
> 6. 不包含 Dashboard / SQLite
> 7. **一个节点只做一件事**
> 8. 严格遵循：**Red → Green → Refactor**

---

## 0.1 当前状态

- 当前主运行时仍是 owned `Store + Bookmark`
- 当前 binary cache 已经切到 `rkyv`，并已具备 cache-hit 热路径 timing
- 当前 query 内核已经不是主要热点
- 已完成：Phase A 契约层
- 已完成：Phase B 借用模型层
- 已完成：Phase C 缓存读取层
- 已完成：Phase D 查询层
- 已完成：Phase E 消费层
- 已完成：Phase F 边界层
- 已完成：Phase G 性能层
- 当前热点已经收口到：
  - `20k` 热命中：`deserialize_cache_payload≈15ms`
  - `50k` 热命中：`materialize_cache_payload≈12ms`
  - `50k` 热命中：`deserialize_cache_index≈6ms`
- 当前结论：**应优先减少 cache-hit 后完整 owned `Bookmark` 实体化**

---

## 0.2 执行顺序总览

本清单按依赖顺序组织：

1. **契约层**：明确 lightweight view 的边界与适用范围
2. **借用模型层**：定义 archived row / ranked view / owner
3. **缓存读取层**：从 cache-hit 读取 borrowed view，而不是 owned `Store`
4. **查询层**：让 query core 能返回 borrowed 结果
5. **消费层**：只读命令、completion、输出适配 borrowed 结果
6. **边界层**：cache miss / mutation 路径保持旧语义
7. **性能层**：release 基准与阶段 timing 验证收益
8. **文档层**：评估、benchmark、任务状态同步

---

## Phase A — 契约层

> 目标：先把 lightweight runtime view 的边界定死，防止“越做越像第二个 Store”。

### A01 lightweight view 只服务内部加速层

- **Red**
  编写测试 `test_lightweight_view_is_internal_acceleration_layer_only`
  预期：CLI help / config / shell init 不暴露新概念
- **Green**
  在模块注释和文档中明确：它只是内部只读加速层
- **Refactor**
  所有命名统一使用 `view` / `archived`，不使用 `store2` / `lite_store`

### A02 第一阶段只允许 cache-hit 读路径使用

- **Red**
  编写测试 `test_lightweight_view_not_used_for_cache_miss`
- **Green**
  只在 cache-hit 路径进入 lightweight view
- **Refactor**
  把入口判断集中到单一分流函数

### A03 第一阶段不允许 mutation 直接消费 lightweight view

- **Red**
  编写测试 `test_mutation_commands_still_require_owned_store`
- **Green**
  `set / save / rename / delete / pin / unpin / import / learn / touch / undo / redo` 全部继续走 owned `Store`
- **Refactor**
  只读命令与写命令分流条件集中

### A04 第一阶段只覆盖纯只读命令

- **Red**
  编写测试 `test_stage1_lightweight_view_command_scope_fixed`
  预期：仅 `__complete`、`--list`、`--why`、`--preview` 进入新路径
- **Green**
  明确受支持命令白名单
- **Refactor**
  白名单集中为常量或单一判断函数

### A05 输出结果必须与 owned 路径完全一致

- **Red**
  编写测试 `test_lightweight_view_output_parity_contract`
- **Green**
  定义“排序、内容、格式完全一致”契约
- **Refactor**
  文档列出 parity 维度

---

## Phase B — 借用模型层

> 目标：先把“借什么、怎么借、生命周期由谁持有”做成稳定的数据层。

### B01 定义 payload owner

- **Red**
  编写测试 `test_archived_payload_owner_keeps_buffer_alive`
- **Green**
  定义持有 aligned payload buffer 的 owner 结构
- **Refactor**
  owner 只负责 buffer 生命周期，不混入业务字段

### B02 定义 `BookmarkArchivedRow<'a>`

- **Red**
  编写测试 `test_bookmark_archived_row_exposes_read_only_fields`
- **Green**
  定义只读 archived row，覆盖：
  `id / name / path / tags / source / pinned / desc / workspace / created_at / last_visited / visit_count / frecency_score`
- **Refactor**
  字段访问统一通过 accessor，而不是散落取值

### B03 archived row 提供 `BookmarkRecordView` 桥接

- **Red**
  编写测试 `test_archived_row_can_project_to_bookmark_record_view`
- **Green**
  为 query core 提供 borrowed bridge
- **Refactor**
  owned / borrowed 共用同一套 scoring 输入结构

### B04 定义 `RankedBookmarkView<'a>`

- **Red**
  编写测试 `test_ranked_bookmark_view_contains_row_and_score_factors`
- **Green**
  为只读 query 结果定义 borrowed ranked view
- **Refactor**
  与现有 `RankedBookmark` 保持字段布局语义一致

### B05 archived tags 只读遍历能力存在

- **Red**
  编写测试 `test_archived_row_tags_iterates_without_owned_clone`
- **Green**
  为 tags 暴露 borrowed 迭代接口
- **Refactor**
  tags 输出与 query 匹配复用同一入口

### B06 archived index 只读恢复结构存在

- **Red**
  编写测试 `test_archived_index_view_restores_lookup_structure`
- **Green**
  定义 lightweight index 恢复结果
- **Refactor**
  与 owned `BookmarkIndex` 接口对齐

---

## Phase C — 缓存读取层

> 目标：从 cache-hit 直接读出 borrowed view，不再先 materialize 完整 owned `Bookmark`。

### C01 cache-hit 读取 owner 成功

- **Red**
  编写测试 `test_load_cache_owner_checked_returns_owner_on_hit`
- **Green**
  从 binary cache 返回 archived payload owner
- **Refactor**
  header 校验 / aligned payload / access 分层

### C02 cache-hit 读取 row slice 成功

- **Red**
  编写测试 `test_cache_owner_exposes_archived_row_slice`
- **Green**
  让 owner 能暴露 archived bookmark slice
- **Refactor**
  访问逻辑不泄漏 `rkyv` 细节到上层命令

### C03 cache-hit 读取 archived index 成功

- **Red**
  编写测试 `test_cache_owner_exposes_archived_index_view`
- **Green**
  让 owner 能暴露 archived index 或其轻量恢复结果
- **Refactor**
  index 读取与 bookmark row 读取分离

### C04 cache-hit 只恢复 query 必需字段

- **Red**
  编写测试 `test_cache_hit_read_path_avoids_full_owned_bookmark_materialization`
- **Green**
  只为旧路径保留 owned 恢复；新路径不做整库 `Bookmark` clone
- **Refactor**
  明确旧、新两条读取 API

### C05 cache miss 自动回退旧路径

- **Red**
  编写测试 `test_lightweight_read_falls_back_to_owned_store_on_cache_miss`
- **Green**
  cache miss / invalid 时继续走当前 owned `Store::load`
- **Refactor**
  fallback 路径统一到单点

---

## Phase D — 查询层

> 目标：让 query core 可以直接消费 borrowed row，并返回 borrowed ranked 结果。

### D01 borrowed query 输入存在

- **Red**
  编写测试 `test_query_core_accepts_archived_rows_as_input`
- **Green**
  为 query 增加 borrowed 输入路径
- **Refactor**
  owned / borrowed 输入共享相同 scoring 逻辑

### D02 borrowed query 排序与 owned 完全一致

- **Red**
  编写测试 `test_borrowed_query_order_matches_owned_query_order`
- **Green**
  borrowed query 与 owned query 排序完全一致
- **Refactor**
  排序比较器只保留一份

### D03 borrowed query 支持 `limit`

- **Red**
  编写测试 `test_borrowed_query_respects_limit`
- **Green**
  borrowed query 复用当前 top-k 行为
- **Refactor**
  `limit` / `select_nth_unstable_by` 逻辑共用

### D04 borrowed query 支持 scope

- **Red**
  编写测试 `test_borrowed_query_respects_scope`
- **Green**
  `Auto / Global / Child / BaseDir / Workspace` 行为与 owned 一致
- **Refactor**
  `compute_scope_mult` 输入继续保持统一桥接接口

### D05 borrowed query 支持 tag 过滤

- **Red**
  编写测试 `test_borrowed_query_respects_tag_filter`
- **Green**
  tag 过滤与 owned 路径一致
- **Refactor**
  tag 匹配函数统一

### D06 completion 走 borrowed query

- **Red**
  编写测试 `test_completion_prefers_borrowed_query_on_cache_hit`
- **Green**
  `__complete` 热路径不再先走 owned `Store`
- **Refactor**
  completion 与 `z --list` 的 borrowed query 共享入口

---

## Phase E — 消费层

> 目标：让只读命令能直接消费 borrowed ranked view，而不是强制依赖 owned `Bookmark`。

### E01 TSV 输出支持 borrowed ranked view

- **Red**
  编写测试 `test_tsv_output_parity_between_borrowed_and_owned`
- **Green**
  TSV 输出直接消费 borrowed 结果
- **Refactor**
  TSV formatter 抽成 shared helper

### E02 JSON 输出支持 borrowed ranked view

- **Red**
  编写测试 `test_json_output_parity_between_borrowed_and_owned`
- **Green**
  JSON 输出直接消费 borrowed 结果
- **Refactor**
  JSON formatter 抽成 shared helper

### E03 Text/Explain 输出支持 borrowed ranked view

- **Red**
  编写测试 `test_text_and_explain_output_parity_between_borrowed_and_owned`
- **Green**
  `--why` / explain 直接消费 borrowed 结果
- **Refactor**
  说明文本生成逻辑共享

### E04 Preview 输出支持 borrowed ranked view

- **Red**
  编写测试 `test_preview_output_parity_between_borrowed_and_owned`
- **Green**
  `--preview` 直接消费 borrowed 结果
- **Refactor**
  preview 与 list 输出共享渲染路径

### E05 `bookmark z --list` 走 borrowed 路径

- **Red**
  编写测试 `test_z_list_uses_borrowed_view_on_cache_hit`
- **Green**
  `z --list` 命中 cache 时不再 materialize owned `Bookmark`
- **Refactor**
  `z / zi / o / oi` 的 list-only 分支共享 borrowed read

### E06 `bookmark list` 走 borrowed 路径

- **Red**
  编写测试 `test_bookmark_list_uses_borrowed_view_on_cache_hit`
- **Green**
  `bookmark list` 在 cache-hit 时直接走 borrowed rows
- **Refactor**
  表格 / tsv / json 渲染共享 row adapter

### E07 `recent / stats / keys / all` 逐项迁移

- **Red**
  编写测试 `test_recent_stats_keys_all_support_borrowed_view`
- **Green**
  第一阶段把这些纯只读命令迁到 borrowed 路径
- **Refactor**
  把共用统计/筛选逻辑抽离

---

## Phase F — 边界层

> 目标：新路径只服务只读命令，写命令不回归。

### F01 `z / zi / o / oi` 默认执行动作仍走 owned `Store`

- **Red**
  编写测试 `test_action_commands_without_list_still_use_owned_store`
- **Green**
  非 list/preview/why 模式维持当前 visit + save 流程
- **Refactor**
  只在命令入口分流，不在深层隐式切换

### F02 cache-hit 与 cache-miss 行为一致

- **Red**
  编写测试 `test_borrowed_and_owned_paths_have_identical_user_visible_behavior`
- **Green**
  cache-hit borrowed path 与 cache-miss owned path 对外一致
- **Refactor**
  parity 测试复用 helper

### F03 fallback 不影响错误语义

- **Red**
  编写测试 `test_lightweight_view_fallback_preserves_error_messages`
- **Green**
  cache 损坏/失效回退后，错误消息与 exit code 不变
- **Refactor**
  错误路径统一

---

## Phase G — 性能层

> 目标：证明 lightweight runtime view 确实把“cache-hit 实体化成本”压下去了。

### G01 新增 lightweight view 专项 timing 字段

- **Red**
  编写测试 `test_lightweight_view_timing_includes_borrowed_read_labels`
- **Green**
  输出 borrowed 读取阶段 timing
- **Refactor**
  timing 标签命名统一

### G02 `20k` 热命中 `__complete` release 对照

- **Red**
  编写测试 `perf_bookmark_release_complete_20000_lightweight_view`
- **Green**
  输出 lightweight view 前后对照
- **Refactor**
  与现有 compare matrix 统一

### G03 `50k` 热命中 `__complete` release 对照

- **Red**
  编写测试 `perf_bookmark_release_complete_50000_lightweight_view`
- **Green**
  输出 lightweight view 前后对照
- **Refactor**
  与现有 compare matrix 统一

### G04 `20k` 热命中 `store_load` 分段对照

- **Red**
  编写测试 `perf_bookmark_store_load_20000_lightweight_view_breakdown`
- **Green**
  验证 `materialize_cache_payload` 不再是主热点
- **Refactor**
  指标输出统一格式

### G05 `50k` 热命中 `store_load` 分段对照

- **Red**
  编写测试 `perf_bookmark_store_load_50000_lightweight_view_breakdown`
- **Green**
  验证 `materialize_cache_payload` 明显下降
- **Refactor**
  指标输出统一格式

### G06 默认开启门槛判断

- **Red**
  编写测试 `test_lightweight_view_stage1_only_enabled_after_parity_and_perf_threshold_pass`
- **Green**
  以 parity + release 结果决定是否默认开启
- **Refactor**
  文档与门槛常量对齐

---

## Phase H — 文档层

> 目标：评估、任务、benchmark 三份文档口径一致。

### H01 评估文档状态同步

- **Red**
  编写检查项 `check_lightweight_view_evaluation_marks_stage1_status`
- **Green**
  更新评估文档实施状态
- **Refactor**
  增加“已验证有效/无效尝试”摘要

### H02 Benchmark 文档同步

- **Red**
  编写检查项 `check_benchmark_doc_tracks_lightweight_view_results`
- **Green**
  写入 lightweight view 的 release 基线
- **Refactor**
  大库结果整理成固定表格

### H03 任务清单状态同步

- **Red**
  编写检查项 `check_tasklist_tracks_completed_phases`
- **Green**
  更新本任务清单的阶段状态
- **Refactor**
  只保留仍有执行价值的待办

---

## 1. 实施建议顺序

建议严格按以下顺序推进：

1. Phase A 契约层
2. Phase B 借用模型层
3. Phase C 缓存读取层
4. Phase D 查询层
5. Phase E 消费层
6. Phase F 边界层
7. Phase G 性能层
8. Phase H 文档层

---

## 2. 一句话版本

> **先把 lightweight runtime view 的边界定死，再定义 archived row / ranked view 和 payload owner，随后把 cache-hit 的只读 query/list/completion 切到 borrowed 路径，最后用 20k / 50k 的 release timing 证明它确实降低了 cache-hit 后的实体化成本。**
