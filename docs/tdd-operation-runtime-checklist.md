# XunYu Operation Runtime 重构 — TDD 任务清单

> 版本: 1.0
> 日期: 2026-05-12
> 关联: [CLI 重构方案](./project/CLI-Refactor-Plan.md) · [CLI 现状](./project/CLI-Current-State.md) · [Dashboard 重构](./project/Dashboard-UI-Refactor.md)
> 方法论: TDD (Red → Green → Refactor)，每个任务单元独立可验证

---

## 总览

本文档将 CLI-Refactor-Plan 和 Dashboard-UI-Refactor 的全部工作拆解为 TDD 任务单元。
每个任务遵循：**写失败测试 → 最小实现 → 重构优化** 的循环。

### 里程碑

| Phase | 名称 | 产出 | 预计耗时 |
|-------|------|------|----------|
| 0 | 环境准备 | clap 共存、core/ 骨架、前端 Pinia | 1 天 |
| 1 | core/ 基础设施 | Value/Renderer/Error/Context/Args/Command/Operation | 3 天 |
| 2 | Proxy 验证 | 端到端验证新架构（CLI + Dashboard） | 2 天 |
| 3 | 全量迁移 | 所有命令迁移到新架构 | 1-2 周 |
| 4 | Dashboard 打通 | WebSocket 协议、DataTable、OperationDialog | 1 周 |
| 5 | 清理交付 | 删除 argh、统一命名、性能基准 | 2 天 |

---

## Phase 0：环境准备 (Alignment)

### 0.1 Rust 侧依赖引入

- [ ] **确认输入**：阅读 CLI-Refactor-Plan.md 第八节附录 8.1 依赖变更。
- [ ] **添加 clap 依赖**：
  ```toml
  clap = { version = "4", features = ["derive", "env", "unicode", "wrap_help"], default-features = false }
  clap_complete = "4"
  ```
- [ ] **验证共存**：`cargo check` 通过（argh + clap 同时存在）。
- [ ] **创建 `src/core/mod.rs`**：空模块，`pub mod` 声明所有子模块。
- [ ] **在 `src/lib.rs` 中引入**：`pub mod core;`
- [ ] **运行全量测试**：`cargo test` 499+ 测试全部通过，无回归。

### 0.2 前端侧依赖引入

- [ ] **安装 Pinia**：`pnpm -C dashboard-ui add pinia`
- [ ] **创建 stores/ 目录**：`dashboard-ui/src/stores/`
- [ ] **在 main.ts 注册 Pinia**。
- [ ] **创建 `generated/` 目录**：占位，后续放自动生成类型。
- [ ] **运行前端测试**：`pnpm -C dashboard-ui test` 全部通过。

### 0.3 测试脚手架

- [ ] **创建 `tests/core_integration.rs`**：core 模块集成测试入口。
- [ ] **创建 `src/core/` 下各子模块空文件**：
  - `error.rs`, `value.rs`, `renderer.rs`, `output.rs`, `table_row.rs`
  - `operation.rs`, `args.rs`, `context.rs`, `command.rs`, `shell.rs`
- [ ] **验证**：`cargo check` 通过。



---

## Phase 1：core/ 基础设施 (Red → Green → Refactor)

### 1.1 XunError — 分层错误类型

#### 🔴 Red

- [ ] **测试用例 1：User 错误构造与 exit code**
  ```rust
  #[test] fn user_error_has_code_1()
  #[test] fn user_error_with_hints_preserves_hints()
  ```
- [ ] **测试用例 2：错误类型分类**
  ```rust
  #[test] fn cancelled_has_code_130()
  #[test] fn elevation_required_has_code_77()
  #[test] fn not_found_has_code_2()
  #[test] fn internal_error_from_anyhow()
  ```
- [ ] **测试用例 3：Display trait 输出**
  ```rust
  #[test] fn display_user_error_shows_message()
  #[test] fn display_internal_error_transparent()
  ```
- [ ] **运行测试**：确认全部失败（XunError 未定义）。

#### 🟢 Green

- [ ] **实现 `src/core/error.rs`**：XunError enum + thiserror derive + exit_code() 方法。
- [ ] **运行测试**：全部通过。
- [ ] **回归**：`cargo test` 全量通过。

#### 🔵 Refactor

- [ ] 确保 `impl From<anyhow::Error> for XunError` 自动转换。
- [ ] 确保 `XunError` 实现 `Send + Sync`。
- [ ] 运行测试确认通过。

---

### 1.2 StructuredValue — 统一数据模型

#### 🔴 Red

- [ ] **测试用例 1：Value 基本类型构造**
  ```rust
  #[test] fn value_string_roundtrip_json()
  #[test] fn value_int_roundtrip_json()
  #[test] fn value_bool_roundtrip_json()
  #[test] fn value_null_serializes_to_null()
  ```
- [ ] **测试用例 2：Record 和 List**
  ```rust
  #[test] fn record_ordered_keys()
  #[test] fn list_heterogeneous_values()
  #[test] fn nested_record_in_list()
  ```
- [ ] **测试用例 3：Table 结构**
  ```rust
  #[test] fn table_with_columns_and_rows()
  #[test] fn table_serializes_with_schema()
  #[test] fn empty_table_valid()
  ```
- [ ] **测试用例 4：语义类型**
  ```rust
  #[test] fn value_duration_serializes_as_millis()
  #[test] fn value_filesize_serializes_as_bytes()
  #[test] fn value_date_serializes_iso8601()
  ```
- [ ] **运行测试**：确认全部失败。

#### 🟢 Green

- [ ] **实现 `src/core/value.rs`**：Value enum + Record type alias + Table struct + ColumnDef + ValueKind。
- [ ] **derive Serialize/Deserialize/Clone/Debug**。
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 为 Value 实现 `From<String>`, `From<i64>`, `From<bool>` 等便捷转换。
- [ ] 为 Table 实现 `Table::new(columns)` + `push_row()` builder。
- [ ] 运行测试确认通过。

#### 🔄 边界

- [ ] **测试用例 5：大 Table 性能**
  ```rust
  #[test] fn table_10k_rows_serializes_under_50ms()
  ```
- [ ] 实现并确认通过。



---

### 1.3 Renderer — 多端输出

#### 🔴 Red

- [ ] **测试用例 1：TerminalRenderer 表格输出**
  ```rust
  #[test] fn terminal_renders_table_with_headers()
  #[test] fn terminal_renders_single_record()
  #[test] fn terminal_respects_no_color()
  ```
- [ ] **测试用例 2：JsonRenderer**
  ```rust
  #[test] fn json_renders_table_as_array()
  #[test] fn json_renders_value_directly()
  #[test] fn json_pretty_vs_compact()
  ```
- [ ] **测试用例 3：TsvRenderer**
  ```rust
  #[test] fn tsv_renders_table_tab_separated()
  #[test] fn tsv_escapes_tabs_in_values()
  ```
- [ ] **测试用例 4：OutputFormat 自动检测**
  ```rust
  #[test] fn auto_format_tty_returns_table()
  #[test] fn auto_format_pipe_returns_json()
  ```
- [ ] **运行测试**：确认全部失败。

#### 🟢 Green

- [ ] **实现 `src/core/renderer.rs`**：
  - `Renderer` trait（render_value, render_table, render_info, render_warning）
  - `TerminalRenderer`（comfy_table 渲染 Table）
  - `JsonRenderer`（serde_json 序列化）
  - `OutputFormat` enum + resolve 逻辑
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 提取 `render_tsv()` 和 `render_csv()` 为独立函数。
- [ ] TerminalRenderer 复用现有 `apply_pretty_table_style()`。
- [ ] 运行测试确认通过。

---

### 1.4 公共参数组 — Args

#### 🔴 Red

- [ ] **测试用例 1：ListArgs 解析**
  ```rust
  #[test] fn list_args_defaults()
  #[test] fn list_args_custom_limit_and_sort()
  #[test] fn list_args_reverse_flag()
  ```
- [ ] **测试用例 2：FuzzyArgs 解析**
  ```rust
  #[test] fn fuzzy_args_multiple_patterns()
  #[test] fn fuzzy_args_list_flag()
  ```
- [ ] **测试用例 3：ScopeArgs 解析**
  ```rust
  #[test] fn scope_args_global_flag()
  #[test] fn scope_args_workspace_option()
  ```
- [ ] **测试用例 4：ConfirmArgs 解析**
  ```rust
  #[test] fn confirm_args_yes_skips_prompt()
  #[test] fn confirm_args_dry_run_flag()
  ```
- [ ] **运行测试**：确认全部失败。

#### 🟢 Green

- [ ] **实现 `src/core/args.rs`**：ListArgs / FuzzyArgs / ScopeArgs / ConfirmArgs，全部 `#[derive(Args, Clone, Debug)]`。
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 为 ListArgs 实现 `apply_to_iter()` 方法（skip/take/sort）。
- [ ] 运行测试确认通过。



---

### 1.5 CmdContext — 执行上下文

#### 🔴 Red

- [ ] **测试用例 1：构造与默认值**
  ```rust
  #[test] fn context_default_format_is_auto()
  #[test] fn context_respects_quiet_flag()
  #[test] fn context_respects_verbose_flag()
  ```
- [ ] **测试用例 2：配置延迟加载**
  ```rust
  #[test] fn config_not_loaded_until_accessed()
  #[test] fn config_loaded_once_and_cached()
  ```
- [ ] **测试用例 3：交互判断**
  ```rust
  #[test] fn non_interactive_flag_disables_confirm()
  #[test] fn confirm_returns_true_when_non_interactive()
  ```
- [ ] **运行测试**：确认全部失败。

#### 🟢 Green

- [ ] **实现 `src/core/context.rs`**：CmdContext struct + from_test() 构造器 + config() 延迟加载。
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 确保 CmdContext 不依赖全局状态（无 OnceLock）。
- [ ] 提供 `CmdContext::for_test()` 方便测试构造。
- [ ] 运行测试确认通过。

---

### 1.6 CommandSpec — 统一命令 trait

#### 🔴 Red

- [ ] **测试用例 1：基本 CommandSpec 实现**
  ```rust
  // 定义一个 MockCmd 实现 CommandSpec
  #[test] fn command_spec_run_returns_output()
  #[test] fn command_spec_validate_default_passes()
  ```
- [ ] **测试用例 2：execute 函数集成**
  ```rust
  #[test] fn execute_calls_validate_then_run()
  #[test] fn execute_renders_output_via_renderer()
  #[test] fn execute_returns_error_on_validate_failure()
  ```
- [ ] **测试用例 3：Pipeline middleware**
  ```rust
  #[test] fn pipeline_before_runs_before_command()
  #[test] fn pipeline_after_runs_after_command()
  #[test] fn pipeline_before_error_aborts_execution()
  ```
- [ ] **运行测试**：确认全部失败。

#### 🟢 Green

- [ ] **实现 `src/core/command.rs`**：
  - `CommandSpec` trait（关联类型 Output: Renderable）
  - `Pipeline` struct（before/after middleware Vec）
  - `execute()` 函数
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 确保 `execute()` 是泛型函数，零 vtable 开销。
- [ ] 运行测试确认通过。

---

### 1.7 Operation — 危险操作协议

#### 🔴 Red

- [ ] **测试用例 1：Preview 结构**
  ```rust
  #[test] fn preview_serializes_to_json()
  #[test] fn preview_with_changes_and_risk_level()
  #[test] fn empty_preview_valid()
  ```
- [ ] **测试用例 2：Operation trait 基本流程**
  ```rust
  // MockOperation 实现 Operation trait
  #[test] fn run_operation_calls_preview_then_execute()
  #[test] fn run_operation_skips_confirm_when_yes_flag()
  #[test] fn run_operation_aborts_on_cancel()
  ```
- [ ] **测试用例 3：风险等级与确认**
  ```rust
  #[test] fn high_risk_forces_confirm_even_without_flag()
  #[test] fn low_risk_skips_confirm_when_no_flag()
  ```
- [ ] **测试用例 4：OperationResult**
  ```rust
  #[test] fn operation_result_serializes_with_duration()
  #[test] fn operation_result_tracks_changes_applied()
  ```
- [ ] **运行测试**：确认全部失败。

#### 🟢 Green

- [ ] **实现 `src/core/operation.rs`**：
  - `Preview`, `Change`, `RiskLevel`, `OperationResult` structs
  - `Operation` trait（preview/execute/rollback）
  - `run_operation()` 函数
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 确保 `Preview` 和 `OperationResult` 都 derive `Serialize`（Dashboard 兼容）。
- [ ] 运行测试确认通过。

#### 🔄 边界

- [ ] **测试用例 5：rollback 默认不支持**
  ```rust
  #[test] fn default_rollback_returns_error()
  ```
- [ ] 实现并确认通过。



---

### 1.8 ShellIntegration — Shell 集成 trait

#### 🔴 Red

- [ ] **测试用例 1：PowerShell 渲染**
  ```rust
  #[test] fn powershell_renders_alias()
  #[test] fn powershell_renders_function()
  #[test] fn powershell_renders_completion()
  ```
- [ ] **测试用例 2：Bash 渲染**
  ```rust
  #[test] fn bash_renders_alias()
  #[test] fn bash_renders_function()
  ```
- [ ] **运行测试**：确认全部失败。

#### 🟢 Green

- [ ] **实现 `src/core/shell.rs`**：ShellIntegration trait + PowerShell/Bash impl。
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 从现有 `dispatch/core.rs` 提取 shell 脚本模板到新 trait impl。
- [ ] 运行测试确认通过。

---

### 1.9 Phase 1 交付检查

- [ ] `cargo test -p xun --lib` 全部通过（含 core 模块所有测试）。
- [ ] `cargo clippy` 无警告。
- [ ] core/ 模块 100% 独立，不依赖现有 commands/cli 模块。
- [ ] 所有 Serialize 类型可被 `serde_json::to_string()` 正确序列化。
- [ ] 提交 commit：`feat(core): implement Operation Runtime foundation`

---

## Phase 2：Proxy 端到端验证

### 2.1 Proxy CLI 定义（clap derive）

#### 🔴 Red

- [ ] **测试用例 1：clap 解析**
  ```rust
  #[test] fn proxy_set_parses_url_and_noproxy()
  #[test] fn proxy_show_parses_empty_args()
  #[test] fn proxy_rm_parses_only_option()
  ```
- [ ] **运行测试**：确认失败（struct 未定义）。

#### 🟢 Green

- [ ] **创建 `src/cli/proxy_v2.rs`**：用 clap derive 定义 ProxyCmd / ProxySubCommand / ProxySetCmd 等。
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 确保 `--format` 通过 global 参数继承，不在 proxy 子命令重复定义。
- [ ] 运行测试确认通过。

---

### 2.2 Proxy 输出类型

#### 🔴 Red

- [ ] **测试用例 1：ProxyInfo TableRow 实现**
  ```rust
  #[test] fn proxy_info_columns_correct()
  #[test] fn proxy_info_cells_match_fields()
  ```
- [ ] **测试用例 2：ProxyInfo 渲染**
  ```rust
  #[test] fn proxy_info_renders_as_json()
  #[test] fn proxy_info_renders_as_table()
  #[test] fn proxy_info_renders_as_tsv()
  ```
- [ ] **运行测试**：确认失败。

#### 🟢 Green

- [ ] **实现 ProxyInfo struct** + derive Serialize + impl TableRow。
- [ ] **运行测试**：全部通过。

---

### 2.3 Proxy CommandSpec 实现

#### 🔴 Red

- [ ] **测试用例 1：ProxyShowCmd 返回 ProxyInfo**
  ```rust
  #[test] fn proxy_show_returns_current_config()
  ```
- [ ] **测试用例 2：ProxySetCmd 修改配置**
  ```rust
  #[test] fn proxy_set_updates_config()
  #[test] fn proxy_set_returns_empty_output()
  ```
- [ ] **运行测试**：确认失败。

#### 🟢 Green

- [ ] **实现 CommandSpec for ProxyShowCmd / ProxySetCmd / ProxyRmCmd**。
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 提取 proxy 业务逻辑到 `src/services/proxy.rs`（命令只是薄适配器）。
- [ ] 运行测试确认通过。

---

### 2.4 Proxy dispatch 集成

- [ ] 在 dispatch 中添加新 proxy 路径（feature flag 或直接替换）。
- [ ] **集成测试**：`cargo test` 全量通过。
- [ ] **手动验证**：`xun proxy show --format json` 输出正确 JSON。
- [ ] **手动验证**：`xun proxy show` 在 TTY 输出表格。
- [ ] **手动验证**：`xun proxy show | cat` 输出 JSON（管道检测）。
- [ ] 提交 commit：`feat(proxy): migrate to Operation Runtime architecture`



---

## Phase 3：全量命令迁移

> 每个批次遵循相同 TDD 流程：定义 CLI struct → 输出类型 TableRow → CommandSpec impl → dispatch → 测试。
> 危险操作额外实现 Operation trait。

### 3.1 批次 1：config / ctx / tree / find

#### config (3 子命令)

- [ ] 🔴 测试：ConfigGetCmd / ConfigSetCmd / ConfigEditCmd 解析 + 输出
- [ ] 🟢 实现：CommandSpec for 各 cmd，输出类型 ConfigEntry
- [ ] 🔵 重构：提取 ConfigService

#### ctx (7 子命令)

- [ ] 🔴 测试：CtxSetCmd / CtxUseCmd / CtxListCmd 等解析 + 输出
- [ ] 🟢 实现：CommandSpec for 各 cmd
- [ ] 🔵 重构：ctx 切换逻辑移入 services/

#### tree (单命令)

- [ ] 🔴 测试：TreeCmd 解析 + 输出（Value::String 树形文本）
- [ ] 🟢 实现：CommandSpec for TreeCmd
- [ ] ✅ 回归：`cargo test`

#### find (单命令)

- [ ] 🔴 测试：FindCmd 解析 + 输出（Table 文件列表）
- [ ] 🟢 实现：CommandSpec for FindCmd，输出 Table
- [ ] ✅ 回归：`cargo test`

---

### 3.2 批次 2：port / proc（重组）

- [ ] 🔴 测试：PortListCmd / PortKillCmd 解析
- [ ] 🔴 测试：ProcListCmd / ProcKillCmd 解析
- [ ] 🟢 实现：CommandSpec，PortKillCmd 实现 **Operation trait**（preview 显示将被杀的进程）
- [ ] 🔵 重构：旧 `ports`/`kill`/`ps`/`pkill` 降级为 hidden alias
- [ ] ✅ 回归：`cargo test`

---

### 3.3 批次 3：backup / video / verify

#### backup (6 子命令)

- [ ] 🔴 测试：BackupCreateCmd 解析 + Operation::preview
- [ ] 🔴 测试：BackupRestoreCmd 解析 + Operation::preview
- [ ] 🟢 实现：BackupCreateOperation + BackupRestoreOperation（实现 Operation trait）
- [ ] 🟢 实现：BackupListCmd → CommandSpec（输出 Table）
- [ ] 🔵 重构：提取 BackupService
- [ ] ✅ 回归

#### video (3 子命令)

- [ ] 🔴 测试：VideoCompressCmd / VideoProbeCmd / VideoRemuxCmd
- [ ] 🟢 实现：CommandSpec for 各 cmd
- [ ] ✅ 回归

---

### 3.4 批次 4：bookmark (26 子命令)

- [ ] 🔴 测试：BookmarkZCmd 使用 FuzzyArgs + ScopeArgs（flatten 复用验证）
- [ ] 🔴 测试：BookmarkListCmd 使用 ListArgs（flatten 复用验证）
- [ ] 🔴 测试：BookmarkDeleteCmd 实现 Operation trait（preview 显示将删除的书签）
- [ ] 🟢 实现：所有 26 个子命令的 CommandSpec
- [ ] 🟢 实现：BookmarkEntry TableRow（统一输出）
- [ ] 🔵 重构：确认 z/zi/o/oi 共享 BookmarkQueryArgs（零重复）
- [ ] 🔵 重构：bookmark undo/redo 迁移到 Operation rollback
- [ ] ✅ 回归：`cargo test`（含 bookmark_phase_*.rs 全部通过）

---

### 3.5 批次 5：env (27 子命令)

- [ ] 🔴 测试：EnvListCmd 输出 Table（EnvVar 行）
- [ ] 🔴 测试：EnvSetCmd / EnvDelCmd 实现 Operation trait
- [ ] 🔴 测试：EnvDoctorCmd 输出 Table（issues 列表）
- [ ] 🟢 实现：所有 27 个子命令的 CommandSpec
- [ ] 🟢 实现：EnvVar / EnvDoctorIssue TableRow
- [ ] 🔵 重构：env 子命令分组（path/snapshot/profile 作为嵌套子命令组）
- [ ] ✅ 回归

---

### 3.6 批次 6：acl (15 子命令)

- [ ] 🔴 测试：AclViewCmd 输出 Table
- [ ] 🔴 测试：AclAddCmd / AclRemoveCmd / AclRepairCmd 实现 Operation trait
- [ ] 🟢 实现：所有 15 个子命令
- [ ] 🔵 重构：AclService 提取
- [ ] ✅ 回归

---

### 3.7 批次 7：alias / brn / vault / desktop

#### alias (11 子命令)

- [ ] 🔴 测试 + 🟢 实现：CommandSpec for 各 cmd
- [ ] ✅ 回归

#### brn (单命令，30+ 参数)

- [ ] 🔴 测试：BrnCmd 解析（验证 clap 处理 30+ 参数）
- [ ] 🔴 测试：BrnCmd 实现 Operation trait（preview 显示重命名计划）
- [ ] 🟢 实现：RenameOperation
- [ ] ✅ 回归

#### vault (8 子命令)

- [ ] 🔴 测试 + 🟢 实现：VaultEncCmd / VaultDecCmd 实现 Operation trait
- [ ] ✅ 回归

#### desktop (大量子命令)

- [ ] 🔴 测试 + 🟢 实现：按子命令组批量迁移
- [ ] ✅ 回归

---

### 3.8 批次 8：其他 feature-gated

- [ ] lock / protect / crypt / redirect / diff / img / dashboard(serve) / xunbak
- [ ] 每个模块：🔴 测试 → 🟢 实现 → ✅ 回归
- [ ] 提交 commit：`feat: complete full command migration to Operation Runtime`



---

## Phase 4：Dashboard 打通 (前后端统一)

### 4.1 类型自动生成

#### 🔴 Red (Rust 侧)

- [ ] **测试用例**：specta 导出验证
  ```rust
  #[test] fn exported_types_include_table()
  #[test] fn exported_types_include_preview()
  #[test] fn exported_types_include_operation_result()
  #[test] fn exported_types_include_value()
  ```
- [ ] **运行测试**：确认失败。

#### 🟢 Green

- [ ] 添加 `specta` 依赖，为 Value/Table/Preview/OperationResult 添加 `#[derive(specta::Type)]`。
- [ ] 创建 `src/bin/generate_types.rs`：输出 TypeScript 类型到 stdout。
- [ ] **运行**：`cargo run --bin generate_types > dashboard-ui/src/generated/types.ts`
- [ ] **验证**：生成的 .ts 文件 TypeScript 编译通过。

#### 🔵 Refactor

- [ ] 添加 build script 或 npm script 自动化类型生成。
- [ ] 删除旧 `dashboard-ui/src/types.ts` 中已自动生成的类型（渐进）。

---

### 4.2 WebSocket 命令协议（后端）

#### 🔴 Red

- [ ] **测试用例 1：WS 命令解析**
  ```rust
  #[test] fn ws_command_deserializes_bookmark_list()
  #[test] fn ws_command_deserializes_with_args()
  ```
- [ ] **测试用例 2：WS 响应序列化**
  ```rust
  #[test] fn ws_query_response_serializes_table()
  #[test] fn ws_preview_response_serializes_preview()
  #[test] fn ws_operation_result_serializes()
  ```
- [ ] **测试用例 3：命令执行集成**
  ```rust
  #[test] fn ws_execute_bookmark_list_returns_table()
  #[test] fn ws_execute_backup_create_returns_preview()
  ```
- [ ] **运行测试**：确认失败。

#### 🟢 Green

- [ ] **实现 Dashboard WebSocket handler**：
  - 解析 WsCommand → 调用 CommandSpec::run 或 Operation::preview
  - 返回 WsQueryResponse / WsPreviewResponse / WsOperationResult
- [ ] **实现 DashboardRenderer**：通过 WebSocket tx 推送 Value/Table。
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 统一 HTTP API 和 WS 命令的 dispatch 路径（共享 CommandSpec 调用）。
- [ ] 运行测试确认通过。

---

### 4.3 前端 API 层重构

#### 🔴 Red (Vitest)

- [ ] **测试用例 1：WS 客户端**
  ```typescript
  test('ws client sends command and receives response')
  test('ws client handles preview response')
  test('ws client reconnects on disconnect')
  ```
- [ ] **测试用例 2：Operation composable**
  ```typescript
  test('useOperation sends preview request')
  test('useOperation shows dialog on preview')
  test('useOperation sends confirm on user accept')
  test('useOperation handles cancel')
  ```
- [ ] **运行测试**：确认失败。

#### 🟢 Green

- [ ] **实现 `stores/ws.ts`**：WebSocket 连接管理（Pinia store）。
- [ ] **实现 `api/commands.ts`**：统一命令调用（通过 WS）。
- [ ] **实现 `api/operations.ts`**：Operation 协议封装。
- [ ] **实现 `composables/useOperation.ts`**：Operation UI 流程。
- [ ] **运行测试**：全部通过。

---

### 4.4 通用 DataTable 组件

#### 🔴 Red (Vitest)

- [ ] **测试用例 1：自动列生成**
  ```typescript
  test('DataTable renders columns from Table schema')
  test('DataTable renders rows from Table data')
  test('DataTable handles empty table')
  ```
- [ ] **测试用例 2：交互功能**
  ```typescript
  test('DataTable sorts by column click')
  test('DataTable filters by search input')
  test('DataTable supports row selection')
  ```
- [ ] **测试用例 3：虚拟滚动**
  ```typescript
  test('DataTable enables virtual scroll for >100 rows')
  ```
- [ ] **运行测试**：确认失败。

#### 🟢 Green

- [ ] **实现 `components/shared/DataTable.vue`**：
  - 从 `Table.columns` 自动生成列头
  - 从 `Table.rows` 渲染数据
  - 内置排序、过滤、搜索
  - >100 行自动启用 @tanstack/vue-virtual
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 集成 PrimeVue DataTable 样式（或自定义轻量实现）。
- [ ] 运行测试确认通过。

---

### 4.5 统一 OperationDialog 组件

#### 🔴 Red (Vitest)

- [ ] **测试用例 1：Preview 展示**
  ```typescript
  test('OperationDialog shows summary from Preview')
  test('OperationDialog lists changes')
  test('OperationDialog shows risk level badge')
  ```
- [ ] **测试用例 2：风险等级交互**
  ```typescript
  test('Low risk shows green confirm button')
  test('High risk shows red button with double confirm')
  test('Critical risk requires text input confirmation')
  ```
- [ ] **测试用例 3：事件**
  ```typescript
  test('emits confirm on user accept')
  test('emits cancel on user reject')
  test('emits cancel on Escape key')
  ```
- [ ] **运行测试**：确认失败。

#### 🟢 Green

- [ ] **实现 `components/shared/OperationDialog.vue`**。
- [ ] **运行测试**：全部通过。

---

### 4.6 面板迁移（BookmarksPanel 示范）

#### 🔴 Red

- [ ] **测试用例**：
  ```typescript
  test('BookmarksPanel uses DataTable with backend Table schema')
  test('BookmarksPanel delete triggers OperationDialog')
  test('BookmarksPanel refreshes on ws event')
  ```
- [ ] **运行测试**：确认失败。

#### 🟢 Green

- [ ] **重构 BookmarksPanel.vue**：
  - 使用 `useCommand('bookmark.list')` 获取 Table
  - 使用 `DataTable` 渲染（不再手写列定义）
  - 删除操作使用 `useOperation` + `OperationDialog`
- [ ] **运行测试**：全部通过。

#### 🔵 Refactor

- [ ] 删除 BookmarksPanel 中旧的手写列定义和自定义表格逻辑。
- [ ] 运行测试确认通过。

---

### 4.7 布局优化

- [ ] **实现 WorkspaceNav.vue**：可滚动 tabs + 溢出下拉。
- [ ] **实现 StatusBar.vue**：WS 连接状态 + 最近操作。
- [ ] **实现 EmptyState.vue**：统一空状态组件。
- [ ] **面板 Grid 布局**：CSS Grid auto-fit，面板可折叠。
- [ ] **键盘导航**：Tab/Shift+Tab 面板间切换。
- [ ] **前端全量测试通过**：`pnpm -C dashboard-ui test`
- [ ] 提交 commit：`feat(dashboard): unified DataTable + OperationDialog + WS protocol`



---

## Phase 5：清理与交付

### 5.1 删除 argh

- [ ] 从 Cargo.toml 移除 `argh = "0.1"`。
- [ ] 删除所有旧 `src/cli/*.rs` 中的 argh struct（已被 clap 替代）。
- [ ] 删除 `*_monolith_backup_*.rs` 文件。
- [ ] `cargo build --release` 通过。
- [ ] `cargo test` 全量通过。

### 5.2 统一命名

- [ ] 所有子命令 `list`（不再有 `ls`）。
- [ ] 所有子命令 `rm`（不再有 `delete`/`del`/`remove`）。
- [ ] 所有子命令 `show`（不再有 `get`/`view`）。
- [ ] 所有子命令 `add`（不再有 `create`/`new`）。
- [ ] 旧命令名保留为 `#[command(hide = true)]` alias。
- [ ] `cargo test` 全量通过。

### 5.3 Shell Completion

- [ ] 集成 `clap_complete`，生成 PowerShell/Bash/Zsh/Fish completion。
- [ ] 替换现有手写 completion 逻辑。
- [ ] 手动验证：PowerShell 中 `xun <Tab>` 正常补全。

### 5.4 性能基准

- [ ] **hyperfine 测试**：
  ```powershell
  hyperfine "xun bookmark z foo" --warmup 3
  hyperfine "xun --help" --warmup 3
  hyperfine "xun proxy show --format json" --warmup 3
  ```
- [ ] 确认 `xun z foo` < 20ms。
- [ ] 确认 `xun --help` < 10ms。
- [ ] **编译时间**：`cargo build --timings` 记录，确认增量编译 < 5s。
- [ ] **二进制大小**：`ls target/release/xun.exe`，确认增加 < 500KB。

### 5.5 前端最终验证

- [ ] `pnpm -C dashboard-ui build` 无错误。
- [ ] `pnpm -C dashboard-ui test` 全部通过。
- [ ] 手动验证：`xun serve` 启动 Dashboard，所有 9 个工作台正常。
- [ ] 手动验证：BookmarksPanel 使用 DataTable 自动列。
- [ ] 手动验证：危险操作弹出 OperationDialog。
- [ ] 手动验证：StatusBar 显示 WS 连接状态。

### 5.6 文档同步

- [ ] 更新 `README.md` 中的命令示例。
- [ ] 更新 `intro/cli/Commands.md`。
- [ ] 更新 `intro/dashboard/Dashboard-Usage.md`。
- [ ] 标记 CLI-Refactor-Plan.md 状态为"已完成"。

### 5.7 最终提交

- [ ] `cargo clippy` 无警告。
- [ ] `cargo test` 全量通过。
- [ ] `pnpm -C dashboard-ui test` 全量通过。
- [ ] 提交 commit：`feat: Operation Runtime architecture complete`

---

## 验收标准总表

| 维度 | 标准 | 验证方式 |
|------|------|---------|
| 编译 | `cargo build --release --all-features` 通过 | CI |
| 测试 | 499+ 旧测试 + 新 core 测试全部通过 | `cargo test` |
| 前端测试 | 全部通过 | `pnpm test` |
| 参数零重复 | `grep -r "pub tag: Option<String>" src/cli/` 结果 ≤ 1 | grep |
| 输出统一 | 所有列表命令输出 Table（带 schema） | 手动 `--format json` |
| Operation 统一 | 所有危险操作走 Operation trait | code review |
| Dashboard 打通 | types.ts 100% 自动生成 | diff generated vs manual |
| 性能 | `xun z foo` < 20ms | hyperfine |
| 二进制 | 增加 < 500KB | ls |
| 编译时间 | 增量 < 5s | cargo build --timings |
| 可访问性 | 键盘可达所有 Dashboard 操作 | 手动测试 |

---

## 附录：Commit 规范

```
Phase 0: chore: add clap dependency, create core/ skeleton
Phase 1: feat(core): implement XunError
         feat(core): implement StructuredValue
         feat(core): implement Renderer
         feat(core): implement Args
         feat(core): implement CmdContext
         feat(core): implement CommandSpec
         feat(core): implement Operation
         feat(core): implement ShellIntegration
Phase 2: feat(proxy): migrate to Operation Runtime
Phase 3: feat(config,ctx): migrate to Operation Runtime
         feat(port,proc): migrate and reorganize
         feat(backup): migrate with Operation trait
         feat(bookmark): migrate to Operation Runtime
         feat(env): migrate to Operation Runtime
         feat(acl): migrate with Operation trait
         feat(alias,brn,vault,desktop): migrate remaining
Phase 4: feat(dashboard): type generation with specta
         feat(dashboard): WebSocket command protocol
         feat(dashboard): DataTable + OperationDialog
         feat(dashboard): panel migration
Phase 5: chore: remove argh, unify naming
         perf: benchmark and optimize
         docs: update all documentation
```
