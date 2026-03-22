# XunYu 测试清单

> 基于源码分析的完整测试清单，每项测试均标注源码依据。
> 标记说明：✅ 已有测试 | ⬚ 待补充

---

## 1. 书签存储 (`src/store.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 1.1 | `load()` 加载合法 JSON 数据库并反序列化为 `BTreeMap<String, Entry>` | `store.rs` `load()` 返回 `BTreeMap` | ✅ `store::tests::load_valid_json_parses_entries` |
| 1.2 | `load()` 文件不存在时返回空 Map | `store.rs` `fs::read_to_string` 失败路径 | ✅ `store::tests::load_missing_returns_empty_map` |
| 1.3 | `load()` 文件内容为非法 JSON 时返回空 Map | `store.rs` `serde_json::from_str` 失败路径 | ✅ `store::tests::load_invalid_json_returns_empty_map` |
| 1.4 | `save_db()` 原子写入（先写临时文件再 rename） | `store.rs` 使用 `.tmp` 后缀 + `fs::rename` | ✅ `store::tests::save_db_roundtrip_and_tmp_is_removed` |
| 1.5 | `save_db()` 写入后可被 `load()` 正确读回（roundtrip） | `store.rs` + `load()` | ✅ `store::tests::save_db_roundtrip_and_tmp_is_removed` |
| 1.6 | `Lock::acquire()` 成功获取文件锁 | `store.rs` 3 秒超时循环 | ✅ `store::tests::lock_acquire_allows_reacquire_after_drop` |
| 1.7 | `Lock::acquire()` 超时返回错误 | `store.rs` 超过 3 秒后 `Err` | ✅ `store::tests::lock_acquire_times_out_when_held_by_another_handle` |
| 1.8 | `Lock` drop 时释放文件锁（允许重新 acquire；lock 文件可能保留） | `store.rs` 文件句柄 drop | ✅ `store::tests::lock_acquire_allows_reacquire_after_drop` |
| 1.9 | `db_path()` 优先使用 `XUN_DB` 环境变量 | `store.rs` `db_path_from_env()` | ✅ `store::tests::db_path_prefers_xun_db_env` |
| 1.10 | `db_path()` 默认回退到 `USERPROFILE/.xun.json` | `store.rs` `db_path_from_env()` | ✅ `store::tests::db_path_falls_back_to_userprofile` |
| 1.11 | `now_secs()` 返回合理的 Unix 时间戳 | `store.rs` `SystemTime::now()` | ✅ `store::tests::now_secs_is_reasonable` |
| 1.12 | 访问日志：`load()` 会合并 visits.jsonl 到 visit_count/last_visited | `store.rs` `apply_visit_log()` | ✅ `store::tests::load_applies_visit_log_lines` |
| 1.13 | 访问日志：`save_db()` 成功后会清理 visits.jsonl | `store.rs` `save_db()` remove log | ✅ `store::tests::save_db_clears_visit_log_after_successful_save` |

## 2. 模糊匹配与 Frecency (`src/fuzzy.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 2.1 | `fuzzy_score()` 精确匹配得分最高 | `fuzzy.rs` 评分算法 | ✅ `fuzzy::tests::fuzzy_score_exact_and_consecutive_get_bonus` |
| 2.2 | `fuzzy_score()` 连续字符匹配有加分 | `fuzzy.rs` 连续匹配 bonus | ✅ `fuzzy::tests::fuzzy_score_exact_and_consecutive_get_bonus` |
| 2.3 | `fuzzy_score()` 不匹配返回 `None` | `fuzzy.rs` 无匹配字符时 | ✅ `fuzzy::tests::fuzzy_score_no_match_is_none` |
| 2.4 | `fuzzy_score()` 调用者传入 lower 后大小写不敏感（`pattern_chars`/`text_lower`） | `commands/bookmarks/list.rs:335-342` `to_lowercase()` | ✅ `fuzzy::tests::fuzzy_score_case_insensitive_when_lowercased` |
| 2.5 | `frecency()` 高访问量 + 近期访问得分高 | `fuzzy.rs` 时间衰减 + 访问计数 | ✅ `fuzzy::tests::frecency_prefers_recent_and_counts_visits` |
| 2.6 | `frecency()` 零访问量按 1 计算（`visit_count.max(1)`） | `fuzzy.rs` `max(1)` 分支 | ✅ `fuzzy::tests::frecency_treats_zero_visits_as_one` |
| 2.7 | `frecency()` 时间衰减：40 天前的访问得分低于今天 | `fuzzy.rs` 时间衰减系数 | ✅ `fuzzy::tests::frecency_prefers_recent_and_counts_visits` |
| 2.8 | `matches_tag()` 精确匹配标签 | `fuzzy.rs` 标签过滤 | ✅ `fuzzy::tests::matches_tag_is_case_insensitive_and_rejects_empty_tag` |
| 2.9 | `matches_tag()` 大小写不敏感 | `fuzzy.rs` `eq_ignore_ascii_case` | ✅ `fuzzy::tests::matches_tag_is_case_insensitive_and_rejects_empty_tag` |
| 2.10 | `fuzzy_score()` 空 pattern 视为匹配并返回得分 | `fuzzy.rs` 空 pattern 分支 | ✅ `fuzzy::tests::fuzzy_score_empty_pattern_matches` |

## 3. 数据模型 (`src/model.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 3.1 | `Entry` 序列化/反序列化 roundtrip | `model.rs` `Serialize, Deserialize` derive | ✅ `model::tests::entry_serde_roundtrip` |
| 3.2 | `Entry` 默认 `visit_count=0`, `last_visited=0` | `model.rs` 字段默认值 | ✅ `model::tests::entry_defaults_are_zero` |
| 3.3 | `parse_list_format()` 解析 "auto/table/tsv/json" | `model.rs` 格式解析 | ✅ `model::tests::parse_list_format_accepts_known_and_rejects_unknown` |
| 3.4 | `parse_list_format()` 无效格式返回 None | `model.rs` 默认分支 | ✅ `model::tests::parse_list_format_accepts_known_and_rejects_unknown` |
| 3.5 | `DedupMode` 解析 "path" 和 "name" | `model.rs` | ✅ `model::tests::parse_dedup_mode_accepts_known_and_rejects_unknown` |
| 3.6 | `parse_io_format()` 解析 "json/tsv" | `model.rs` | ✅ `model::tests::parse_io_format_accepts_known_and_rejects_unknown` |
| 3.7 | `parse_import_mode()` 解析 "merge/overwrite" | `model.rs` | ✅ `model::tests::parse_import_mode_accepts_known_and_rejects_unknown` |

## 4. 输出格式化 (`src/output.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 4.1 | `format_age()` 秒级显示 "Xs" | `output.rs` 时间格式化 | ✅ `output::tests::format_age_formats_seconds_minutes_hours_days` |
| 4.2 | `format_age()` 分钟级显示 "Xm" | `output.rs` 分钟分支 | ✅ `output::tests::format_age_formats_seconds_minutes_hours_days` |
| 4.3 | `format_age()` 天级显示 "Xd" | `output.rs` 天分支 | ✅ `output::tests::format_age_formats_seconds_minutes_hours_days` |
| 4.4 | `prefer_table_output()` 在 `XUN_UI=1/true/yes` 时返回 true | `output.rs` `force_ui_value()` | ✅ `output::tests::prefer_table_output_can_be_forced` |
| 4.5 | `can_interact()` 在 non_interactive 时返回 false（短路，无需依赖 TTY） | `output.rs` `can_interact_with()` | ✅ `output::tests::can_interact_is_false_when_non_interactive` |
| 4.6 | `format_age()` 小时级显示 "Xh" | `output.rs` 小时分支 | ✅ `output::tests::format_age_formats_seconds_minutes_hours_days` |
| 4.7 | `format_age(0)` 返回 "never" | `output.rs` `ts == 0` | ✅ `output::tests::format_age_zero_is_never` |

## 5. 配置管理 (`src/config.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 5.1 | `load_config()` 文件不存在返回默认配置 | `config.rs` fallback 到 `Default` | ✅ `config::tests::load_config_missing_returns_default` |
| 5.2 | `load_config()` 合法 JSON 正确解析 | `config.rs` `serde_json::from_str` | ✅ `config::tests::load_config_valid_json_is_parsed` |
| 5.3 | `save_config()` 写入后可被 `load_config()` 读回 | `config.rs` roundtrip（feature=protect） | ✅ `config::tests::save_config_roundtrip` |
| 5.4 | `config_path()` 优先使用 `XUN_CONFIG` 环境变量 | `config.rs` `config_path_from_env()` | ✅ `config::tests::config_path_prefers_xun_config_env` |
| 5.5 | `config_path()` 默认回退到 `USERPROFILE` | `config.rs` `config_path_from_env()` | ✅ `config::tests::config_path_falls_back_to_userprofile` |
| 5.6 | `TreeConfig` 默认 `default_depth=None`, `exclude_names` 为空 | `config.rs` Default derive | ✅ `config::tests::tree_config_and_proxy_config_defaults` |
| 5.7 | `ProxyConfig` 默认 `default_url=None` | `config.rs` | ✅ `config::tests::tree_config_and_proxy_config_defaults` |
| 5.8 | `xun config set/get` 支持点路径读写 | `commands/app_config.rs` `cmd_set/cmd_get` | ✅ `test_basic::config_set_and_get_roundtrip` |

## 6. 工具函数 (`src/util.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 6.1 | `split_csv()` 逗号分隔正确拆分 | `util.rs` `split_csv` 函数 | ✅ `util::tests::split_csv_splits_and_trims` |
| 6.2 | `split_csv()` 空字符串返回空 Vec | `util.rs` 空输入处理 | ✅ `util::tests::split_csv_empty_returns_empty_vec` |
| 6.3 | `normalize_glob_path()` 反斜杠转正斜杠并小写 | `util.rs` 路径规范化 | ✅ `util::tests::normalize_glob_path_normalizes_slashes_and_case_and_prefix` |
| 6.4 | `matches_patterns()` glob 模式匹配文件 | `util.rs` 模式匹配逻辑 | ✅ `util::tests::matches_patterns_matches_file_globs` |
| 6.5 | `matches_patterns()` 目录尾部 `/` 匹配 | `util.rs` 目录模式 | ✅ `util::tests::matches_patterns_directory_suffix_matches_directories` |
| 6.6 | `read_ignore_file()` 解析 `.xunignore` 的 include/exclude | `util.rs` ignore 文件解析 | ✅ `util::tests::read_ignore_file_parses_include_and_exclude` |
| 6.7 | `read_ignore_file()` 文件不存在返回空规则 | `util.rs` 文件缺失处理 | ✅ `util::tests::read_ignore_file_missing_returns_empty` |
| 6.8 | `has_cmd()` 检测系统命令是否存在 | `util.rs` 命令检测 | ✅ `util::tests::has_cmd_detects_existing_and_missing_commands` |

## 7. 书签 CRUD 命令 (`src/commands/bookmarks/`)

### 7.1 set / sv / del (`mutate.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 7.1.1 | `set` 保存指定路径的书签 | `mutate.rs:10-45` `cmd_set()` 写入 db | ✅ `test_basic::set_warns_on_missing_path` |
| 7.1.2 | `set` 路径不存在时输出 Warning 但仍保存 | `mutate.rs:25-28` `!path.exists()` 分支 | ✅ `test_basic::set_warns_on_missing_path` |
| 7.1.3 | `set` 带 `-t` 参数同时设置标签 | `mutate.rs:35-40` `split_csv(&args.tags)` | ✅ `test_basic::list_json_contains_tags` |
| 7.1.4 | `sv` 无参数时使用当前目录名作为 key | `mutate.rs:48-65` `cmd_sv()` 取 `current_dir` 的 `file_name` | ✅ `test_basic::sv_defaults_to_dir_name` |
| 7.1.5 | `del` 删除已有书签 | `mutate.rs:70-85` `cmd_del()` 从 db 移除 | ✅ `test_basic::del_deletes_existing_bookmark` |
| 7.1.6 | `del` 删除不存在的 key 时提示 not found 并返回成功 | `mutate.rs:75-78` key 不存在分支 | ✅ `test_basic::del_missing_reports_not_found_and_does_not_error` |
| 7.1.7 | `rename` 重命名书签 key | `mutate.rs:90-115` `cmd_rename()` | ✅ `test_basic::rename_changes_key` |
| 7.1.8 | `rename` 目标 key 已存在时报错 | `mutate.rs:100-105` 冲突检测 | ✅ `test_basic::rename_to_existing_fails` |
| 7.1.9 | `touch` 递增 `visit_count` 并更新 `last_visited` | `mutate.rs:120-135` `cmd_touch()` | ✅ `test_basic::touch_increments_visits` |

### 7.2 list / all / fuzzy / keys (`list.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 7.2.1 | `list --format tsv` 输出 5 列 TSV | `list.rs:15-50` TSV 格式化 | ✅ `test_basic::list_tsv_has_fields` |
| 7.2.2 | `list --format json` 输出合法 JSON 数组 | `list.rs:55-80` JSON 格式化 | ✅ `test_basic::list_json_contains_tags` |
| 7.2.3 | `list` 无 format 参数时默认 TSV（非 TTY） | `list.rs:10` `Auto` → `Tsv` | ✅ `test_basic::list_auto_outputs_tsv` |
| 7.2.4 | `list --format nope` 无效格式退出码非 0 | `list.rs:12` `parse_list_format` 失败 | ✅ `test_basic::invalid_format_fails` |
| 7.2.5 | `list -t tag` 按标签过滤 | `list.rs:30-35` `matches_tag` 过滤 | ✅ `test_basic::list_tag_filters_results` |
| 7.2.6 | `list -s visits` 按 visits 降序排序 | `list.rs:20-27` visits 排序分支 | ✅ `test_basic::list_sort_visits_descending` |
| 7.2.7 | `all` 按标签过滤输出 TSV | `list.rs:85-100` `cmd_all()` | ✅ `test_basic::all_and_fuzzy_work` |
| 7.2.8 | `fuzzy` 模糊搜索输出匹配结果 | `list.rs:105-125` `cmd_fuzzy()` | ✅ `test_basic::all_and_fuzzy_work` |
| 7.2.9 | `keys` 输出所有书签名（用于 shell 补全） | `list.rs:130-140` `cmd_keys()` | ✅ `test_basic::keys_outputs_all_bookmark_names` |
| 7.2.10 | `recent` 按最近访问排序输出 | `list.rs:145-165` `cmd_recent()` | ✅ `test_basic::recent_outputs_tsv` |
| 7.2.11 | `stats` 输出统计信息 | `list.rs:170-195` `cmd_stats()` | ✅ `test_basic::stats_outputs_tsv` |

### 7.3 标签管理 (`tags.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 7.3.1 | `tag add` 为书签添加标签 | `tags.rs:10-30` `cmd_tag_add()` | ✅ `test_basic::tag_add_remove_rename_list` |
| 7.3.2 | `tag add` 重复标签不会重复添加 | `tags.rs:20` 去重逻辑 | ✅ `test_basic::tag_add_does_not_duplicate_existing_tags` |
| 7.3.3 | `tag remove` 移除指定标签 | `tags.rs:35-50` `cmd_tag_remove()` | ✅ `test_basic::tag_add_remove_rename_list` |
| 7.3.4 | `tag rename` 全局重命名标签 | `tags.rs:55-75` `cmd_tag_rename()` | ✅ `test_basic::tag_add_remove_rename_list` |
| 7.3.5 | `tag list` 输出所有标签及计数 | `tags.rs:80-100` `cmd_tag_list()` | ✅ `test_basic::tag_add_remove_rename_list` |

### 7.4 导航 (`navigation.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 7.4.1 | `z` 精确匹配输出 `__CD__:path` 魔法命令 | `navigation.rs:10-35` `cmd_z()` | ✅ `test_basic::z_outputs_cd_magic` |
| 7.4.2 | `z` 模糊匹配选择最高分结果 | `navigation.rs:25-30` fuzzy 排序 | ✅ `test_basic::z_fuzzy_selects_highest_scored_match_in_non_interactive` |
| 7.4.3 | `z` 无匹配时报错退出 | `navigation.rs:32` 空结果分支 | ✅ `test_basic::z_no_match_prints_message_and_exits_success` |
| 7.4.4 | `o` 调用 Explorer 打开路径 | `navigation.rs:40-55` `cmd_open()` | ✅ `commands::bookmarks::navigation::tests::open_in_explorer_spec_uses_cmd_start_for_files_and_explorer_for_dirs` |
| 7.4.5 | `ws` 打开标签下所有路径到 Windows Terminal | `navigation.rs:60-90` `cmd_ws()` | ✅ `commands::bookmarks::navigation::tests::wt_new_tab_args_matches_expected_shape` |

### 7.5 维护 (`maintenance.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 7.5.1 | `gc --purge` 删除路径不存在的书签 | `maintenance.rs:10-40` `cmd_gc()` | ✅ `test_basic::gc_purge_removes_missing` |
| 7.5.2 | `gc` 不带 `--purge` 仅报告不删除 | `maintenance.rs:25` 非 purge 分支 | ✅ `test_basic::gc_without_purge_only_reports` |
| 7.5.3 | `dedup` 按路径检测重复 | `maintenance.rs:45-70` `cmd_dedup()` | ✅ `test_basic::dedup_reports_duplicates` |
| 7.5.4 | `dedup --mode name` 按名称检测重复（大小写不敏感） | `maintenance.rs:55` `DedupMode::Name` | ✅ `test_basic::dedup_mode_name_detects_case_insensitive_duplicates` |
| 7.5.5 | `check --format json` 输出 missing/stale/duplicate | `maintenance.rs:17-120` `cmd_check()` | ✅ `test_basic::check_outputs_missing_stale_and_duplicate_in_json` |

### 7.6 导入导出 (`io.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 7.6.1 | `export --format json` 输出合法 JSON | `io.rs:10-35` `cmd_export()` JSON 分支 | ✅ `test_basic::export_import_json_roundtrip` |
| 7.6.2 | `export --format tsv` 输出 5 列 TSV | `io.rs:40-55` TSV 分支 | ✅ `test_basic::export_tsv_has_fields` |
| 7.6.3 | `export --out file` 写入文件 | `io.rs:30` 文件输出 | ✅ `test_basic::export_import_json_roundtrip` |
| 7.6.4 | `import --format json` 导入 JSON 数据 | `io.rs:60-90` `cmd_import()` | ✅ `test_basic::export_import_json_roundtrip` |
| 7.6.5 | `import --format tsv` 导入 TSV 数据 | `io.rs:95-120` TSV 导入 | ✅ `test_basic::import_tsv_works` |
| 7.6.6 | `import --mode overwrite` 无 `--yes` 时冲突报错 | `io.rs:105-110` 冲突检测 | ✅ `test_basic::import_overwrite_requires_yes` |
| 7.6.7 | `import --mode merge` 合并 tags 且 visits/last_visited 取 max；path 仅在非空时覆盖 | `io.rs:138-172` merge 逻辑 | ✅ `test_basic::import_merge_merges_without_overwriting_when_path_is_empty` |

## 8. 目录树 (`src/commands/tree.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 8.1 | `tree` 输出包含子目录和文件 | `tree.rs:222-296` `build_tree_inner()` 递归遍历 | ✅ `test_basic::tree_no_clip_outputs` |
| 8.2 | `tree -d N` 限制递归深度 | `tree.rs:236-238` `depth > max_depth` 截断 | ✅ `test_basic::tree_no_clip_outputs` |
| 8.3 | `tree -o file` 输出到文件，内容与 stdout 一致 | `tree.rs:481-491` 文件写入逻辑 | ✅ `test_basic::tree_output_file_matches_stdout` |
| 8.4 | `tree --plain` 无 Unicode 树形符号 | `tree.rs:254-256` `plain` 分支使用空字符串 | ✅ `test_basic::tree_plain_has_no_unicode_box_drawing_chars` |
| 8.5 | `tree --max-items N` 限制输出条目数 | `tree.rs:239-243` + `248-252` `max_items` 检查 | ✅ `test_basic::tree_plain_max_items_limits_output` |
| 8.6 | `.xunignore` 排除指定目录 | `tree.rs:412-414` `read_ignore_file` + `exclude_patterns` | ✅ `test_basic::tree_xunignore_excludes_entries` |
| 8.7 | `tree --sort mtime` 按修改时间排序 | `tree.rs:208` `SortKey::Mtime` 排序分支 | ✅ `commands::tree::tests::collect_items_sorts_by_mtime_descending` |
| 8.8 | `tree --sort size` 按文件大小排序 | `tree.rs:209` `SortKey::Size` 排序分支 | ✅ `commands::tree::tests::collect_items_sorts_by_size_descending` |
| 8.9 | `tree --sort nope` 无效排序参数退出码 2 | `tree.rs:436-439` `parse_sort` 失败 | ✅ `test_basic::tree_invalid_sort_fails` |
| 8.10 | `tree` 路径不存在时报错退出 | `tree.rs:394-397` `!root.is_dir()` | ✅ `test_basic::tree_invalid_path_fails` |
| 8.11 | `is_version_dir()` 排除 `v1.x` 等版本目录 | `tree.rs:66-73` 以 `v` + 数字开头的目录 | ✅ `commands::tree::tests::is_version_dir_detects_v_number_prefix` |
| 8.12 | `should_exclude()` 隐藏文件默认排除 | `tree.rs:97-98` `.` 开头且 `!hidden` | ✅ `commands::tree::tests::should_exclude_hides_dotfiles_by_default_and_allows_when_hidden_enabled` |
| 8.13 | `should_exclude()` 排除 `.dll/.exe/.obj` 等扩展名 | `tree.rs:110-120` `exclude_exts` 匹配 | ✅ `commands::tree::tests::should_exclude_filters_version_dirs_and_excluded_exts` |
| 8.14 | `should_exclude()` include 模式优先于 exclude | `tree.rs:123-127` include 匹配时返回 false | ✅ `commands::tree::tests::should_exclude_include_patterns_override_exclude_patterns` |
| 8.15 | `tree --stats-only` 仅输出统计不输出树 | `tree.rs:441-448` `stats_only` 分支 | ✅ `test_basic::tree_stats_only_outputs_stats_without_tree_lines` |
| 8.16 | `tree --hidden` 显示隐藏文件 | `tree.rs:97` `filters.hidden` 跳过排除 | ✅ `commands::tree::tests::should_exclude_hides_dotfiles_by_default_and_allows_when_hidden_enabled` |
| 8.17 | `tree --exclude pattern` 排除指定模式 | `tree.rs:416-420` CLI `--exclude` 参数 | ✅ `test_basic::tree_exclude_pattern_filters_entries` |
| 8.18 | `tree --include pattern` 仅包含指定模式 | `tree.rs:421-425` CLI `--include` 参数 | ✅ `test_basic::tree_include_pattern_overrides_exclude_pattern` |
| 8.19 | `tree --size` 输出每项大小（文件/目录） | `tree.rs:346-381` size 逻辑 | ✅ `test_basic::tree_size_outputs_human_readable_sizes` |

## 9. 增量备份 (`src/commands/backup.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 9.1 | `backup` 创建备份目录和版本文件夹 | `backup.rs` `cmd_backup()` 创建 `A_backups/vN-*` | ✅ `test_basic::backup_creates_backup_folder` |
| 9.2 | `bak --dry-run` 不创建任何版本 | `backup.rs` `dry_run` 分支跳过写入 | ✅ `test_basic::bak_dry_run_creates_no_version` |
| 9.3 | `bak` 增量检测：新增文件标记 `+` | `backup.rs` 增量比较逻辑 | ✅ `test_basic::bak_incremental_reports_new_file` |
| 9.4 | `bak` retention 策略删除旧版本 | `backup.rs` `maxBackups` + `deleteCount` 逻辑 | ✅ `test_basic::bak_retention_removes_old_versions` |
| 9.5 | `bak` 使用 `.gitignore` 排除文件 | `backup.rs` `useGitignore` 配置项 | ✅ `test_basic::bak_gitignore_excludes_file` |
| 9.6 | `backup` 无 `.xun-bak.json` 时自动创建默认配置 | `backup.rs` 配置文件缺失处理 | ✅ `test_basic::bak_missing_config_auto_creates_default_config` |
| 9.7 | `bak` 压缩模式 (`compress: true`) 生成 zip | `backup.rs` zip 压缩分支 | ✅ `test_basic::bak_compress_true_creates_zip` |
| 9.8 | `bak` 增量检测：修改文件标记 `~` | `backup.rs` 文件内容/时间变更检测 | ✅ `test_basic::bak_incremental_reports_modified_file_with_tilde` |
| 9.9 | `bak` 增量检测：删除文件标记 `-` | `backup.rs` 旧版本有但新版本无的文件 | ✅ `test_basic::bak_incremental_reports_deleted_file_with_minus` |
| 9.10 | `bak` 版本号自增（v1, v2, v3...） | `backup.rs` 版本号解析和递增 | ✅ `test_basic::bak_version_increments_v1_v2` |
| 9.11 | `backup list` 输出现有备份列表（`bak list` 为别名） | `backup/list.rs:13` `cmd_backup_list()` | ✅ `test_basic::bak_list_shows_human_readable_mtime` |
| 9.12 | `backup list` 在空目录下提示无备份 | `backup/list.rs` 空列表分支 | ✅ `test_basic::backup_list_empty_reports_no_backups` |
| 9.13 | `bak verify` 对 zip 备份返回不支持错误 | `backup/verify.rs` zip 分支 | ✅ `test_basic::bak_verify_zip_backup_reports_not_supported` |
| 9.14 | `bak find <tag>` 按标签过滤备份 | `backup/find.rs` tag 过滤 | ✅ `test_basic::bak_find_filters_backups_by_tag` |
| 9.15 | retention `keepWeekly` 保留最近周代表 | `backup/retention.rs` weekly 保留分支 | ✅ `commands::backup::retention::tests::keep_weekly_preserves_recent_week_representatives` |
| 9.16 | retention `keepMonthly` 保留最近月代表 | `backup/retention.rs` monthly 保留分支 | ✅ `commands::backup::retention::tests::keep_monthly_preserves_recent_month_representatives` |
| 9.17 | manifest 校验支持成功与损坏分支（feature: `bak`） | `backup/checksum.rs` `verify_manifest()` | ✅ `commands::backup::checksum::tests::verify_manifest_roundtrip_ok` / `verify_manifest_detects_corrupted_files` |
| 9.18 | `restore_core` 统计部分恢复失败数量 | `restore_core.rs` `restore_many_from_dir()` | ✅ `commands::restore_core::tests::restore_many_from_dir_counts_partial_failures` |
| 9.19 | `backup --skip-if-unchanged` 在无变化时跳过新版本 | `backup.rs` no-change skip 分支 | ✅ `test_basic::backup_skip_if_unchanged_skips_new_version` |
| 9.20 | `backup --skip-if-unchanged` 在有变化时仍创建新版本 | `backup.rs` no-change skip 分支 | ✅ `test_basic::backup_skip_if_unchanged_still_creates_version_when_changed` |
| 9.21 | `restore --file` 可恢复单个文件 | `restore.rs:15` `cmd_restore()` / `restore.rs:125` `restore_single_file()` | ✅ `test_basic::restore::restore_cmd_file_from_dir_backup` / `test_basic::restore::restore_cmd_file_from_zip_backup` |

## 10. 端口管理 (`src/ports.rs` + `src/commands/ports.rs`)

### 10.1 底层端口枚举 (`src/ports.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 10.1.1 | `port_from_be()` 大端转小端正确 | `ports.rs:33-35` `u16::from_be` | ✅ `ports::tests::port_from_be_swaps_endianness` |
| 10.1.2 | `list_tcp_listeners()` 返回去重的 TCP 监听端口 | `ports.rs:239-246` IPv4+IPv6 合并 + `HashSet` 去重 | ✅ `ports::tests::list_tcp_listeners_contains_self_bound_port_and_is_deduped` |
| 10.1.3 | `list_udp_endpoints()` 返回去重的 UDP 端口 | `ports.rs:248-255` 同上 UDP 版本 | ✅ `ports::tests::list_udp_endpoints_contains_self_bound_port_and_is_deduped` |
| 10.1.4 | `process_name_from_path()` 提取文件名 | `ports.rs:56-65` `Path::file_name()` | ✅ `ports::tests::process_name_from_path_extracts_basename_or_falls_back_to_pid` |
| 10.1.5 | `process_name_from_path()` 空路径返回 `pid N` | `ports.rs:57-59` 空路径分支 | ✅ `ports::tests::process_name_from_path_extracts_basename_or_falls_back_to_pid` |
| 10.1.6 | `terminate_pid()` 无效 PID 返回错误 | `ports.rs:257-278` `OpenProcess` 失败分支 | ✅ `ports::tests::terminate_pid_invalid_pid_returns_error` |
| 10.1.7 | `terminate_pid()` 权限不足返回 "access denied" | `ports.rs:262` error code 5 | ✅ `ports::tests::terminate_pid_error_message_maps_known_win32_codes` |

### 10.2 端口命令 (`src/commands/ports.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 10.2.1 | `is_dev_port()` 识别 3000-3999, 5000-5999, 8000-8999, 4173, 5173 | `commands/ports.rs:12-18` 端口范围判断 | ✅ `commands::ports::tests::is_dev_port_matches_expected_ranges` |
| 10.2.2 | `parse_range()` 解析 "3000-4000" 为 (3000, 4000) | `commands/ports.rs:36-48` 范围解析 | ✅ `commands::ports::tests::parse_range_parses_and_normalizes` |
| 10.2.3 | `parse_range()` 反向范围 "4000-3000" 自动纠正 | `commands/ports.rs:44-47` `start > end` 交换 | ✅ `commands::ports::tests::parse_range_parses_and_normalizes` |
| 10.2.4 | `parse_range()` 无效格式返回 None | `commands/ports.rs:40-42` 多段或解析失败 | ✅ `commands::ports::tests::parse_range_parses_and_normalizes` |
| 10.2.5 | `trunc()` 超长字符串截断为 `...suffix` | `commands/ports.rs:20-27` 截断逻辑 | ✅ `commands::ports::tests::trunc_short_strings_are_unchanged_and_long_strings_keep_suffix` |
| 10.2.6 | `ports` 默认仅显示开发端口 | `commands/ports.rs:57-59` `!all` 时 `is_dev_port` 过滤 | ✅ `test_proxy_net::ports_default_filters_to_dev_ports` |
| 10.2.7 | `ports --all` 显示所有端口 | `commands/ports.rs:57` `args.all` 跳过过滤 | ✅ `test_proxy_net::ports_all_includes_tcp_listener` |
| 10.2.8 | `ports --udp` 显示 UDP 端口 | `commands/ports.rs:51-55` `args.udp` 分支 | ✅ `test_proxy_net::ports_udp_includes_socket` |
| 10.2.9 | `ports --range 3000-4000` 范围过滤 | `commands/ports.rs:61-67` range 过滤 | ✅ `test_proxy_net::ports_range_filters_tcp` |
| 10.2.10 | `ports --pid N` 按 PID 过滤 | `commands/ports.rs:68-69` pid 过滤 | ✅ `test_proxy_net::ports_pid_filters_tcp` |
| 10.2.11 | `ports --name str` 按进程名过滤 | `commands/ports.rs:71-74` 名称过滤（大小写不敏感） | ✅ `test_proxy_net::ports_name_filters_case_insensitive` |
| 10.2.12 | `kill` 解析逗号分隔端口列表 | `commands/ports.rs:207-225` 端口解析 | ✅ `test_proxy_net::kill_force_parses_comma_list_and_terminates_process` |
| 10.2.13 | `kill --force` 跳过确认直接终止 | `commands/ports.rs:265-266` `args.force` 分支 | ✅ `test_proxy_net::kill_force_parses_comma_list_and_terminates_process` |
| 10.2.14 | `ports --range` 无效范围退出码 2 | `commands/ports.rs:61-67` `parse_range` 失败 | ✅ `test_proxy_net::ports_invalid_range_fails` |
| 10.2.15 | `kill` 非法端口参数退出码 2 | `commands/ports.rs:207-220` 端口解析失败 | ✅ `test_proxy_net::kill_invalid_port_fails` |

## 11. 文件锁定检测 (`feature: lock`)

### 11.1 Restart Manager (`src/windows/restart_manager.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 11.1.1 | `get_locking_processes()` 空路径返回空 Vec | `restart_manager.rs:203-206` 空输入快速返回 | ✅ `windows::restart_manager::tests::get_locking_processes_empty_paths_returns_empty` |
| 11.1.2 | `get_locking_processes()` 检测自身进程持有的文件锁 | `restart_manager.rs:203-336` RM 完整流程 | ✅ `windows::restart_manager::tests::test_restart_manager_self_lock` |
| 11.1.3 | `LockQueryError::is_registry_unavailable()` 识别 code=29 | `restart_manager.rs:102-104` `ERROR_WRITE_FAULT_CODE` | ✅ `windows::restart_manager::tests::lock_query_error_classification_and_guidance` |
| 11.1.4 | `LockQueryError::is_registry_mutex_timeout()` 识别 code=121 | `restart_manager.rs:106-108` `ERROR_SEM_TIMEOUT_CODE` | ✅ `windows::restart_manager::tests::lock_query_error_classification_and_guidance` |
| 11.1.5 | `LockQueryError::is_directory_path_error()` 识别 RM 阶段的 code=5 | `restart_manager.rs:110-114` RM 阶段 + ACCESS_DENIED | ✅ `windows::restart_manager::tests::lock_query_error_classification_and_guidance` |
| 11.1.6 | `LockQueryError::guidance()` 各错误类型返回正确提示 | `restart_manager.rs:120-136` 5 个分支 | ✅ `windows::restart_manager::tests::lock_query_error_classification_and_guidance` |
| 11.1.7 | `LockQueryError` Display 格式化 Win32 和 NtStatus | `restart_manager.rs:139-158` `fmt::Display` impl | ✅ `windows::restart_manager::tests::lock_query_error_display_formats_win32_and_ntstatus` |
| 11.1.8 | `probe_registry_access()` 注册表不可写时返回错误 | `restart_manager.rs:164-187` HKCU 读写探测 | ✅ `windows::restart_manager::tests::probe_registry_access_reports_error_when_key_is_not_writable` |

### 11.2 Handle Query 引擎 (`src/windows/handle_query.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 11.2.1 | `normalize_path_like()` 正斜杠转反斜杠 | `handle_query.rs:803-825` `replace('/', "\\")` | ✅ `windows::handle_query::tests::normalize_path_like_converts_slashes_and_strips_prefixes` |
| 11.2.2 | `normalize_path_like()` 去除 `\\?\` 前缀 | `handle_query.rs:808-809` `strip_prefix` | ✅ `windows::handle_query::tests::normalize_path_like_converts_slashes_and_strips_prefixes` |
| 11.2.3 | `normalize_path_like()` 去除 `\\?\UNC\` 转为 `\\` | `handle_query.rs:806-807` UNC 前缀处理 | ✅ `windows::handle_query::tests::normalize_path_like_converts_slashes_and_strips_prefixes` |
| 11.2.4 | `normalize_path_like()` 去除 `\??\` 前缀 | `handle_query.rs:810-811` NT 前缀处理 | ✅ `windows::handle_query::tests::normalize_path_like_converts_slashes_and_strips_prefixes` |
| 11.2.5 | `normalize_path_like()` 驱动器号大写 | `handle_query.rs:819-822` `to_ascii_uppercase` | ✅ `windows::handle_query::tests::normalize_path_like_converts_slashes_and_strips_prefixes` |
| 11.2.6 | `normalize_path_like()` 去除尾部反斜杠（保留根路径） | `handle_query.rs:815-818` `while ends_with('\\')` | ✅ `windows::handle_query::tests::normalize_path_like_preserves_drive_root_and_trims_trailing_backslashes` |
| 11.2.7 | `path_eq()` 精确路径比较 | `handle_query.rs:688-690` 字符串相等 | ✅ `windows::handle_query::tests::path_eq_and_is_same_or_child_behave_as_expected` |
| 11.2.8 | `is_same_or_child()` 父路径匹配 | `handle_query.rs:692-697` `strip_prefix` + `\\` 检查 | ✅ `windows::handle_query::tests::path_eq_and_is_same_or_child_behave_as_expected` |
| 11.2.9 | `is_same_or_child()` 精确匹配也返回 true | `handle_query.rs:693` 调用 `path_eq` | ✅ `windows::handle_query::tests::path_eq_and_is_same_or_child_behave_as_expected` |
| 11.2.10 | `strip_prefix_ascii_insensitive()` 大小写不敏感前缀匹配 | `handle_query.rs:835-841` `eq_ignore_ascii_case` | ✅ `windows::handle_query::tests::strip_prefix_ascii_insensitive_is_case_insensitive` |
| 11.2.11 | `dos_to_nt_paths()` 驱动器号映射为 NT 设备路径 | `handle_query.rs:768-783` 设备映射转换 | ✅ `windows::handle_query::tests::dos_to_nt_paths_maps_drive_letters_and_rejects_non_drive` |
| 11.2.12 | `dos_to_nt_paths()` 非驱动器路径返回空 | `handle_query.rs:769-771` 长度/格式检查 | ✅ `windows::handle_query::tests::dos_to_nt_paths_maps_drive_letters_and_rejects_non_drive` |
| 11.2.13 | `nt_to_dos_path()` `\??\C:\...` 转为 `C:\...` | `handle_query.rs:786-788` `\??\` 前缀 | ✅ `windows::handle_query::tests::nt_to_dos_path_converts_prefixes_and_device_map` |
| 11.2.14 | `nt_to_dos_path()` `\Device\Mup\...` 转为 UNC 路径 | `handle_query.rs:789-791` MUP 前缀 | ✅ `windows::handle_query::tests::nt_to_dos_path_converts_prefixes_and_device_map` |
| 11.2.15 | `nt_to_dos_path()` 设备路径通过 device_map 转换 | `handle_query.rs:793-800` 遍历设备映射 | ✅ `windows::handle_query::tests::nt_to_dos_path_converts_prefixes_and_device_map` |
| 11.2.16 | `looks_like_unc_root()` 识别 `\\server\share` | `handle_query.rs:827-833` UNC 根判断 | ✅ `windows::handle_query::tests::looks_like_unc_root_detects_unc_roots` |
| 11.2.17 | `max_handle_buffer_bytes()` 默认 256MB | `handle_query.rs:640-647` `DEFAULT_MAX_HANDLE_BUFFER_BYTES` | ✅ `windows::handle_query::tests::max_handle_buffer_bytes_from_env_defaults_and_clamps` |
| 11.2.18 | `max_handle_buffer_bytes()` 通过 `XUN_MAX_HANDLE_BUFFER_MB` 环境变量覆盖 | `handle_query.rs:641-644` `env::var` | ✅ `windows::handle_query::tests::max_handle_buffer_bytes_from_env_defaults_and_clamps` |
| 11.2.19 | `max_handle_buffer_bytes()` clamp 到 64MB-1024MB | `handle_query.rs:646` `.clamp()` | ✅ `windows::handle_query::tests::max_handle_buffer_bytes_from_env_defaults_and_clamps` |
| 11.2.20 | `infer_app_type()` explorer.exe 返回 RM_EXPLORER(5) | `handle_query.rs:623-625` 进程名匹配 | ✅ `windows::handle_query::tests::infer_app_type_explorer_is_rm_explorer` |
| 11.2.21 | `infer_app_type()` session 0 进程返回 RM_SERVICE(4) | `handle_query.rs:629-631` `session_id == 0` | ✅ `windows::handle_query::tests::infer_app_type_session0_is_rm_service` |
| 11.2.22 | `infer_app_type()` session 查询失败返回 RM_UNKNOWN(0) | `handle_query.rs:633-635` `session_id` 查询失败分支 | ✅ `windows::handle_query::tests::infer_app_type_unknown_when_session_lookup_fails` |

### 11.3 SeDebugPrivilege (`handle_query.rs:237-283`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 11.3.1 | `try_enable_debug_privilege()` 管理员下返回 true | `handle_query.rs:237-283` 完整提权流程 | ✅ `windows::handle_query::tests::try_enable_debug_privilege_returns_true_when_granted` |
| 11.3.2 | `try_enable_debug_privilege()` 非管理员返回 false 且不崩溃 | `handle_query.rs:274` `GetLastError() != 0` | ✅ `windows::handle_query::tests::try_enable_debug_privilege_returns_false_when_not_granted` |
| 11.3.3 | `try_enable_debug_privilege()` verbose 模式输出日志 | `handle_query.rs:241-243, 258-260, 275-280` `is_verbose()` 分支 | ✅ `test_lock_e2e::lock_who_verbose_emits_debug_privilege_log_line` |
| 11.3.4 | `query_with_handles()` 开头调用 `try_enable_debug_privilege()` | `handle_query.rs:357` 函数首行 | ✅ `test_lock_e2e::lock_who_verbose_emits_debug_privilege_log_line` |
| 11.3.5 | `try_enable_debug_privilege()` 打开进程令牌失败返回 false | `handle_query.rs:244-251` `OpenProcessToken` 失败分支 | ✅ `windows::handle_query::tests::try_enable_debug_privilege_returns_false_when_open_process_token_fails` |
| 11.3.6 | `try_enable_debug_privilege()` 查询权限值失败返回 false | `handle_query.rs:252-258` `LookupPrivilegeValueW` 失败 | ✅ `windows::handle_query::tests::try_enable_debug_privilege_returns_false_when_lookup_privilege_value_fails` |

### 11.4 进程模块枚举 (`handle_query.rs:285-324`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 11.4.1 | `enumerate_process_modules()` 对自身进程返回非空模块列表 | `handle_query.rs:285-324` 完整枚举流程 | ✅ `windows::handle_query::tests::enumerate_process_modules_self_returns_non_empty_and_normalized` |
| 11.4.2 | `enumerate_process_modules()` 模块路径经 `normalize_path_like` 规范化 | `handle_query.rs:320` 调用 `normalize_path_like` | ✅ `windows::handle_query::tests::enumerate_process_modules_self_returns_non_empty_and_normalized` |
| 11.4.3 | `enumerate_process_modules()` 无效 PID 返回空 Vec | `handle_query.rs:287-289` `handle.is_null()` | ✅ `windows::handle_query::tests::enumerate_process_modules_invalid_pid_returns_empty` |
| 11.4.4 | `enumerate_process_modules()` 动态扩展模块缓冲区 | `handle_query.rs:306-311` `count > modules.len()` 时 resize | ✅ `windows::handle_query::tests::enumerate_process_modules_resizes_when_needed` |
| 11.4.5 | 模块枚举二次匹配：仅对未匹配 PID 执行 | `handle_query.rs:447-451` `filter(!matched_pids.contains)` | ✅ `windows::handle_query::tests::match_pids_by_modules_only_enumerates_unmatched_pids` |
| 11.4.6 | 模块枚举：文件目标用 `path_eq` 匹配 | `handle_query.rs:459` `path_eq(mod_path, &target.dos_path)` | ✅ `windows::handle_query::tests::modules_match_targets_uses_file_and_dir_rules` |
| 11.4.7 | 模块枚举：目录目标用 `is_same_or_child` 匹配 | `handle_query.rs:457` `is_same_or_child(mod_path, &target.dos_path)` | ✅ `windows::handle_query::tests::modules_match_targets_uses_file_and_dir_rules` |

### 11.5 Lock 命令 (`src/commands/lock.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 11.5.1 | `lock who` 检测持有文件锁的进程（JSON 格式） | `lock.rs:165-234` `cmd_lock_who()` | ✅ `test_lock_e2e::lock_who_detects_holder_and_rm_unlock_force_kill_deletes` |
| 11.5.2 | `lock who` 路径不存在时退出码 2 | `lock.rs:167-170` `!path.exists()` | ✅ `test_dry_run_format::lock_who_missing_path_exits_2` |
| 11.5.3 | `lock who --format tsv` 输出 TSV | `lock.rs:205-209` TSV 分支 | ✅ `test_dry_run_format::lock_who_tsv_outputs_three_columns` |
| 11.5.4 | `lock who` 无锁定进程时输出提示 | `lock.rs:180-185` `lockers.is_empty()` | ✅ `test_dry_run_format::lock_who_no_lockers_outputs_message_in_auto_format` |
| 11.5.5 | `rm --unlock --force-kill` 解锁后删除文件 | `lock.rs:246-336` `cmd_rm()` + `unlock_and_retry_or_exit()` | ✅ `test_lock_e2e::lock_who_detects_holder_and_rm_unlock_force_kill_deletes` |
| 11.5.6 | `rm --unlock` 非交互模式无 `--force-kill` 退出码 10 | `lock.rs:98-114` `ensure_force_kill_authorized()` | ✅ `test_lock_e2e::rm_unlock_without_force_kill_in_non_interactive_fails` |
| 11.5.7 | `rm --dry-run` 不执行实际删除 | `lock.rs:262-265` `dry_run` 分支 | ✅ `test_dry_run_format::rm_dry_run_keeps_target_intact` |
| 11.5.8 | `rm --on-reboot` 调度重启后删除 | `lock.rs:267-285` `schedule_delete_on_reboot` | ✅ `test_lock_e2e::rm_on_reboot_non_admin_fails` |
| 11.5.9 | `mv --unlock` 解锁后移动文件 | `lock.rs:338-425` `do_move()` | ✅ `test_lock_e2e::mv_unlock_force_kill_moves_locked_file` |
| 11.5.10 | `mv --dry-run` 不执行实际移动 | `lock.rs:384-387` `dry_run` 分支 | ✅ `test_dry_run_format::mv_dry_run_does_not_move` |
| 11.5.11 | `CRITICAL_PROCESSES` 列表中的进程显示警告 | `lock.rs:91-96` `print_lockers()` 关键进程检测 | ✅ `commands::lock::tests::critical_process_name_detection_is_case_insensitive` |
| 11.5.12 | `unlock_and_retry_or_exit()` 最多重试 3 次 | `lock.rs:148-158` `for attempt in 0..3` | ✅ `commands::lock::tests::unlock_max_retries_is_three` |
| 11.5.13 | `lock who --format json` 输出字段稳定 | `lock.rs:195-214` JSON 分支输出 | ✅ `test_dry_run_format::lock_who_json_has_stable_fields` |
| 11.5.14 | `ren --dry-run` 不执行实际重命名 | `lock.rs:356-405` `dry_run` 分支 | ✅ `test_dry_run_format::ren_dry_run_does_not_rename` |

### 11.6 重启删除 (`src/windows/reboot_ops.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 11.6.1 | `schedule_delete_on_reboot()` UNC 路径返回 `ERROR_NOT_SUPPORTED` | `reboot_ops.rs:8-9` `is_unc_path` 检查 | ✅ `windows::reboot_ops::tests::schedule_delete_on_reboot_rejects_unc_paths` |
| 11.6.2 | `schedule_delete_on_reboot()` 非管理员返回错误 | `reboot_ops.rs:15-28` `MoveFileExW` 需要管理员权限 | ✅ `test_lock_e2e::rm_on_reboot_non_admin_fails` |
| 11.6.3 | `is_unc_path()` 识别 `\\` 和 `//` 开头的路径 | `reboot_ops.rs:31-34` 双前缀检测 | ✅ `windows::reboot_ops::tests::is_unc_path_detects_both_slash_styles` |

## 12. 代理管理 (`src/commands/proxy/`)

### 12.1 代理配置 (`proxy/config.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 12.1.1 | `parse_proxy_only()` 解析 "cargo,git,npm,msys2" | `proxy/config.rs:11-36` 合法值集合 | ✅ `commands::proxy::config::tests::parse_proxy_only_parses_known_values` |
| 12.1.2 | `parse_proxy_only()` "all" 返回 None（不限制） | `proxy/config.rs:21-23` `"all"` 分支 | ✅ `commands::proxy::config::tests::parse_proxy_only_all_returns_none` |
| 12.1.3 | `parse_proxy_only()` 无效值返回 Err | `proxy/config.rs:28` 默认分支 | ✅ `commands::proxy::config::tests::parse_proxy_only_invalid_returns_err` |
| 12.1.4 | `read_cargo_proxy()` 解析 `config.toml` 中的 `[http] proxy` | `proxy/config.rs:47-71` TOML 行解析 | ✅ `test_proxy_net::pst_reads_cargo_proxy_after_proxy_set_only_cargo` |
| 12.1.5 | `read_cargo_proxy()` 文件不存在返回 None | `proxy/config.rs:49-51` `!path.exists()` | ✅ `test_proxy_net::pst_json_has_four_rows_and_cargo_off_when_no_config` |
| 12.1.6 | `save_proxy_state()` / `load_proxy_state()` roundtrip | `proxy/config.rs:101-115` JSON 序列化 | ✅ `test_proxy_net::proxy_state_is_used_by_pon_when_no_url` |
| 12.1.7 | `set_proxy()` 写入 cargo config.toml | `proxy/config.rs:117-168` cargo 分支 | ✅ `test_proxy_net::proxy_set_only_cargo` |
| 12.1.8 | `set_proxy()` `--only cargo` 仅设置 cargo | `proxy/config.rs:123` `want_only(only, "cargo")` | ✅ `test_proxy_net::proxy_set_only_cargo` |
| 12.1.9 | `del_proxy()` 从 cargo config.toml 删除 proxy 行 | `proxy/config.rs:221-236` 行过滤 | ✅ `test_proxy_net::proxy_set_only_cargo` |
| 12.1.10 | `set_proxy()` 写入代理状态文件 `.xun.proxy.json` | `proxy/config.rs:101-115` `save_proxy_state()` | ✅ `test_proxy_net::proxy_set_persists_state` |
| 12.1.11 | `proxy set --only` 无效值报错 | `proxy/config.rs:11-36` `parse_proxy_only()` 返回 Err | ✅ `test_proxy_net::proxy_only_invalid_fails` |

### 12.2 代理环境变量 (`proxy/env.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 12.2.1 | `out_env_set()` 输出 `__ENV_SET__:key=value` 魔法命令 | `proxy/env.rs:4-6` 格式化输出 | ✅ `test_proxy_net::pon_outputs_env_set_magic_lines` |
| 12.2.2 | `out_env_unset()` 输出 `__ENV_UNSET__:key` 魔法命令 | `proxy/env.rs:8-10` 格式化输出 | ✅ `test_proxy_net::poff_outputs_env_unset_magic_lines` |
| 12.2.3 | `get_system_proxy_url()` 注册表启用时返回代理 URL | `proxy/env.rs:12-39` `ProxyEnable=1` 分支 | ✅ `commands::proxy::env::tests::resolve_proxy_url_enabled_returns_url_and_no_fallback` |
| 12.2.4 | `get_system_proxy_url()` 注册表禁用时返回 fallback | `proxy/env.rs:37-38` `used_fallback=true` | ✅ `commands::proxy::env::tests::resolve_proxy_url_disabled_returns_fallback_and_used_fallback_true` |
| 12.2.5 | `get_system_proxy_url()` 解析 `http=host:port;...` 格式 | `proxy/env.rs:22-29` 多协议代理解析 | ✅ `commands::proxy::env::tests::resolve_proxy_url_multi_protocol_prefers_http_part` |

### 12.3 代理操作命令 (`proxy/ops.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 12.3.1 | `pon` 输出 8 个 `__ENV_SET__` 魔法命令 | `proxy/ops.rs:79-87` 8 个环境变量 | ✅ `test_proxy_net::pon_outputs_env_set_magic_lines` |
| 12.3.2 | `pon` 无 URL 参数时自动检测系统代理 | `proxy/ops.rs:24-42` `resolve_proxy_url_and_noproxy` 优先级链 | ✅ `test_proxy_net::pon_without_url_auto_detects_or_falls_back` |
| 12.3.3 | `pon` URL 无 scheme 时自动补 `http://` | `proxy/ops.rs:43-45` scheme 补全 | ✅ `test_proxy_net::pon_without_scheme_adds_http_prefix` |
| 12.3.4 | `poff` 输出 8 个 `__ENV_UNSET__` 魔法命令 | `proxy/ops.rs:91-105` 8 个环境变量 | ✅ `test_proxy_net::poff_outputs_env_unset_magic_lines` |
| 12.3.5 | `proxy detect --format json` 输出 JSON | `proxy/ops.rs:107-157` JSON 分支 | ✅ `test_proxy_net::proxy_detect_json_outputs_object` |
| 12.3.6 | `proxy detect --format tsv` 输出 TSV | `proxy/ops.rs:129-136` TSV 分支 | ✅ `test_proxy_net::proxy_detect_outputs_status` |
| 12.3.7 | `pst` 聚合 Env/Git/npm/Cargo 代理状态 | `proxy/ops.rs:159-382` `cmd_proxy_status()` | ✅ `test_proxy_net::pst_json_has_four_rows_and_cargo_off_when_no_config` |
| 12.3.8 | `px` 子进程继承代理环境变量 | `proxy/ops.rs:384-416` `cmd.env()` 设置 | ✅ `test_proxy_net::px_inherits_proxy_env_vars` |
| 12.3.9 | `px` 无命令参数时退出码 2 | `proxy/ops.rs:385-388` `args.cmd.is_empty()` | ✅ `test_proxy_net::px_without_command_exits_2` |
| 12.3.10 | `proxy detect --format` 无效格式退出码 2 | `proxy/ops.rs:121-128` `parse_list_format` 失败 | ✅ `test_proxy_net::proxy_detect_invalid_format_fails` |

### 12.4 代理延迟测试 (`proxy/test.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 12.4.1 | `parse_proxy_addr()` 解析 `http://host:port` | `proxy/test.rs:6-13` URL 解析 | ✅ `commands::proxy::test::tests::parse_proxy_addr_parses_http_url` |
| 12.4.2 | `parse_proxy_targets()` 空输入返回默认 3 个目标 | `proxy/test.rs:65-96` `default_proxy_targets()` | ✅ `commands::proxy::test::tests::parse_proxy_targets_empty_returns_default_three_targets` |
| 12.4.3 | `parse_proxy_targets()` 自定义目标无端口时补 `:80` | `proxy/test.rs:80-83` 端口补全 | ✅ `commands::proxy::test::tests::parse_proxy_targets_adds_default_port_80_when_missing` |
| 12.4.4 | `run_proxy_tests_with()` 无效 URL 返回错误 | `proxy/test.rs:104-106` `parse_proxy_addr` 失败 | ✅ `test_proxy_net::proxy_test_invalid_url` |
| 12.4.5 | `run_proxy_tests_with()` 并发执行（jobs 参数） | `proxy/test.rs:112-138` 线程池分批 | ✅ `test_proxy_net::proxy_test_fake_server_ok` |
| 12.4.6 | `proxy test --targets` 仅测试指定目标 | `proxy/test.rs:65-96` 目标列表解析 | ✅ `test_proxy_net::proxy_test_custom_targets` |

## 13. 审计日志 (`src/security/audit.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 13.1 | `audit_log()` 写入 JSONL 格式日志 | `audit.rs:7-48` JSON 序列化 + append 写入 | ✅ `security::audit::tests::audit_log_writes_jsonl_with_expected_fields` |
| 13.2 | `audit_log()` 日志包含 timestamp/action/target/user/result/reason | `audit.rs:20-28` `json!` 字段 | ✅ `security::audit::tests::audit_log_writes_jsonl_with_expected_fields` |
| 13.3 | `audit_log()` 文件超过 10MB 自动轮转为 `.jsonl.1` | `audit.rs:37-43` `meta.len() >= 10MB` 触发 rename | ✅ `security::audit::tests::audit_log_rotates_when_file_is_large` |
| 13.4 | `audit_log()` 序列化失败时静默返回 | `audit.rs:30-32` `let Ok(line)` guard | ✅ `security::audit::tests::audit_log_silently_returns_when_serializer_fails` |
| 13.5 | `get_audit_file_path()` 与 db_path 同目录 | `audit.rs:50-54` `set_file_name("audit.jsonl")` | ✅ `security::audit::tests::audit_file_path_is_next_to_db_path` |

## 14. Shell 集成 (`src/main.rs` init 命令)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 14.1 | `init powershell` 输出包含 `function xun` 和魔法命令解析 | `main.rs` init 分支生成 PowerShell 脚本 | ✅ `test_basic::init_powershell_contains_wrappers` |
| 14.2 | `init powershell` 包含 list/gc/delete/rename/tag 等子命令包装 | `main.rs` 子命令函数生成 | ✅ `test_basic::init_powershell_contains_wrappers` |
| 14.3 | `init powershell` 使用 `xtree` 而非 `tree`（避免冲突） | `main.rs` tree 别名策略 | ✅ `test_basic::init_powershell_contains_wrappers` |
| 14.4 | `init bash` 使用 `xtree` 而非 `tree` | `main.rs` bash 脚本生成 | ✅ `test_basic::init_bash_uses_xtree_only` |
| 14.5 | `init powershell` 包含 `__ENV_SET__` 和 `__ENV_UNSET__` 解析 | `main.rs` 魔法命令处理 | ✅ `test_basic::init_powershell_contains_wrappers` |

## 15. 运行时选项 (`src/runtime.rs`)

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 15.1 | `init()` 对 `--quiet` / `XUN_QUIET` 设置 quiet 标志 | `runtime.rs` `compute_options()` | ✅ `runtime::tests::compute_options_respects_args_and_env_flags` |
| 15.2 | `init()` 对 `--verbose` / `XUN_VERBOSE` 设置 verbose 标志（并覆盖 quiet） | `runtime.rs` `compute_options()` | ✅ `runtime::tests::compute_options_verbose_disables_quiet` |
| 15.3 | `init()` 对 `--non-interactive` / `XUN_NON_INTERACTIVE` 设置 non_interactive 标志 | `runtime.rs` `compute_options()` | ✅ `runtime::tests::compute_options_respects_args_and_env_flags` |
| 15.4 | `init()` 对 `--no-color` / `NO_COLOR` 设置 no_color 标志 | `runtime.rs` `compute_options()` | ✅ `runtime::tests::compute_options_respects_args_and_env_flags` |
| 15.5 | `env_flag_value()` 接受常见 truthy 值 | `runtime.rs` `env_flag_value()` | ✅ `runtime::tests::env_flag_value_accepts_common_truthy_values` |

## 16. 加密/解密 (`src/commands/crypt.rs`)（feature: crypt）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 16.1 | `encrypt --to` + `decrypt --identity` age 收发并写入审计日志 | `crypt.rs` recipients 分支 + `audit_log("encrypt_age"/"decrypt_age")` | ✅ `test_crypt_e2e::crypt_age_recipient_roundtrip_and_audit` |
| 16.2 | `encrypt --to` 无效 recipient 退出码 5 | `crypt.rs` recipients 分支错误处理 | ✅ `test_crypt_e2e::crypt_age_invalid_recipient_fails` |
| 16.3 | `decrypt` 缺失 identity/passphrase 退出码 2 | `crypt.rs` decrypt 参数校验分支 | ✅ `test_crypt_e2e::crypt_decrypt_requires_identity_or_passphrase` |
| 16.4 | `encrypt --passphrase` 非交互空口令快速失败 | `crypt.rs` passphrase 分支空值处理 | ✅ `test_crypt_e2e::crypt_encrypt_passphrase_non_interactive_aborts_fast` |
| 16.5 | `encrypt/decrypt --efs` 成功或能力不足返回一致错误码 | `crypt.rs` EFS 分支 | ✅ `test_crypt_e2e::crypt_efs_roundtrip_or_reports_capability_issue` |

## 17. 保护规则 (`src/commands/protect.rs`)（feature: protect, lock）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 17.1 | `protect set` + 删除拦截 + `rm --force --reason` 放行并写入审计 | `protect.rs` `cmd_set()` + `lock.rs` 保护校验 + `audit_log()` | ✅ `test_protect_e2e::protect_blocks_then_force_reason_allows_and_audit_persists` |
| 17.2 | `protect status --format json` 输出字段完整 | `protect.rs` `cmd_status()` JSON 分支 | ✅ `test_dry_run_format::protect_status_json_is_array_and_fields_exist` |

## 18. 性能与资源 (`tests/test_performance.rs`)（部分 `#[ignore]`）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 18.1 | `tree` 大目录性能（`#[ignore]`） | `test_performance.rs` `perf_tree_large_directory` | ✅ `test_performance::perf_tree_large_directory` |
| 18.2 | `list` 热缓存性能（`#[ignore]`） | `test_performance.rs` `speed_list_hot` | ✅ `test_performance::speed_list_hot` |
| 18.3 | 句柄数稳定性（`#[ignore]`） | `test_performance.rs` `resource_handle_count_stable` | ✅ `test_performance::resource_handle_count_stable` |
| 18.4 | 工作集内存稳定性（`#[ignore]`） | `test_performance.rs` `resource_memory_working_set_stable` | ✅ `test_performance::resource_memory_working_set_stable` |
| 18.5 | CPU 峰值控制（`#[ignore]`） | `test_performance.rs` `resource_cpu_peak_percent` | ✅ `test_performance::resource_cpu_peak_percent` |
| 18.6 | `lock who` 单文件耗时阈值（feature: lock） | `test_performance.rs` `perf_lock_who_single_file_under_200ms` | ✅ `test_performance::perf_lock_who_single_file_under_200ms` |
| 18.7 | `rm` 删除 1k 文件耗时阈值（feature: lock） | `test_performance.rs` `perf_rm_delete_1k_files_under_5s` | ✅ `test_performance::perf_rm_delete_1k_files_under_5s` |
| 18.8 | `backup` 500 文件全量备份耗时（`#[ignore]`） | `test_performance.rs` `perf_backup_full_500_files` | ✅ `test_performance::perf_backup_full_500_files` |
| 18.9 | `backup --incremental` 50 文件变更耗时（`#[ignore]`） | `test_performance.rs` `perf_backup_incremental_50_changed_files` | ✅ `test_performance::perf_backup_incremental_50_changed_files` |
| 18.10 | `restore` 目录备份全量恢复耗时（`#[ignore]`） | `test_performance.rs` `perf_restore_dir_500_files` | ✅ `test_performance::perf_restore_dir_500_files` |
| 18.11 | `restore` zip 备份全量恢复耗时（`#[ignore]`） | `test_performance.rs` `perf_restore_zip_500_files` | ✅ `test_performance::perf_restore_zip_500_files` |

---

## 19. Redirect 分类引擎（`src/commands/redirect/*`）（feature: redirect）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 19.1 | 基础 ext 分类（move） | `redirect/engine.rs` + `matcher.rs` ext | ✅ `test_redirect_e2e::redirect_moves_by_ext_to_dest_dir` |
| 19.2 | glob 匹配 + 规则顺序首条命中 | `redirect/matcher.rs` AND + short-circuit | ✅ `test_redirect_e2e::redirect_glob_matches_and_rule_order_is_first_match` |
| 19.3 | `--dry-run` 零副作用（文件不移动/不建目录） | `redirect/engine.rs` dry-run 分支 | ✅ `test_redirect_e2e::redirect_dry_run_has_no_side_effects` |
| 19.4 | `--copy` 不删除源文件 | `redirect/engine.rs` copy 分支 | ✅ `test_redirect_e2e::redirect_copy_keeps_source_file` |
| 19.5 | 冲突 `rename_new` 追加 `(n)` | `redirect/engine.rs` `unique_dest_path()` | ✅ `test_redirect_e2e::redirect_rename_new_conflict_adds_suffix` |
| 19.6 | 冲突 `rename_date` 追加时间戳 | `redirect/engine.rs` `unique_dest_path_with_timestamp()` | ✅ `test_redirect_e2e::redirect_rename_date_conflict_adds_timestamp_suffix` |
| 19.7 | 冲突 `rename_existing` 先改名旧目标再落新文件 | `redirect/engine.rs` `rename_existing` 分支 | ✅ `test_redirect_e2e::redirect_rename_existing_conflict_renames_old_file_and_moves_new_in_place` |
| 19.8 | 冲突 `trash` 删除旧目标后落新文件 | `windows/trash.rs` + `engine.rs` | ✅ `test_redirect_e2e::redirect_trash_conflict_removes_existing_then_moves_new` |
| 19.9 | `hash_dedup`：相同内容 move 删除源并记录 action=dedup | `redirect/engine.rs` SHA-256 | ✅ `test_redirect_e2e::redirect_hash_dedup_move_deletes_source_when_dest_same_content` |
| 19.10 | `hash_dedup`：相同内容 copy 跳过（保留源） | `redirect/engine.rs` copy dedup 分支 | ✅ `test_redirect_e2e::redirect_hash_dedup_copy_skips_when_dest_same_content` |
| 19.11 | `hash_dedup`：不同内容回退为 rename_new | `redirect/engine.rs` fallback | ✅ `test_redirect_e2e::redirect_hash_dedup_different_content_falls_back_to_rename_new` |
| 19.12 | 匹配器 `regex` | `redirect/matcher.rs` regex | ✅ `test_redirect_e2e::redirect_regex_matches_file_name` |
| 19.13 | 匹配器 `size` | `redirect/matcher.rs` size expr | ✅ `test_redirect_e2e::redirect_size_matches_file_size` |
| 19.14 | 匹配器 `age`（mtime） | `redirect/matcher.rs` age expr | ✅ `test_redirect_e2e::redirect_age_matches_file_mtime` |
| 19.15 | `unmatched=archive:<age>:<dest>` 归档旧文件到 Others | `redirect/config.rs` parse + `engine.rs` | ✅ `test_redirect_e2e::redirect_unmatched_archive_moves_old_files_to_others` |
| 19.16 | `dest` 模板：`{created.year}/{created.month}` | `redirect/engine.rs` template render | ✅ `test_redirect_e2e::redirect_dest_template_renders_created_year_month` |
| 19.17 | `recursive/max_depth` 递归扫描 + `.xunignore` 生效 | `redirect/engine.rs` recursive scan + ignore | ✅ `test_redirect_e2e::redirect_recursive_scan_moves_nested_files_and_respects_xunignore` |
| 19.23 | `max_depth` 边界：超深目录不扫描 | `redirect/engine.rs` max_depth | ✅ `test_redirect_e2e::redirect_recursive_scan_respects_max_depth` |
| 19.18 | `--undo <tx>`：move 反向回滚 | `redirect/undo.rs` + `audit.jsonl` | ✅ `test_redirect_undo::redirect_undo_restores_moved_file` |
| 19.19 | `--undo <tx>`：copy 撤销仅删除副本 | `redirect/undo.rs` copy semantics | ✅ `test_redirect_undo::redirect_undo_removes_copied_file` |
| 19.20 | `--undo <tx>`：dedup 不可恢复需明确报错 | `redirect/undo.rs` dedup branch | ✅ `test_redirect_undo::redirect_undo_reports_dedup_as_unrestorable` |
| 19.24 | `--undo <tx>`：`dst` 路径中包含 ` copy=` 片段也能正确解析 | `redirect/undo.rs` params parse | ✅ `test_redirect_undo::redirect_undo_parses_dst_even_if_path_contains_copy_param_marker` |
| 19.25 | `--plan <file>`：生成 plan 文件且无副作用 | `redirect/mod.rs` plan_redirect + JSON | ✅ `test_redirect_tools::redirect_plan_writes_plan_file_without_side_effects` |
| 19.26 | `--apply <file>`：指纹不一致标记 `stale` 并跳过 | `redirect/engine.rs` apply_plan_item stale check | ✅ `test_redirect_tools::redirect_apply_executes_plan_and_skips_stale_items` |
| 19.27 | 性能：watch name-only 预过滤 + regex/size/age 解析缓存 | `redirect/matcher.rs` `any_rule_matches_name_only` + `REGEX_CACHE` + `SIZE_EXPR_CACHE/AGE_EXPR_CACHE`，`redirect/watcher.rs` prefilter | ✅ `commands::redirect::matcher::tests::*` |
| 19.21 | `--watch`：新文件移动 + 空目录清理 + 锁重试 + 溢出补账 | `redirect/watcher.rs` + `watch_core.rs` | ✅ `test_redirect_watch::*` |
| 19.22 | P3.4 跨盘进度条/取消：需要跨卷或手动 Ctrl+C 场景验证 | `CopyFileExW/MoveFileWithProgressW` progress routine | ⬚（手动） |

## 20. Dashboard API（`src/commands/dashboard/*`）（feature: dashboard）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 20.1 | `/api/config` 返回 JSON（含 `proxy/tree` 等） | `handlers.rs` `get_config()` | ✅ `test_dashboard_api::dashboard_api_config_returns_json` |
| 20.2 | `/api/config` POST patch 更新/清空字段 | `handlers.rs` `post_config_patch()` | ✅ `test_dashboard_api::dashboard_api_config_patch_updates_and_clears_fields` |
| 20.3 | `/api/config` PUT 替换配置 | `handlers.rs` `put_config_replace()` | ✅ `test_dashboard_api::dashboard_api_config_put_replaces_config` |
| 20.4 | `/api/bookmarks/{name}/rename` 重命名回环 | `handlers.rs` `rename_bookmark()` | ✅ `test_dashboard_api::dashboard_api_bookmark_rename_roundtrip` |
| 20.5 | `/api/bookmarks/export` 支持 `json/tsv` | `handlers.rs` `export_bookmarks()` | ✅ `test_dashboard_api::dashboard_api_bookmark_export_supports_json_and_tsv` |
| 20.6 | `/api/bookmarks/import` 支持 `json/tsv` | `handlers.rs` `import_bookmarks()` | ✅ `test_dashboard_api::dashboard_api_bookmark_import_supports_json_and_tsv` |
| 20.7 | `/api/bookmarks/batch` 批量删/加/减标签 | `handlers.rs` `bookmarks_batch()` | ⬚ 待补 |
| 20.8 | `/api/bookmarks` 列表（含 tags/visits/last_visited） | `handlers.rs` `list_bookmarks()` | ⬚ 待补 |
| 20.9 | `/api/ports` 返回详情（含 cmdline/cwd） | `handlers.rs` `list_ports()` | ✅ `test_dashboard_api::dashboard_api_ports_supports_kill_pid_and_details` |
| 20.10 | `/api/ports/kill-pid/{pid}` 终止进程 | `handlers.rs` `kill_pid()` | ✅ `test_dashboard_api::dashboard_api_ports_supports_kill_pid_and_details` |
| 20.11 | `/api/ports/icon/{pid}` 返回进程图标 | `handlers.rs` `port_icon()` | ⬚ 待补 |
| 20.12 | `/api/ports/kill/{port}` 终止端口进程 | `handlers.rs` `kill_port()` | ⬚ 待补 |
| 20.13 | `/api/proxy/status` 代理状态汇总 | `handlers.rs` `proxy_status()` | ⬚ 待补 |
| 20.14 | `/api/proxy/config` 读写代理配置 | `handlers.rs` `get_proxy_config()` / `set_proxy_config()` | ⬚ 待补 |
| 20.15 | `/api/proxy/test` 支持 timeout/jobs | `handlers.rs` `proxy_test()` | ⬚ 待补 |
| 20.16 | `/api/proxy/set` / `/api/proxy/del` 应用/移除代理 | `handlers.rs` `proxy_set()` / `proxy_del()` | ⬚ 待补 |
| 20.17 | `/api/redirect/profiles`：GET 回环 | `handlers.rs` redirect CRUD | ✅ `test_dashboard_api::dashboard_api_redirect_profiles_roundtrip` |
| 20.18 | `/api/redirect/profiles/{name}`：POST 回环 | `handlers.rs` redirect CRUD | ✅ `test_dashboard_api::dashboard_api_redirect_profiles_roundtrip` |
| 20.19 | `/api/redirect/dry-run` 返回预演结果 | `handlers.rs` `redirect_dry_run()` | ✅ `test_dashboard_api::dashboard_api_redirect_dry_run_returns_results` |
| 20.20 | `/api/audit` 支持 range/cursor/csv | `handlers.rs` `get_audit()` | ✅ `test_dashboard_api::dashboard_api_audit_supports_range_cursor_and_csv` |

## 21. ACL 管理（`src/acl/*`）

### 21.1 读取与保护路径（`acl/reader.rs`）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 21.1.1 | `is_protected_path()` 识别系统保护路径（含大小写、非保护路径） | `reader.rs` `is_protected_path()` | ✅ `acl::reader::tests::is_protected_path_matches_recycle_bin`, `acl::reader::tests::is_protected_path_case_insensitive`, `acl::reader::tests::is_protected_path_normal_dir_is_false` |
| 21.1.2 | `get_acl()` 能读取临时目录 ACL | `reader.rs` `get_acl()` | ✅ `acl::reader::tests::get_acl_temp_dir` |
| 21.1.3 | `list_children()` 能枚举临时目录 | `reader.rs` `list_children()` | ✅ `acl::reader::tests::list_children_temp_dir` |

### 21.2 差异对比（`acl/diff.rs`）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 21.2.1 | 相同 ACL 与空 ACL 不产生差异 | `diff.rs` `diff_acl()` | ✅ `acl::diff::tests::identical_snapshots_no_diff`, `acl::diff::tests::both_empty` |
| 21.2.2 | owner 变更可检测 | `diff.rs` `owner_diff` | ✅ `acl::diff::tests::owner_diff_detected` |
| 21.2.3 | inheritance 变更可检测 | `diff.rs` `inherit_diff` | ✅ `acl::diff::tests::inherit_diff_detected` |
| 21.2.4 | A/B 单侧存在差异 | `diff.rs` only_in_a / only_in_b | ✅ `acl::diff::tests::entry_only_in_a`, `acl::diff::tests::entry_only_in_b` |
| 21.2.5 | 继承与显式 ACE 视为不同 | `diff.rs` entry key 规则 | ✅ `acl::diff::tests::inherited_vs_explicit_are_different` |
| 21.2.6 | 完全不同 ACL 有差异 | `diff.rs` diff 统计 | ✅ `acl::diff::tests::completely_different_snapshots` |

### 21.3 有效权限（`acl/effective.rs`）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 21.3.1 | Allow/deny 基本规则（deny 优先） | `effective.rs` `compute_effective_access()` | ✅ `acl::effective::tests::allow_read_granted`, `acl::effective::tests::deny_overrides_allow` |
| 21.3.2 | FullControl 授权覆盖 6 个核心权限 | `effective.rs` 全量位判定 | ✅ `acl::effective::tests::full_control_grants_all` |
| 21.3.3 | 用户 SID / 组 SID 匹配与无匹配 | `effective.rs` SID 匹配 | ✅ `acl::effective::tests::group_membership_via_sid_list`, `acl::effective::tests::no_matching_sids_all_no_rule` |

### 21.4 解析与截断（`acl/parse.rs`）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 21.4.1 | `parse_ace_type()` 支持 Allow/Deny/非法输入 | `parse.rs` ACE 类型解析 | ✅ `acl::parse::tests::parse_ace_type_allow_variants`, `acl::parse::tests::parse_ace_type_deny_variants`, `acl::parse::tests::parse_ace_type_invalid_errors` |
| 21.4.2 | `parse_inheritance()` 支持 None/Object/Container/Both/非法 | `parse.rs` 继承解析 | ✅ `acl::parse::tests::parse_inheritance_none_variants`, `acl::parse::tests::parse_inheritance_object_variants`, `acl::parse::tests::parse_inheritance_container_variants`, `acl::parse::tests::parse_inheritance_both_variants`, `acl::parse::tests::parse_inheritance_invalid_errors`, `acl::parse::tests::parse_inheritance_strips_hyphens` |
| 21.4.3 | `parse_rights()` 支持表项/大小写/十进制/十六进制/非法 | `parse.rs` 权限解析 | ✅ `acl::parse::tests::parse_rights_all_table_entries`, `acl::parse::tests::parse_rights_case_insensitive`, `acl::parse::tests::parse_rights_decimal`, `acl::parse::tests::parse_rights_hex_lowercase`, `acl::parse::tests::parse_rights_hex_uppercase`, `acl::parse::tests::parse_rights_hex_uppercase_prefix`, `acl::parse::tests::parse_rights_invalid_hex_errors`, `acl::parse::tests::parse_rights_unknown_string_errors`, `acl::parse::tests::parse_rights_empty_string_errors`, `acl::parse::tests::parse_rights_zero` |
| 21.4.4 | `truncate()` / `truncate_left()` 处理边界 | `parse.rs` 截断逻辑 | ✅ `acl::parse::tests::truncate_exact_length_unchanged`, `acl::parse::tests::truncate_short_string_unchanged`, `acl::parse::tests::truncate_long_string_ellipsis`, `acl::parse::tests::truncate_left_long_keeps_tail`, `acl::parse::tests::truncate_left_short_unchanged`, `acl::parse::tests::truncate_max_zero_gives_ellipsis` |

### 21.5 备份与导出（`acl/export.rs`）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 21.5.1 | 备份文件名规范化 + 备份/恢复回环 | `export.rs` backup/restore | ✅ `acl::export::tests::backup_filename_is_sanitized`, `acl::export::tests::backup_roundtrip` |
| 21.5.2 | diff CSV 导出行数正确 | `export.rs` `export_diff_csv()` | ✅ `acl::export::tests::export_diff_csv_writes_rows` |
| 21.5.3 | ACL/repair 导出 CSV 行数正确 | `export.rs` `export_acl_csv()` / `export_repair_errors_csv()` | ✅ `acl::export::tests::export_acl_csv_row_count`, `acl::export::tests::export_repair_errors_csv_counts` |

### 21.6 孤儿 SID（`acl/orphan.rs`）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 21.6.1 | 孤儿 SID 过滤逻辑 | `orphan.rs` 过滤分支 | ✅ `acl::orphan::tests::orphan_filter_logic` |

### 21.7 审计日志（`acl/audit.rs`）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 21.7.1 | append + tail + 轮转裁剪 | `audit.rs` `append()` / `rotate_if_needed()` | ✅ `acl::audit::tests::append_and_tail`, `acl::audit::tests::rotation_trims_to_max`, `acl::audit::tests::tail_returns_newest_last` |
| 21.7.2 | CSV 导出 | `audit.rs` `export_csv()` | ✅ `acl::audit::tests::export_csv_writes_rows` |

### 21.8 特权启用（`acl/privilege.rs`）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 21.8.1 | 恢复特权启用 + 非法特权报错 | `privilege.rs` `enable_privilege()` | ✅ `acl::privilege::tests::enable_restore_privilege_does_not_panic`, `acl::privilege::tests::invalid_privilege_name_returns_error` |

### 21.9 数据结构（`acl/types.rs`）

| # | 测试项 | 源码依据 | 状态 |
|---|--------|----------|------|
| 21.9.1 | AceType/TriState 显示值 | `types.rs` Display impl | ✅ `acl::types::tests::ace_type_display`, `acl::types::tests::tri_state_display` |
| 21.9.2 | rights_short 映射与 Synchronize 剥离 | `types.rs` `rights_short()` | ✅ `acl::types::tests::rights_short_known_values`, `acl::types::tests::rights_short_strips_synchronize` |
| 21.9.3 | rights_desc 表项非空 | `types.rs` `rights_desc()` | ✅ `acl::types::tests::rights_desc_all_table_entries_nonempty` |
| 21.9.4 | 继承/传播 flags 显示值 | `types.rs` flag Display | ✅ `acl::types::tests::inheritance_flags_display`, `acl::types::tests::propagation_flags_display` |
| 21.9.5 | diff_key 稳定性 | `types.rs` `diff_key()` | ✅ `acl::types::tests::ace_entry_diff_key_stable` |
| 21.9.6 | Snapshot 统计 | `types.rs` `count_*()` | ✅ `acl::types::tests::acl_snapshot_counts` |
| 21.9.7 | RepairStats 汇总输出 | `types.rs` `summary()` | ✅ `acl::types::tests::repair_stats_summary` |

## 汇总统计

| 模块 | 总计 | ✅ 已有 | ⬚ 待补 |
|------|------|---------|---------|
| 1. 书签存储 | 11 | 11 | 0 |
| 2. 模糊匹配 | 10 | 10 | 0 |
| 3. 数据模型 | 7 | 7 | 0 |
| 4. 输出格式化 | 7 | 7 | 0 |
| 5. 配置管理 | 7 | 7 | 0 |
| 6. 工具函数 | 8 | 8 | 0 |
| 7. 书签 CRUD | 41 | 41 | 0 |
| 8. 目录树 | 18 | 18 | 0 |
| 9. 增量备份 | 10 | 10 | 0 |
| 10. 端口管理 | 22 | 22 | 0 |
| 11. 文件锁定 | 60 | 60 | 0 |
| 12. 代理管理 | 32 | 32 | 0 |
| 13. 审计日志 | 5 | 5 | 0 |
| 14. Shell 集成 | 5 | 5 | 0 |
| 15. 运行时选项 | 5 | 5 | 0 |
| 16. 加密/解密 | 5 | 5 | 0 |
| 17. 保护规则 | 2 | 2 | 0 |
| 18. 性能与资源 | 7 | 7 | 0 |
| 19. Redirect 分类引擎 | 26 | 25 | 1 |
| 20. Dashboard API | 20 | 12 | 8 |
| 21. ACL 管理 | 27 | 27 | 0 |
| **合计** | **335** | **326** | **9** |
