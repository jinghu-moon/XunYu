# xun bookmark — TDD 开发任务清单（精修版）

> **版本**：2.0 · **日期**：2026-03-30  
> **关联文档**：bookmark-PRD.md · Bookmarks-Feature-Roadmap.md · bookmark-Config-Spec.md  
> **执行原则**：
>
> 1. **一个任务只做一件事**
> 2. **严格从底层到消费层推进**
> 3. **先纯函数 / 纯数据，再 Store，再 Query Core，再命令，再 Shell**
> 4. 每个任务都遵循：**Red → Green → Refactor**

## 0.1 当前执行状态（2026-03-30）

- 已完成：Phase A 契约层
- 已完成：Phase B 纯函数层
- 已完成：Phase C 状态层
- 已完成：Phase D 查询层
- 已完成：Phase E 消费层
- 已完成：Phase F 集成层
- 已完成：Phase G 治理与性能基线
- 已完成：Phase H `zi / oi`、completion 最终统一、旧入口清理
- 已完成：Phase J delta-based `undo / redo` 历史重构
- 未纳入本轮：Dashboard 面板、SQLite

---

## 0. 执行顺序总览

本清单按依赖顺序组织：

1. **契约层**：schema、配置、基础类型
2. **纯函数层**：路径标准化、名称标准化、score/frecency/scope 计算
3. **状态层**：Store、持久化、aging、dirty save
4. **查询层**：候选召回、排序、Query Core
5. **消费层**：CLI 命令、输出格式、dead-link、preview
6. **集成层**：导入、自动学习、shell init、completion
7. **治理与性能层**：check/gc/recent、benchmark、`fuzzy` 退场

---

## Phase A — 契约层

> 目标：先把“什么是合法数据、合法配置、合法命令面”定死。

### A01 `bookmark` 子命名空间存在

- **Red**
  编写测试 `test_bookmark_subcommand_registered`
  输入：`xun bookmark --help`
  预期：列出 `z / zi / o / oi / open / save / set / tag / pin / rename / list / recent / stats / check / gc / dedup / export / import / init / touch / learn / keys / all`
- **Green**
  在当前 CLI 定义层注册 `bookmark` 顶层子命令树
- **Refactor**
  提取统一的 bookmark 子命令清单常量

### A02 旧顶层命令不存在

- **Red**
  编写测试 `test_legacy_top_level_bookmark_commands_absent`
  输入：`xun --help`
  预期：不出现旧顶层 `z / o / ws / sv / fuzzy`
- **Green**
  从顶层公共 CLI 中移除旧入口
- **Refactor**
  清理 dispatch 中所有仅为旧顶层命令服务的分支

### A03 `workspace` 动作子命令不存在

- **Red**
  编写测试 `test_workspace_subcommand_absent`
  输入：`xun bookmark workspace --help`
  预期：未知子命令
- **Green**
  确认 parser 中不注册 `workspace` 动作子命令
- **Refactor**
  清理 help / completion / 文档里对 `workspace` 动作子命令的旧引用

### A04 主存储必须显式带 `schema_version`

- **Red**
  编写测试 `test_load_missing_schema_version_fails`
  构造无 `schema_version` 的主存储 JSON
  预期：`Store::load` 返回 `Err(StoreError::MissingSchemaVersion)`
- **Green**
  在 load 逻辑中强制校验 `schema_version`
- **Refactor**
  提取 `parse_schema_version()`

### A05 当前仅支持 `schema_version = 1`

- **Red**
  编写测试 `test_future_schema_version_rejected`
  构造 `schema_version = 2`
  预期：`Err(StoreError::UnsupportedSchemaVersion(2))`
- **Green**
  加入版本检查
- **Refactor**
  提取 `check_schema_version()`

### A06 主存储写出包含 `schema_version`

- **Red**
  编写测试 `test_save_includes_schema_version`
  保存 Store 后检查 JSON 中存在 `"schema_version": 1`
- **Green**
  在 Store 序列化中包含 `schema_version`
- **Refactor**
  保证 `schema_version` 只在根对象出现一次

### A07 `bookmark` section 配置可读取

- **Red**
  编写测试 `test_config_reads_bookmark_section`
  配置文件包含 `"bookmark": { "defaultScope": "child" }`
  预期：`config.bookmark.default_scope == Child`
- **Green**
  在全局配置模型中增加 `bookmark` section
- **Refactor**
  `BookmarkConfig` 所有字段使用 `#[serde(default)]`

### A08 缺失 `bookmark` section 使用默认值

- **Red**
  编写测试 `test_config_missing_bookmark_section_uses_defaults`
- **Green**
  为 `BookmarkConfig` 提供默认值
- **Refactor**
  默认值函数集中到 `config::defaults`

### A09 配置优先级：CLI > Env > Config > Default

- **Red**
  编写测试 `test_config_cli_overrides_env`
  编写测试 `test_config_env_overrides_file`
  编写测试 `test_config_file_overrides_default`
- **Green**
  实现统一 resolve 逻辑
- **Refactor**
  提取 `resolve_scope()` / `resolve_limit()` 等通用覆盖函数

### A10 新数据文件路径名生效

- **Red**
  编写测试 `test_default_data_file_uses_new_name`
  预期：默认文件名包含 `xun.bookmark.json`
- **Green**
  实现 `bookmark.dataFile` 默认路径
- **Refactor**
  路径常量集中到 `default_paths`

### A11 访问日志文件路径可配置

- **Red**
  编写测试 `test_visit_log_file_reads_from_config`
  编写测试 `test_visit_log_file_env_overrides_config`
- **Green**
  实现 `bookmark.visitLogFile` 与 `_BM_VISIT_LOG_FILE`
- **Refactor**
  `dataFile` / `visitLogFile` 共用路径覆盖逻辑

### A12 `_BM_EXCLUDE_DIRS` 按平台解析

- **Red**
  编写测试 `test_exclude_dirs_windows_semicolon_separator`
  编写测试 `test_exclude_dirs_unix_colon_separator`
- **Green**
  实现平台分隔规则
- **Refactor**
  提取 `parse_exclude_dirs_env()`

---

## Phase B — 纯函数层

> 目标：先把不会碰 IO 的纯逻辑做成稳定地基。

### B01 `NormalizedPath` 拥有 display/key

- **Red**
  编写测试 `test_normalized_path_has_display_and_key`
- **Green**
  定义 `NormalizedPath { display, key }`
- **Refactor**
  实现 `Display` / `Eq` / `Hash`

### B02 展开 `~`

- **Red**
  编写测试 `test_tilde_expansion_unix`
  编写测试 `test_tilde_expansion_windows`
  编写测试 `test_tilde_in_middle_not_expanded`
- **Green**
  实现 `expand_tilde()`
- **Refactor**
  HOME/USERPROFILE 读取逻辑统一

### B03 相对路径解析为绝对路径

- **Red**
  编写测试 `test_relative_path_resolved_against_cwd`
  编写测试 `test_dot_path_resolved`
- **Green**
  实现逻辑解析，不要求路径存在
- **Refactor**
  避免依赖 `canonicalize()`

### B04 分隔符统一为 `/`

- **Red**
  编写测试 `test_backslash_converted_to_forward_slash`
  编写测试 `test_mixed_slashes_normalized`
- **Green**
  实现分隔符转换
- **Refactor**
  提取 `normalize_separators()`

### B05 Windows 比较键小写化

- **Red**
  编写测试 `test_windows_key_is_lowercase`
  编写测试 `test_unix_key_preserves_case`
- **Green**
  实现平台差异化 comparison key
- **Refactor**
  提取 `to_comparison_key()`

### B06 去尾斜杠但保留根路径

- **Red**
  编写测试 `test_trailing_slash_removed`
  编写测试 `test_root_trailing_slash_preserved_windows`
  编写测试 `test_root_trailing_slash_preserved_unix`
- **Green**
  实现尾随分隔符处理
- **Refactor**
  提取 `strip_trailing_sep()`

### B07 UNC 路径验证

- **Red**
  编写测试 `test_unc_path_valid`
  编写测试 `test_unc_path_missing_share_fails`
- **Green**
  实现 UNC 验证
- **Refactor**
  提取 `validate_unc()`

### B08 同路径相等性

- **Red**
  编写测试 `test_same_path_different_casing_equals`
  编写测试 `test_same_path_different_separator_equals`
  编写测试 `test_same_path_trailing_slash_equals`
- **Green**
  `NormalizedPath` 按 `key` 比较
- **Refactor**
  `Hash` 同样按 `key`

### B09 名称标准化

- **Red**
  编写测试 `test_name_norm_is_lowercase`
- **Green**
  实现 `normalize_name()`
- **Refactor**
  所有显式名称比较统一走 `name_norm`

### B10 MatchScore: name exact

- **Red**
  编写测试 `test_name_exact_match_scores_100`
  编写测试 `test_name_exact_match_case_insensitive`
- **Green**
  实现 exact 匹配
- **Refactor**
  提取 `score_name_exact()`

### B11 MatchScore: name prefix

- **Red**
  编写测试 `test_name_prefix_match_scores_80`
  编写测试 `test_name_prefix_never_beats_exact`
- **Green**
  实现 prefix 匹配
- **Refactor**
  与 exact 合并为 `score_name()`

### B12 MatchScore: basename exact/prefix

- **Red**
  编写测试 `test_basename_exact_match_scores_70`
  编写测试 `test_basename_prefix_match_scores_60`
- **Green**
  实现 basename 匹配
- **Refactor**
  提取 `extract_basename()`

### B13 MatchScore: segment ordered match

- **Red**
  编写测试 `test_segment_ordered_match`
  编写测试 `test_segment_out_of_order_not_matched`
- **Green**
  实现 ordered segment 匹配
- **Refactor**
  提取 `score_segment_ordered()`

### B14 MatchScore: multi-token AND

- **Red**
  编写测试 `test_multi_token_all_must_match`
  编写测试 `test_multi_token_one_missing_returns_zero`
- **Green**
  实现 token AND
- **Refactor**
  提取 `match_token()`

### B15 MatchScore: tag bonus

- **Red**
  编写测试 `test_tag_hit_adds_bonus`
  编写测试 `test_tag_hit_does_not_beat_name_exact`
- **Green**
  实现 tag bonus
- **Refactor**
  提取 `compute_tag_bonus()`

### B16 MatchScore: 最后 token basename bonus

- **Red**
  编写测试 `test_last_token_basename_bonus`
- **Green**
  实现最后 token 额外加权
- **Refactor**
  用 `is_last_token` 控制

### B17 MatchScore: subsequence fuzzy fallback

- **Red**
  编写测试 `test_subsequence_fuzzy_fallback_range`
  编写测试 `test_subsequence_fuzzy_returns_zero_if_no_subsequence`
  编写测试 `test_fuzzy_never_beats_strong_match`
- **Green**
  实现 subsequence fallback
- **Refactor**
  提取 `subsequence_score()`

### B18 时间衰减桶

- **Red**
  编写测试 `test_decay_under_1_hour`
  编写测试 `test_decay_under_1_day`
  编写测试 `test_decay_under_7_days`
  编写测试 `test_decay_under_30_days`
  编写测试 `test_decay_over_30_days`
- **Green**
  实现 `time_decay()`
- **Refactor**
  常量化 `DECAY_BUCKETS`

### B19 原始 frecency 计算

- **Red**
  编写测试 `test_raw_frecency_formula`
  编写测试 `test_raw_frecency_zero_visits`
- **Green**
  实现 `raw_frecency()`
- **Refactor**
  保持纯函数

### B20 `FrecencyMult` 正常路径

- **Red**
  编写测试 `test_frecency_mult_range`
  编写测试 `test_frecency_mult_max_at_global_max`
  编写测试 `test_frecency_mult_min_at_zero`
- **Green**
  实现 `frecency_mult()`
- **Refactor**
  `global_max` 由调用方传入

### B21 `FrecencyMult` imported seed 路径

- **Red**
  编写测试 `test_frecency_mult_for_imported_with_null_visit_count`
- **Green**
  当 `visit_count/last_visited` 缺失时改用 `frecency_score`
- **Refactor**
  提取 `effective_frecency_score()`

### B22 ScopeMult: Auto 基础关系

- **Red**
  编写测试 `test_scope_exact_same_dir`
  编写测试 `test_scope_bookmark_is_parent_of_cwd`
  编写测试 `test_scope_bookmark_is_child_of_cwd`
  编写测试 `test_scope_same_workspace`
  编写测试 `test_scope_unrelated`
- **Green**
  实现 `compute_scope_mult()`
- **Refactor**
  提取 `path_relationship()`

### B23 ScopeMult: Global

- **Red**
  编写测试 `test_global_scope_always_1_0`
- **Green**
  `Global` 分支短路返回 `1.0`
- **Refactor**
  避免不必要路径计算

### B24 ScopeMult: Child

- **Red**
  编写测试 `test_child_scope_subdir_boosted`
  编写测试 `test_child_scope_non_subdir_penalized`
- **Green**
  实现 Child 分支
- **Refactor**
  系数常量化

### B25 ScopeMult: BaseDir

- **Red**
  编写测试 `test_base_dir_filters_non_matching`
  编写测试 `test_base_dir_keeps_matching`
- **Green**
  实现 BaseDir 过滤
- **Refactor**
  提取 `is_under_base()`

### B26 FinalScore 乘法公式

- **Red**
  编写测试 `test_final_score_formula_correctness`
- **Green**
  实现 `compute_final_score()`
- **Refactor**
  使用 `ScoreFactors`

### B27 SourceMult

- **Red**
  编写测试 `test_source_mult_explicit_1_20`
  编写测试 `test_source_mult_imported_1_05`
  编写测试 `test_source_mult_learned_1_00`
- **Green**
  实现 `source_mult()`
- **Refactor**
  `match` 实现

### B28 PinMult

- **Red**
  编写测试 `test_pin_mult_pinned_1_50`
  编写测试 `test_pin_mult_unpinned_1_00`
- **Green**
  实现 `pin_mult()`
- **Refactor**
  保持纯函数

---

## Phase C — 状态层

> 目标：在纯逻辑稳定后，再构建 Store 和持久化。

### C01 `Bookmark` 新字段 roundtrip

- **Red**
  编写测试 `test_bookmark_roundtrip_with_new_fields`
- **Green**
  为 Bookmark 增加 `source/pinned/name_norm/frecency_score/...`
- **Refactor**
  默认值函数集中

### C02 `Store::set` 同名更新

- **Red**
  编写测试 `test_set_duplicate_name_updates_existing`
- **Green**
  `explicit` 条目按 `name_norm` 更新
- **Refactor**
  提取 `find_explicit_mut()`

### C03 `Store::rename` 冲突报错

- **Red**
  编写测试 `test_rename_to_existing_name_fails`
- **Green**
  重名拒绝覆盖
- **Refactor**
  提取 `assert_name_available()`

### C04 `Store::pin`

- **Red**
  编写测试 `test_pin_sets_pinned_true`
  编写测试 `test_pin_nonexistent_bookmark_fails`
- **Green**
  实现 `pin`
- **Refactor**
  复用显式条目定位逻辑

### C05 `Store::unpin`

- **Red**
  编写测试 `test_unpin_sets_pinned_false`
- **Green**
  实现 `unpin`
- **Refactor**
  复用显式条目定位逻辑

### C06 `Store::set` 创建 explicit

- **Red**
  编写测试 `test_set_command_creates_explicit_bookmark`
- **Green**
  `source = explicit`
- **Refactor**
  统一构造函数

### C07 `Store::learn` 创建 learned

- **Red**
  编写测试 `test_learn_creates_learned_entry`
- **Green**
  `source = learned`
- **Refactor**
  统一构造函数

### C08 `Store::learn` 同路径累计访问

- **Red**
  编写测试 `test_learn_same_path_increments_visit_count`
- **Green**
  learned 同路径增量更新
- **Refactor**
  统一按 `path_norm` 匹配

### C09 `Store::learn` 不覆盖 explicit

- **Red**
  编写测试 `test_learn_does_not_override_explicit`
- **Green**
  learned 同路径命中 explicit 时仅更新访问，不替换 source
- **Refactor**
  提取 `merge_learned_visit_into_explicit()`

### C10 导入条目标记 imported

- **Red**
  编写测试 `test_import_creates_imported_bookmark`
- **Green**
  `source = imported`
- **Refactor**
  统一构造函数

### C11 主存储写出新 schema

- **Red**
  编写测试 `test_store_serializes_schema_v1_shape`
- **Green**
  序列化为新主存储结构
- **Refactor**
  提取 `StoreFile` DTO

### C12 访问日志 append-only

- **Red**
  编写测试 `test_visit_log_appends_entries`
- **Green**
  WAL append-only
- **Refactor**
  单独 `VisitLogWriter`

### C13 Aging 不作用于 explicit/pinned

- **Red**
  编写测试 `test_explicit_bookmarks_survive_aging`
  编写测试 `test_pinned_bookmarks_survive_aging`
- **Green**
  老化时排除 exempt 条目
- **Refactor**
  提取 `is_aging_exempt()`

### C14 dirty count 触发 flush

- **Red**
  编写测试 `test_dirty_save_triggered_after_n_accesses`
  编写测试 `test_dirty_save_not_triggered_below_threshold`
- **Green**
  实现 dirty count flush
- **Refactor**
  提取 `should_flush_by_count()`

### C15 dirty time 触发 flush

- **Red**
  编写测试 `test_dirty_save_triggered_after_t_seconds`
- **Green**
  实现 time-based flush
- **Refactor**
  可 mock 时间接口

### C16 原子写入

- **Red**
  编写测试 `test_save_is_atomic`
- **Green**
  temp file + rename
- **Refactor**
  提取 `atomic_write()`

### C17 空迁移框架

- **Red**
  编写测试 `test_apply_migrations_noop_when_same_version`
- **Green**
  实现 no-op 迁移框架
- **Refactor**
  预留未来 v2+

---

## Phase D — 查询层

> 目标：先有稳定 Query Core，再把所有命令挂上去。

### D01 `BookmarkQuerySpec::default`

- **Red**
  编写测试 `test_query_spec_default_values`
- **Green**
  实现 `Default`
- **Refactor**
  如有必要再引入 builder

### D02 `QueryContext` 捕获 cwd

- **Red**
  编写测试 `test_context_captures_current_dir`
- **Green**
  实现 `QueryContext { cwd, workspace }`
- **Refactor**
  `from_env()` / `from_store()` 拆分

### D03 `QueryContext` 计算 workspace

- **Red**
  编写测试 `test_context_workspace_from_store`
- **Green**
  实现 workspace 解析
- **Refactor**
  独立函数

### D04 空库查询返回空

- **Red**
  编写测试 `test_query_empty_store_returns_empty`
- **Green**
  实现 `bookmark::query()`
- **Refactor**
  固定函数签名

### D05 候选召回：按 token 取交集

- **Red**
  编写测试 `test_candidate_recall_intersects_tokens`
- **Green**
  实现召回交集
- **Refactor**
  召回与精排拆分

### D06 排序后 `explicit` 优先于 learned

- **Red**
  编写测试 `test_explicit_beats_learned_with_same_match`
- **Green**
  接入 `SourceMult`
- **Refactor**
  排序因子结构化

### D07 排序后 `pinned explicit` 优先

- **Red**
  编写测试 `test_pinned_explicit_beats_learned`
- **Green**
  接入 `PinMult`
- **Refactor**
  使用 `total_cmp`

### D08 imported seed frecency 生效

- **Red**
  编写测试 `test_frecency_mult_for_imported_with_null_visit_count`
- **Green**
  导入 seed 走 `frecency_score`
- **Refactor**
  提取 `effective_frecency_score()`

---

## Phase E — 消费层

> 目标：命令只消费 Query Core，不重造第二套逻辑。

### E01 `cmd_z` 消费 Query Core

- **Red**
  编写测试 `test_cmd_z_and_query_have_same_order`
- **Green**
  `cmd_z` 委托 `bookmark::query()`
- **Refactor**
  提取执行动作层

### E02 `cmd_o` 消费 Query Core

- **Red**
  编写测试 `test_cmd_o_and_cmd_z_produce_identical_order`
- **Green**
  `cmd_o` 委托同一 query core
- **Refactor**
  复用 action 执行函数

### E03 `--list` 文本输出

- **Red**
  编写测试 `test_list_output_text_format`
- **Green**
  实现 text formatter
- **Refactor**
  `ResultFormatter`

### E04 `--list --json`

- **Red**
  编写测试 `test_list_output_json_format`
- **Green**
  实现 JSON formatter
- **Refactor**
  复用 DTO

### E05 `--list --tsv`

- **Red**
  编写测试 `test_list_output_tsv_format`
- **Green**
  实现 TSV formatter
- **Refactor**
  与 completion 共享格式约定

### E06 `--score`

- **Red**
  编写测试 `test_score_output_contains_all_factors`
  编写测试 `test_score_values_consistent_with_final_score`
- **Green**
  展示各因子列
- **Refactor**
  提取 `fmt_score()`

### E07 `--why`

- **Red**
  编写测试 `test_why_output_explains_top1`
  编写测试 `test_why_match_score_shows_winning_tier`
- **Green**
  `RankedBookmark` 附带 `ScoreExplanation`
- **Refactor**
  `ScoreExplanation` 实现 `Display`

### E08 歧义提示

- **Red**
  编写测试 `test_ambiguity_hint_shown_when_scores_close`
  编写测试 `test_ambiguity_hint_not_shown_when_scores_far`
  编写测试 `test_ambiguity_hint_does_not_block_jump`
- **Green**
  在 `cmd_z` 结束时输出提示
- **Refactor**
  提取 `is_ambiguous()`

### E09 `--preview`

- **Red**
  编写测试 `test_preview_does_not_output_bm_cd`
  编写测试 `test_preview_shows_candidates_and_scores`
  编写测试 `test_preview_shows_dry_run_header`
- **Green**
  preview 跳过执行动作
- **Refactor**
  preview / explain 标志合流

### E10 dead-link 本地路径检测

- **Red**
  编写测试 `test_dead_link_detected_for_local_path`
  编写测试 `test_live_link_passes_through`
- **Green**
  命中后检测路径存在性
- **Refactor**
  提取 `check_path_exists()`

### E11 dead-link UNC 超时保护

- **Red**
  编写测试 `test_unc_path_check_respects_timeout`
  编写测试 `test_unc_path_timeout_does_not_fail_command`
- **Green**
  实现带超时检测
- **Refactor**
  超时时间从配置读取

---

## Phase F — 集成层

> 目标：在核心能力稳定后，再接导入、自动学习、shell、completion。

### F01 自动学习创建 learned 条目

- **Red**
  编写测试 `test_learn_creates_learned_entry`
- **Green**
  实现 `cmd_learn`
- **Refactor**
  learn 入口与 Store 分离

### F02 自动学习 obey exclude list

- **Red**
  编写测试 `test_learn_excludes_node_modules`
  编写测试 `test_learn_excludes_custom_dir`
  编写测试 `test_learn_excludes_tmp_dirs`
  编写测试 `test_learn_allows_non_excluded_dirs`
- **Green**
  实现 `is_excluded()`
- **Refactor**
  glob 预编译

### F03 自动学习 obey global switch

- **Red**
  编写测试 `test_learn_disabled_by_config`
- **Green**
  入口检查 `autoLearn.enabled`
- **Refactor**
  提取 `auto_learn_enabled()`

### F04 PowerShell history 解析

- **Red**
  编写测试 `test_parse_powershell_history_extracts_cd_paths`
- **Green**
  实现 PowerShell 解析器
- **Refactor**
  与其他 shell 统一入口

### F05 Bash history 解析

- **Red**
  编写测试 `test_parse_bash_history_extracts_cd_paths`
- **Green**
  实现 Bash 解析器
- **Refactor**
  与其他 shell 统一入口

### F06 冷启动预填充分值

- **Red**
  编写测试 `test_cold_start_prefill_score_is_30_percent`
- **Green**
  实现 30% seed
- **Refactor**
  提取 seed 策略函数

### F07 通用导入流水线

- **Red**
  编写测试 `test_import_pipeline_normalizes_paths`
  编写测试 `test_import_pipeline_deduplicates`
  编写测试 `test_import_pipeline_marks_source_imported`
  编写测试 `test_import_pipeline_does_not_override_explicit`
- **Green**
  实现统一导入 pipeline
- **Refactor**
  步骤串联拆成函数

### F08 score 归一化

- **Red**
  编写测试 `test_score_normalization_maps_to_1_100`
  编写测试 `test_score_normalization_preserves_relative_order`
- **Green**
  实现 `normalize_score()`
- **Refactor**
  批量 min/max 扫描独立化

### F09 autojump 解析

- **Red**
  编写测试 `test_parse_autojump_database`
  编写测试 `test_parse_autojump_skips_malformed_lines`
- **Green**
  实现 `parse_autojump()`
- **Refactor**
  行解析函数独立

### F10 `z` 家族数据库解析

- **Red**
  编写测试 `test_parse_z_database`
  编写测试 `test_parse_z_time_field_used_for_last_visited`
- **Green**
  实现 `parse_z()`
- **Refactor**
  `z.lua / zsh-z` 共用

### F11 fasd 仅目录导入

- **Red**
  编写测试 `test_parse_fasd_only_dirs`
  编写测试 `test_parse_fasd_files_discarded`
- **Green**
  实现 `parse_fasd()`
- **Refactor**
  提取类型判断

### F12 zoxide 输出解析

- **Red**
  编写测试 `test_parse_zoxide_query_output`
  编写测试 `test_parse_zoxide_query_output_with_spaces_in_path`
  编写测试 `test_zoxide_imported_entry_has_null_visit_count`
- **Green**
  实现 `parse_zoxide_output()`
- **Refactor**
  optional 字段统一处理

### F13 PowerShell init 生成

- **Red**
  编写测试 `test_ps_init_contains_wrapper_functions`
  编写测试 `test_ps_init_contains_hook_registration`
  编写测试 `test_ps_init_uses_start_process_not_start_job`
  编写测试 `test_ps_init_contains_completion_registration`
  编写测试 `test_ps_init_cd_alias_is_commented`
- **Green**
  实现 `generate_init(Shell::PowerShell, ...)`
- **Refactor**
  使用模板渲染

### F14 `__BM_CD__` 协议

- **Red**
  编写测试 `test_z_command_outputs_bm_cd_prefix`
  编写测试 `test_z_command_no_results_outputs_nothing`
- **Green**
  统一输出协议
- **Refactor**
  提取 `BM_CD_PREFIX`

### F15 `--cmd` 前缀输出

- **Red**
  编写测试 `test_ps_init_with_cmd_prefix_j`
- **Green**
  init 输出支持 `--cmd`
- **Refactor**
  `InitOptions`

### F16 Phase 2 提前输出 `zi/oi` wrapper

- **Red**
  编写测试 `test_ps_init_zi_wrapper_present_even_in_phase2`
- **Green**
  init 模板无条件输出四个 wrapper
- **Refactor**
  wrapper 与功能实现解耦

### F17 Bash init 不递归

- **Red**
  编写测试 `test_bash_init_does_not_eval_generate_itself`
- **Green**
  输出最终脚本而不是自调用生成器
- **Refactor**
  多 shell 模板走统一渲染入口

### F18 Bash root completion

- **Red**
  编写测试 `test_bash_root_completion_only_lists_subcommands`
- **Green**
  `bm` 只补子命令
- **Refactor**
  子命令集合统一来源

### F19 Bash query completion

- **Red**
  编写测试 `test_bash_query_completion_only_for_navigation_commands`
- **Green**
  `z/zi/o/oi` 才补候选路径
- **Refactor**
  与 PowerShell 策略对齐

### F20 Completion 顺序与 query core 一致

- **Red**
  编写测试 `test_completion_order_matches_list_order`
  编写测试 `test_completion_respects_scope`
- **Green**
  completion 改为调用 query core
- **Refactor**
  统一 `action = Complete`

---

## Phase G — 治理与性能层

> 目标：最后补治理、benchmark 和历史包袱清理。

### G01 `check` missing

- **Red**
  编写测试 `test_check_finds_missing_paths`
- **Green**
  实现 missing 检测
- **Refactor**
  检测与输出分离

### G02 `check` stale

- **Red**
  编写测试 `test_check_finds_stale_entries`
- **Green**
  实现 stale 检测
- **Refactor**
  stale 阈值可配置

### G03 `check` duplicate

- **Red**
  编写测试 `test_check_finds_duplicate_paths`
- **Green**
  按 `path_norm` 分组
- **Refactor**
  提取 `find_duplicates()`

### G04 `gc --dry-run`

- **Red**
  编写测试 `test_gc_dry_run_lists_but_does_not_delete`
- **Green**
  实现 dry-run
- **Refactor**
  “找出待删项”与“执行删除”分离

### G05 `gc --learned`

- **Red**
  编写测试 `test_gc_learned_only_does_not_touch_explicit`
- **Green**
  仅删除 learned
- **Refactor**
  提取 `should_gc()`

### G06 `desc` 写入

- **Red**
  编写测试 `test_set_with_desc_stores_description`
  编写测试 `test_set_without_desc_stores_empty_string`
- **Green**
  实现 desc 字段写入
- **Refactor**
  空字符串压缩序列化

### G07 `desc` 展示

- **Red**
  编写测试 `test_list_shows_desc_when_present`
  编写测试 `test_list_no_desc_column_when_all_empty`
- **Green**
  list 条件展示 desc
- **Refactor**
  formatter 内按列动态开关

### G08 `recent --tag`

- **Red**
  编写测试 `test_recent_filter_by_tag`
- **Green**
  实现 tag 过滤
- **Refactor**
  通用 predicate

### G09 `recent --workspace`

- **Red**
  编写测试 `test_recent_filter_by_workspace`
- **Green**
  实现 workspace 过滤
- **Refactor**
  通用 predicate

### G10 `recent --since`

- **Red**
  编写测试 `test_recent_filter_by_since`
  编写测试 `test_recent_since_duration_parsing`
- **Green**
  实现 since 过滤
- **Refactor**
  提取 `parse_duration()`

### G11 `cmd_z` benchmark

- **Red**
  编写 benchmark `bench_z_5000_entries`
- **Green**
  达到目标
- **Refactor**
  如超标再做优化

### G12 completion benchmark

- **Red**
  编写 benchmark `bench_completion_5000_entries`
- **Green**
  达到目标
- **Refactor**
  Top-K 优化

### G13 内存占用 benchmark

- **Red**
  编写测试 `test_memory_under_20mb_for_5000_entries`
- **Green**
  达到目标
- **Refactor**
  如超标再考虑惰性加载

### G14 `bm fuzzy` 从 help 移除

- **Red**
  编写测试 `test_fuzzy_subcommand_absent_from_help`
- **Green**
  从 help 移除
- **Refactor**
  文档同步清理

### G15 `bm fuzzy` 调用报错

- **Red**
  编写测试 `test_fuzzy_invocation_returns_error`
- **Green**
  parser 中彻底删除
- **Refactor**
  清理残余代码路径

---

## 附录：约束

### A. 测试辅助工具

```rust
pub fn build_store(n: usize) -> Store { ... }
pub fn make_bookmark(name: Option<&str>, path: &str, source: Source) -> Bookmark { ... }
pub fn make_context(cwd: &str) -> QueryContext { ... }
```

### B. Mock 约束

- 文件系统：优先 `tempfile`
- 进程调用：trait + mock struct
- 时间：注入时间源，不直接在核心逻辑里调用 `SystemTime::now()`

### C. CI 门槛

- 单元测试通过率 100%
- 核心模块覆盖率 > 90%
- benchmark 回归超过基线 +20% 触发失败
