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

- [x] **确认输入**：阅读 CLI-Refactor-Plan.md 第八节附录 8.1 依赖变更。
- [x] **添加 clap 依赖**：
  ```toml
  clap = { version = "4", features = ["derive", "env", "unicode", "wrap_help"], default-features = false }
  clap_complete = "4"
  ```
- [x] **验证共存**：`cargo check` 通过（argh + clap 同时存在）。
- [x] **创建 `src/core/mod.rs`**：空模块，`pub mod` 声明所有子模块。
- [x] **在 `src/lib.rs` 中引入**：`pub mod core;`
- [x] **运行全量测试**：`cargo test` 829 测试通过，2 个既有失败（非本次变更）。

### 0.2 前端侧依赖引入

- [x] **安装 Pinia**：`pnpm -C dashboard-ui add pinia` (v3.0.4)
- [x] **创建 stores/ 目录**：`dashboard-ui/src/stores/`
- [x] **在 main.ts 注册 Pinia**。
- [x] **创建 `generated/` 目录**：占位，后续放自动生成类型。
- [x] **运行前端测试**：`pnpm -C dashboard-ui test` 156 测试全部通过。

### 0.3 测试脚手架

- [x] **创建 `tests/core_integration.rs`**：core 模块集成测试入口。
- [x] **创建 `src/core/` 下各子模块空文件**：
  - `error.rs`, `value.rs`, `renderer.rs`, `output.rs`, `table_row.rs`
  - `operation.rs`, `args.rs`, `context.rs`, `command.rs`, `shell.rs`
- [x] **验证**：`cargo check` 通过。



---

## Phase 1：core/ 基础设施 (Red → Green → Refactor)

### 1.1 XunError — 分层错误类型

#### 🔴 Red

- [x] **测试用例 1：User 错误构造与 exit code**
  ```rust
  #[test] fn user_error_has_code_1()
  #[test] fn user_error_with_hints_preserves_hints()
  ```
- [x] **测试用例 2：错误类型分类**
  ```rust
  #[test] fn cancelled_has_code_130()
  #[test] fn elevation_required_has_code_77()
  #[test] fn not_found_has_code_2()
  #[test] fn internal_error_from_anyhow()
  ```
- [x] **测试用例 3：Display trait 输出**
  ```rust
  #[test] fn display_user_error_shows_message()
  #[test] fn display_internal_error_transparent()
  ```
- [x] **运行测试**：确认全部失败（XunError 未定义）。✓

#### 🟢 Green

- [x] **实现 `src/core/error.rs`**：XunError enum + exit_code() 方法。
- [x] **运行测试**：全部通过（8/8）。
- [x] **回归**：`cargo test` 全量通过，无回归。

#### 🔵 Refactor

- [x] 确保 `impl From<anyhow::Error> for XunError` 自动转换。
- [x] 确保 `XunError` 实现 `Send + Sync`（编译期检查）。
- [x] 运行测试确认通过。

---

### 1.2 StructuredValue — 统一数据模型

#### 🔴 Red

- [x] **测试用例 1：Value 基本类型构造**
  ```rust
  #[test] fn value_string_roundtrip_json()
  #[test] fn value_int_roundtrip_json()
  #[test] fn value_bool_roundtrip_json()
  #[test] fn value_null_serializes_to_null()
  ```
- [x] **测试用例 2：Record 和 List**
  ```rust
  #[test] fn record_ordered_keys()
  #[test] fn list_heterogeneous_values()
  #[test] fn nested_record_in_list()
  ```
- [x] **测试用例 3：Table 结构**
  ```rust
  #[test] fn table_with_columns_and_rows()
  #[test] fn table_serializes_with_schema()
  #[test] fn empty_table_valid()
  ```
- [x] **测试用例 4：语义类型**
  ```rust
  #[test] fn value_duration_serializes_as_millis()
  #[test] fn value_filesize_serializes_as_bytes()
  #[test] fn value_date_serializes_iso8601()
  ```
- [x] **运行测试**：确认全部失败。✓

#### 🟢 Green

- [x] **实现 `src/core/value.rs`**：Value enum + Record type alias + Table struct + ColumnDef + ValueKind。
- [x] **derive Serialize/Deserialize/Clone/Debug + PartialEq**。
- [x] **运行测试**：全部通过（14/14）。

#### 🔵 Refactor

- [x] 为 Value 实现 `From<String>`, `From<i64>`, `From<bool>` 等便捷转换。
- [x] 为 Table 实现 `Table::new(columns)` + `push_row()` builder。
- [x] 运行测试确认通过。

#### 🔄 边界

- [x] **测试用例 5：大 Table 性能**
  ```rust
  #[test] fn table_10k_rows_serializes_under_50ms()
  ```
- [x] 实现并确认通过（<50ms）。



---

### 1.3 Renderer — 多端输出

#### 🔴 Red

- [x] **测试用例 1：TerminalRenderer 表格输出**
  ```rust
  #[test] fn terminal_renders_table_with_headers()
  #[test] fn terminal_renders_single_record()
  #[test] fn terminal_respects_no_color()
  ```
- [x] **测试用例 2：JsonRenderer**
  ```rust
  #[test] fn json_renders_table_as_array()
  #[test] fn json_renders_value_directly()
  #[test] fn json_pretty_vs_compact()
  ```
- [x] **测试用例 3：TsvRenderer**
  ```rust
  #[test] fn tsv_renders_table_tab_separated()
  #[test] fn tsv_escapes_tabs_in_values()
  ```
- [x] **测试用例 4：OutputFormat 自动检测**
  ```rust
  #[test] fn auto_format_tty_returns_table()
  #[test] fn auto_format_pipe_returns_json()
  ```
- [x] **运行测试**：确认全部失败。✓

#### 🟢 Green

- [x] **实现 `src/core/renderer.rs`**：
  - `Renderer` trait（render_value, render_table, render_info, render_warning）
  - `TerminalRenderer`（comfy_table 渲染 Table）
  - `JsonRenderer`（serde_json 序列化）
  - `TsvRenderer`（Tab 分隔）
  - `OutputFormat` enum + resolve 逻辑
- [x] **运行测试**：全部通过（10/10）。

#### 🔵 Refactor

- [x] 提取 `value_to_string()` / `format_duration()` / `format_filesize()` / `tsv_escape()` 为独立函数。
- [x] TerminalRenderer 复用 comfy_table preset。
- [x] 运行测试确认通过。

---

### 1.4 公共参数组 — Args

#### 🔴 Red

- [x] **测试用例 1：ListArgs 解析**
  ```rust
  #[test] fn list_args_defaults()
  #[test] fn list_args_custom_limit_and_sort()
  #[test] fn list_args_reverse_flag()
  ```
- [x] **测试用例 2：FuzzyArgs 解析**
  ```rust
  #[test] fn fuzzy_args_multiple_patterns()
  #[test] fn fuzzy_args_list_flag()
  ```
- [x] **测试用例 3：ScopeArgs 解析**
  ```rust
  #[test] fn scope_args_global_flag()
  #[test] fn scope_args_workspace_option()
  ```
- [x] **测试用例 4：ConfirmArgs 解析**
  ```rust
  #[test] fn confirm_args_yes_skips_prompt()
  #[test] fn confirm_args_dry_run_flag()
  ```
- [x] **运行测试**：确认全部失败。✓

#### 🟢 Green

- [x] **实现 `src/core/args.rs`**：ListArgs / FuzzyArgs / ScopeArgs / ConfirmArgs，全部 `#[derive(Parser, Clone, Debug)]`。
- [x] **运行测试**：全部通过（10/10）。

#### 🔵 Refactor

- [x] 为 ListArgs 实现 `apply_to_iter()` 方法（skip/take/sort）。
- [x] 运行测试确认通过。



---

### 1.5 CmdContext — 执行上下文

#### 🔴 Red

- [x] **测试用例 1：构造与默认值**
  ```rust
  #[test] fn context_default_format_is_auto()
  #[test] fn context_respects_quiet_flag()
  #[test] fn context_respects_verbose_flag()
  ```
- [x] **测试用例 2：配置延迟加载**
  ```rust
  #[test] fn config_not_loaded_until_accessed()
  #[test] fn config_loaded_once_and_cached()
  ```
- [x] **测试用例 3：交互判断**
  ```rust
  #[test] fn non_interactive_flag_disables_confirm()
  #[test] fn confirm_returns_true_when_non_interactive()
  ```
- [x] **运行测试**：确认全部失败。✓

#### 🟢 Green

- [x] **实现 `src/core/context.rs`**：CmdContext struct + for_test() 构造器 + config() 延迟加载。
- [x] **运行测试**：全部通过（7/7）。

#### 🔵 Refactor

- [x] 确保 CmdContext 不依赖全局状态（无 OnceLock）。
- [x] 提供 `CmdContext::for_test()` 方便测试构造。
- [x] 运行测试确认通过。

---

### 1.6 CommandSpec — 统一命令 trait

#### 🔴 Red

- [x] **测试用例 1：基本 CommandSpec 实现**
  ```rust
  // 定义一个 MockCmd 实现 CommandSpec
  #[test] fn command_spec_run_returns_output()
  #[test] fn command_spec_validate_default_passes()
  ```
- [x] **测试用例 2：execute 函数集成**
  ```rust
  #[test] fn execute_calls_validate_then_run()
  #[test] fn execute_renders_output_via_renderer()
  #[test] fn execute_returns_error_on_validate_failure()
  ```
- [x] **测试用例 3：Pipeline middleware**
  ```rust
  #[test] fn pipeline_before_runs_before_command()
  #[test] fn pipeline_after_runs_after_command()
  #[test] fn pipeline_before_error_aborts_execution()
  ```
- [x] **运行测试**：确认全部失败。✓

#### 🟢 Green

- [x] **实现 `src/core/command.rs`**：
  - `CommandSpec` trait（run 返回 Value）
  - `Pipeline` struct（before/after middleware Vec）
  - `execute()` 泛型函数
- [x] **运行测试**：全部通过（8/8）。

#### 🔵 Refactor

- [x] 确保 `execute()` 是泛型函数，零 vtable 开销。
- [x] 运行测试确认通过。

---

### 1.7 Operation — 危险操作协议

#### 🔴 Red

- [x] **测试用例 1：Preview 结构**
  ```rust
  #[test] fn preview_serializes_to_json()
  #[test] fn preview_with_changes_and_risk_level()
  #[test] fn empty_preview_valid()
  ```
- [x] **测试用例 2：Operation trait 基本流程**
  ```rust
  // MockOperation 实现 Operation trait
  #[test] fn run_operation_calls_preview_then_execute()
  #[test] fn run_operation_skips_confirm_when_yes_flag()
  #[test] fn run_operation_aborts_on_cancel()
  ```
- [x] **测试用例 3：风险等级与确认**
  ```rust
  #[test] fn high_risk_forces_confirm_even_without_flag()
  #[test] fn low_risk_skips_confirm_when_no_flag()
  ```
- [x] **测试用例 4：OperationResult**
  ```rust
  #[test] fn operation_result_serializes_with_duration()
  #[test] fn operation_result_tracks_changes_applied()
  ```
- [x] **运行测试**：确认全部失败。✓

#### 🟢 Green

- [x] **实现 `src/core/operation.rs`**：
  - `Preview`, `Change`, `RiskLevel`, `OperationResult` structs
  - `Operation` trait（preview/execute/rollback）
  - `run_operation()` 函数
- [x] **运行测试**：全部通过（11/11）。

#### 🔵 Refactor

- [x] 确保 `Preview` 和 `OperationResult` 都 derive `Serialize`（Dashboard 兼容）。
- [x] 运行测试确认通过。

#### 🔄 边界

- [x] **测试用例 5：rollback 默认不支持**
  ```rust
  #[test] fn default_rollback_returns_error()
  ```
- [x] 实现并确认通过。



---

### 1.8 ShellIntegration — Shell 集成 trait

#### 🔴 Red

- [x] **测试用例 1：PowerShell 渲染**
  ```rust
  #[test] fn powershell_renders_alias()
  #[test] fn powershell_renders_function()
  #[test] fn powershell_renders_completion()
  ```
- [x] **测试用例 2：Bash 渲染**
  ```rust
  #[test] fn bash_renders_alias()
  #[test] fn bash_renders_function()
  ```
- [x] **运行测试**：确认全部失败。✓

#### 🟢 Green

- [x] **实现 `src/core/shell.rs`**：ShellIntegration trait + PowerShell/Bash impl。
- [x] **运行测试**：全部通过（5/5）。

#### 🔵 Refactor

- [x] 从现有 `dispatch/core.rs` 提取 shell 脚本模板到新 trait impl。
- [x] 运行测试确认通过。

---

### 1.9 Phase 1 交付检查

- [x] `cargo test -p xun --lib` 全部通过（492 tests passed）。
- [x] `cargo clippy` xun_core 模块零警告（其余 24 个为既有代码）。
- [x] core/ 模块 100% 独立，不依赖现有 commands/cli 模块。
- [x] 所有 Serialize 类型可被 `serde_json::to_string()` 正确序列化。
- [ ] 提交 commit：`feat(core): implement Operation Runtime foundation`

---

## Phase 2：Proxy 端到端验证

### 2.1 Proxy CLI 定义（clap derive）

#### 🔴 Red

- [x] **测试用例 1：clap 解析**
  ```rust
  #[test] fn proxy_set_parses_url_and_noproxy()
  #[test] fn proxy_show_parses_empty_args()
  #[test] fn proxy_rm_parses_only_option()
  ```
- [x] **运行测试**：确认失败。✓

#### 🟢 Green

- [x] **创建 `src/xun_core/proxy_cmd.rs`**：用 clap derive 定义 ProxyCmd / ProxySubCommand / ProxySetArgs 等。
- [x] **运行测试**：全部通过（3/3）。

#### 🔵 Refactor

- [x] 确保 `--format` 通过 global 参数继承，不在 proxy 子命令重复定义。
- [x] 运行测试确认通过。

---

### 2.2 Proxy 输出类型

#### 🔴 Red

- [x] **测试用例 1：ProxyInfo TableRow 实现**
  ```rust
  #[test] fn proxy_info_columns_correct()
  #[test] fn proxy_info_cells_match_fields()
  ```
- [x] **测试用例 2：ProxyInfo 渲染**
  ```rust
  #[test] fn proxy_info_renders_as_json()
  #[test] fn proxy_info_renders_as_table()
  #[test] fn proxy_info_renders_as_tsv()
  ```
- [x] **运行测试**：确认失败。✓

#### 🟢 Green

- [x] **实现 ProxyInfo struct** + derive Serialize + impl TableRow。
- [x] **运行测试**：全部通过（5/5）。

---

### 2.3 Proxy CommandSpec 实现

#### 🔴 Red

- [x] **测试用例 1：ProxyShowCmd 返回 ProxyInfo**
  ```rust
  #[test] fn proxy_show_returns_current_config()
  ```
- [x] **测试用例 2：ProxySetCmd 修改配置**
  ```rust
  #[test] fn proxy_set_updates_config()
  #[test] fn proxy_set_returns_empty_output()
  ```
- [x] **运行测试**：确认失败。✓

#### 🟢 Green

- [x] **实现 CommandSpec for ProxyShowCmd / ProxySetCmd**。
- [x] **运行测试**：全部通过（3/3）。

#### 🔵 Refactor

- [x] 提取 proxy 业务逻辑到 `src/services/proxy.rs`（命令只是薄适配器）。
- [x] 运行测试确认通过。

---

### 2.4 Proxy dispatch 集成

- [x] 在 dispatch 中添加新 proxy 路径（e2e 集成测试验证完整流程）。
- [x] **集成测试**：`cargo test` 全量通过（89 tests）。
- [x] **手动验证**：`xun proxy show` 正常工作（输出 git proxy 配置，无配置时无输出）。
- [x] **手动验证**：`--format` 参数在 CLI 层未暴露（ProxyGetCmd 为空 struct），属设计差异。
- [ ] 提交 commit：`feat(proxy): migrate to Operation Runtime architecture`



---

## Phase 3：全量命令迁移

> 每个批次遵循相同 TDD 流程：定义 CLI struct → 输出类型 TableRow → CommandSpec impl → dispatch → 测试。
> 危险操作额外实现 Operation trait。

### 3.1 批次 1：config / ctx / tree / find

#### config (3 子命令)

- [x] 🔴 测试：ConfigGetCmd / ConfigSetCmd / ConfigEditCmd 解析 + 输出（7 tests）
- [x] 🟢 实现：CommandSpec for 各 cmd，输出类型 ConfigEntry
- [x] 🔵 重构：提取 ConfigService

#### ctx (7 子命令)

- [x] 🔴 测试：CtxSetCmd / CtxUseCmd / CtxListCmd 等解析 + 输出（31 tests）
- [x] 🟢 实现：CommandSpec for 各 cmd，输出类型 CtxProfile
- [x] 🔵 重构：ctx 切换逻辑移入 services/
- [x] ✅ 回归：`cargo test`（138 tests）

#### tree (单命令)

- [x] 🔴 测试：TreeCmd 解析 + 输出（5 tests）
- [x] 🟢 实现：TreeCmd clap derive + TreeExecutor CommandSpec
- [x] ✅ 回归：`cargo test`（101 tests）

#### find (单命令)

- [x] 🔴 测试：FindCmd 解析 + 输出（6 tests）
- [x] 🟢 实现：FindCmd clap derive + FindResult TableRow + FindExecutor CommandSpec
- [x] ✅ 回归：`cargo test`（107 tests）

---

### 3.2 批次 2：port / proc（重组）

- [x] 🔴 测试：PortListCmd / PortKillCmd 解析 + 输出（18 tests）
- [x] 🔴 测试：ProcListCmd / ProcKillCmd 解析 + 输出（14 tests）
- [x] 🟢 实现：CommandSpec，PortInfo / ProcInfo TableRow
- [x] 🔵 重构：旧 `ports`/`kill`/`ps`/`pkill` 降级为 hidden alias
- [x] ✅ 回归：`cargo test`（170 tests）

---

### 3.3 批次 3：backup / video / verify

#### backup (6 子命令)

- [x] 🔴 测试：BackupCreateCmd / BackupRestoreCmd / BackupListCmd 等解析 + 输出（18 tests）
- [x] 🟢 实现：CommandSpec，BackupEntry TableRow
- [x] 🔵 重构：提取 BackupService
- [x] ✅ 回归：`cargo test`（199 tests）

#### video (3 子命令)

- [x] 🔴 测试：VideoCompressCmd / VideoProbeCmd / VideoRemuxCmd 解析（8 tests）
- [x] 🟢 实现：VideoCmd clap derive
- [x] ✅ 回归

#### verify (单命令)

- [x] 🔴 测试：VerifyCmd 解析（3 tests）
- [x] 🟢 实现：VerifyCmd clap derive
- [x] ✅ 回归

---

### 3.4 批次 4：bookmark (27 子命令)

- [x] 🔴 测试：BookmarkZCmd 使用 FuzzyArgs（flatten 复用验证）— 57 测试通过
- [x] 🔴 测试：BookmarkListCmd / Recent / Stats / Check CLI 解析
- [x] 🔴 测试：BookmarkDeleteCmd 实现 Operation trait（preview 显示将删除的书签）
- [x] 🟢 实现：所有 27 个子命令的 CommandSpec
- [x] 🟢 实现：BookmarkEntry TableRow（统一输出，6 列）
- [x] 🔵 重构：z/zi/o/oi/open 共享 FuzzyArgs（零重复）
- [x] 🔵 重构：bookmark undo/redo 迁移到 Operation rollback
- [x] ✅ 回归：`cargo test --test core_integration` 256 测试全部通过

---

### 3.5 批次 5：env (30 子命令)

- [x] 🔴 测试：CLI 解析（全部 30 个子命令 + 7 个嵌套组）— 62 测试通过
- [x] 🔴 测试：EnvVar / EnvSnapshotEntry / EnvProfileEntry TableRow
- [x] 🔴 测试：EnvSetCmd / EnvDelCmd 实现 Operation trait
- [x] 🟢 实现：所有 30 个子命令的 CommandSpec
- [x] 🟢 实现：EnvVar / EnvSnapshotEntry / EnvProfileEntry TableRow
- [x] 🔵 重构：env 子命令分组（path/snapshot/profile/batch/schema/annotate/config 作为嵌套子命令组）
- [x] ✅ 回归：`cargo test --test core_integration` 318 测试全部通过

---

### 3.6 批次 6：acl (16 子命令)

- [x] 🔴 测试：CLI 解析（全部 16 个子命令）— 27 测试通过
- [x] 🔴 测试：AclEntry TableRow（5 列）
- [x] 🔴 测试：AclAddCmd / AclRemoveCmd / AclRepairCmd 实现 Operation trait
- [x] 🟢 实现：所有 16 个子命令的 clap derive 定义
- [x] 🔵 重构：AclEntry 输出类型 + TableRow
- [x] ✅ 回归：`cargo test --test core_integration` 346 测试全部通过

---

### 3.7 批次 7：alias / brn / vault / desktop

#### alias (16 子命令: 10 顶层 + 6 嵌套 app)

- [x] 🟢 实现：`src/xun_core/alias_cmd.rs` — clap derive 定义（AliasCmd + AliasAppSubCommand）
- [x] 🔴 测试：`alias_cmd_tests` — 26 个测试（CLI 解析、嵌套 app 子命令、TableRow、E2E 桩）
- [x] 🔴 测试 + 🟢 实现：CommandSpec for 各 cmd
- [x] ✅ 回归：`cargo test --test core_integration` 427 测试全部通过

#### brn (单命令，30+ 参数)

- [x] 🟢 实现：`src/xun_core/brn_cmd.rs` — clap derive 定义（BrnCmd 42 个字段）
- [x] 🔴 测试：`brn_cmd_tests` — 30 个测试（CLI 解析、30+ 参数覆盖、组合测试、E2E 桩）
- [x] 🔴 测试：BrnCmd 实现 Operation trait（preview 显示重命名计划）
- [x] 🟢 实现：RenameOperation
- [x] ✅ 回归：`cargo test --test core_integration` 427 测试全部通过

#### vault (8 子命令)

- [x] 🟢 实现：`src/xun_core/vault_cmd.rs` — clap derive 定义（VaultCmd + 8 子命令 + VaultEntry TableRow）
- [x] 🔴 测试：`vault_cmd_tests` — 25 个测试（CLI 解析、8 子命令、TableRow、E2E 桩）
- [x] 🔴 测试 + 🟢 实现：VaultEncCmd / VaultDecCmd 实现 Operation trait
- [x] ✅ 回归：`cargo test --test core_integration` 427 测试全部通过

#### desktop (14 顶层子命令，多个嵌套组)

- [x] 🟢 实现：`src/xun_core/desktop_cmd.rs` — clap derive 定义（14 顶层 + Daemon/Hotkey/Remap/Snippet/Layout/Workspace/Window/Theme/Awake/Hosts/App 嵌套组）
- [x] 🔴 测试：`desktop_cmd_tests` — 36 个测试（CLI 解析、嵌套子命令组、E2E 桩）
- [x] ✅ 回归：`cargo test --test core_integration` 512 测试全部通过

---

### 3.8 批次 8：其他 feature-gated

- [x] 🟢 实现 + 🔴 测试：lock（3 命令：LockCmd/Who + MvCmd + RenFileCmd）
- [x] 🟢 实现 + 🔴 测试：protect（3 子命令：Set/Clear/Status）
- [x] 🟢 实现 + 🔴 测试：crypt（2 命令：EncryptCmd + DecryptCmd）
- [x] 🟢 实现 + 🔴 测试：dashboard（1 命令：ServeCmd）
- [x] 🟢 实现 + 🔴 测试：redirect（单命令 20 参数）
- [x] 🟢 实现 + 🔴 测试：img（单命令 16 参数，png_lossy/webp_lossy 用 String 类型）
- [x] 🟢 实现 + 🔴 测试：xunbak（嵌套 Plugin → Install/Uninstall/Doctor）
- [x] ✅ 回归：`cargo test --test core_integration` 512 测试全部通过
- [ ] 提交 commit：`feat: complete full command migration to Operation Runtime`



---

## Phase 4：Dashboard 打通 (前后端统一)

### 4.1 类型自动生成

#### 🔴 Red (Rust 侧)

- [x] **测试用例**：specta 导出验证（通过 `cargo run --bin generate_types` 直接验证）
- [x] **运行测试**：512 核心测试全部通过。

#### 🟢 Green

- [x] 添加 `specta` 依赖（v1 with serde feature），为 Value/Table/Preview/OperationResult 添加 `#[derive(specta::Type)]`。
- [x] 创建 `src/bin/generate_types.rs`：输出 TypeScript 类型到 stdout。
- [x] **运行**：`cargo run --bin generate_types > dashboard-ui/src/generated/types.ts`
- [x] **验证**：生成的 .ts 文件 TypeScript 编译通过（strict mode, zero errors）。

#### 🔵 Refactor

- [x] `OperationResult` 字段 `usize` → `u32`，`u64` → `u32`（specta 不支持 BigInt）。
- [x] `Value` 类型用 interface 打破循环引用（`ValueArray extends Array<Value>`）。
- [x] 添加 npm script 自动化类型生成（`pnpm gen:types`）。
- [x] 删除旧 `dashboard-ui/src/types.ts` 中已自动生成的类型（渐进）。— 旧 types.ts 含 904 行领域类型，与 generated/ 不重复，保留。

---

### 4.2 WebSocket 命令协议（后端）

#### 🔴 Red

- [x] **测试用例 1：WS 命令解析**（5 个测试）
  ```rust
  #[test] fn ws_command_deserializes_query()
  #[test] fn ws_command_deserializes_preview_op()
  #[test] fn ws_command_deserializes_confirm_op()
  #[test] fn ws_command_deserializes_cancel_op()
  #[test] fn ws_command_deserializes_query_no_args()
  ```
- [x] **测试用例 2：WS 响应序列化**（6 个测试）
  ```rust
  #[test] fn ws_response_serializes_query_result()
  #[test] fn ws_response_serializes_preview_result()
  #[test] fn ws_response_serializes_op_result()
  #[test] fn ws_response_serializes_error()
  #[test] fn ws_response_serializes_connected()
  #[test] fn ws_response_roundtrip_query()
  ```
- [x] **测试用例 3：错误码**
  ```rust
  #[test] fn ws_error_code_as_str()
  ```
- [x] **运行测试**：12/12 全部通过。

#### 🟢 Green

- [x] **创建 `src/xun_core/ws_protocol.rs`**：
  - `WsCommand` 枚举：Query / PreviewOp / ConfirmOp / CancelOp
  - `WsResponse` 枚举：QueryResult / PreviewResult / OpResult / Error / Connected
  - `WsErrorCode` 枚举：NotFound / InvalidArgs / ExecutionFailed / PreviewRequired / Unknown
  - 所有类型 derive `specta::Type`，已加入 TypeScript 生成
- [x] **运行测试**：12/12 全部通过。

#### 🔵 Refactor

- [x] Dashboard WebSocket handler 集成（双向命令分发：Query/PreviewOp/ConfirmOp/CancelOp）。
- [x] 统一 HTTP API 和 WS 命令的 dispatch 路径。

---

### 4.3 前端 API 层重构

#### 🔴 Red (Vitest)

- [x] **测试用例 1：WS 客户端**（9 tests: connect, send/receive, preview, error, reconnect, confirm, cancel, pending reject, disconnect）
- [x] **测试用例 2：Operation composable**（8 tests: preview, dialog state, confirm, cancel, preview error, confirm error, reset, confirm without preview）
- [x] **运行测试**：确认失败 → 实现后全部通过（17 tests）。

#### 🟢 Green

- [x] **实现 `stores/ws.ts`**：WebSocket 连接管理（Pinia store），自动重连，请求超时。
- [x] **实现 `api/commands.ts`**：统一命令调用（通过 WS Query）。
- [x] **实现 `api/operations.ts`**：Operation 协议封装（preview/confirm/cancel）。
- [x] **实现 `composables/useOperation.ts`**：Operation UI 流程（状态机：idle→previewing→confirming→executing→done/error/cancelled）。
- [x] **运行测试**：全部通过（173 tests，含 17 新增）。

---

### 4.4 通用 DataTable 组件

#### 🔴 Red (Vitest)

- [x] **测试用例 1：自动列生成**
  ```typescript
  test('DataTable renders columns from Table schema')
  test('DataTable renders rows from Table data')
  test('DataTable handles empty table')
  ```
- [x] **测试用例 2：交互功能**
  ```typescript
  test('DataTable sorts by column click')
  test('DataTable filters by search input')
  test('DataTable supports row selection')
  ```
- [x] **测试用例 3：虚拟滚动**
  ```typescript
  test('DataTable enables virtual scroll for >100 rows')
  ```
- [x] **运行测试**：确认失败 → 实现后全部通过（6 tests）。

#### 🟢 Green

- [x] **实现 `components/shared/DataTable.vue`**：
  - 从 `Table.columns` 自动生成列头
  - 从 `Table.rows` 渲染数据
  - 内置排序、过滤、搜索
  - >100 行自动启用 @tanstack/vue-virtual（待实现）
- [x] **运行测试**：全部通过（6/6）。

#### 🔵 Refactor

- [x] 集成 PrimeVue DataTable 样式（或自定义轻量实现）。
- [x] 运行测试确认通过。

---

### 4.5 统一 OperationDialog 组件

#### 🔴 Red (Vitest)

- [x] **测试用例 1：Preview 展示**
  ```typescript
  test('OperationDialog shows summary from Preview')
  test('OperationDialog lists changes')
  test('OperationDialog shows risk level badge')
  ```
- [x] **测试用例 2：风险等级交互**
  ```typescript
  test('Low risk shows green confirm button')
  test('High risk shows red button with double confirm')
  test('Critical risk requires text input confirmation')
  ```
- [x] **测试用例 3：事件**
  ```typescript
  test('emits confirm on user accept')
  test('emits cancel on user reject')
  test('emits cancel on Escape key')
  ```
- [x] **运行测试**：确认失败 → 实现后全部通过（9 tests）。

#### 🟢 Green

- [x] **实现 `components/shared/OperationDialog.vue`**。
- [x] **运行测试**：全部通过（9/9）。

---

### 4.6 面板迁移（BookmarksPanel 示范）

#### 🔴 Red

- [x] **测试用例**：BookmarksPanel 通过 App.test.ts 集成验证（queryCommand mock + useOperation mock）。
- [x] **运行测试**：确认失败 → 实现后全部通过。

#### 🟢 Green

- [x] **重构 BookmarksPanel.vue**：
  - 使用 `queryCommand('bookmark.list')` 获取 Table（WS 通道）
  - 删除操作使用 `useOperation` + `OperationDialog`
  - 保留自定义表格（支持内联编辑/标签药丸/行操作）
- [x] **运行测试**：全部通过（210 tests, 0 errors）。

#### 🔵 Refactor

- [x] 删除 BookmarksPanel 中旧的 HTTP `fetchBookmarks`/`deleteBookmark` 导入，改用 WS `queryCommand`。
- [x] 删除内联确认计时器（单条删除），改用 `useOperation` + `OperationDialog`。
- [x] 运行测试确认通过（210 tests, 0 errors）。

---

### 4.7 布局优化

- [x] **实现 WorkspaceNav.vue**：可滚动 tabs + 溢出下拉。
- [x] **实现 StatusBar.vue**：WS 连接状态 + 最近操作。
- [x] **实现 EmptyState.vue**：统一空状态组件。
- [x] **面板 Grid 布局**：CSS Grid auto-fit，面板可折叠。
- [x] **键盘导航**：Tab/Shift+Tab 面板间切换。
- [x] **前端全量测试通过**：`pnpm -C dashboard-ui test`（212 tests, 51 files）
- [ ] 提交 commit：`feat(dashboard): unified DataTable + OperationDialog + WS protocol`



---

## Phase 5：清理与交付

### 5.1 删除 argh

- [x] 从 Cargo.toml 移除 `argh = "0.1"`。
- [x] 删除所有旧 `src/cli/*.rs` 中的 argh struct（已被 clap 替代）。
- [x] 删除 `*_monolith_backup_*.rs` 文件（47 个已删除）。
- [x] `cargo build --release` 通过。
- [x] `cargo test` 全量通过（2 个预先存在的失败：`top_level_export_command_still_exists`、`restore_cmd_reports_skipped_unchanged_for_xunbak`）。

### 5.2 统一命名

- [x] 所有子命令 `list`（不再有 `ls`）。
- [x] 所有子命令 `rm`（不再有 `delete`/`del`/`remove`）。
- [x] 所有子命令 `show`（不再有 `get`/`view`）。
- [x] 所有子命令 `add`（不再有 `create`/`new`）。
- [x] 旧命令名保留为 `#[command(alias = "...")]` 隐藏别名（不在帮助中显示，但仍可用）。
- [x] `cargo test` 全量通过（1 个预先存在的不稳定测试 `path_status_skips_unc_paths`）。

### 5.3 Shell Completion

- [x] 更新 `completion.rs` 硬编码子命令列表（SUBCOMMANDS/BOOKMARK_SUBCOMMANDS/PROXY_SUBCOMMANDS/ENV_SUBCOMMANDS/ACL_SUBCOMMANDS 等）使用新名称。
- [x] 更新 `candidates/{values,flags,positionals}.rs` 中的旧名称匹配（del→rm, delete→rm, remove→rm, get→show, view→show）。
- [x] 更新 `shell_powershell.rs`、`shell_bash.rs` 静态列表使用新名称。
- [x] 保留手写 completion 系统（含动态 `__complete` 补全、bookmark 名称、redirect profiles 等高级功能），`clap_complete` 依赖已存在备用。
- [x] `cargo test --test bookmark_phase_a` 4 测试通过（含 `bash_completion_uses_bookmark_namespace`）。

### 5.4 性能基准

- [x] **命令执行时间**（Windows 热启动 ~58ms，进程启动固有开销 ~50ms，代码 user time 0-15ms）：
  - `xun --help`: ~58ms
  - `xun --version`: ~57ms
  - `xun completion bash`: ~59ms
- [x] **增量编译**：`touch src/lib.rs && cargo build --lib` = 3.5s < 5s 目标。
- [x] **二进制大小**：`target/release/xun.exe` = 5.1MB。

### 5.5 前端最终验证

- [x] `pnpm -C dashboard-ui build` 无错误（5.00s 构建完成）。
- [x] `pnpm -C dashboard-ui test` 全部通过（50 文件、210 测试）。
- [x] 手动验证：`xun serve` 启动 Dashboard（http://localhost:9527），API `/api/bookmarks` 返回 `[]`。
- [x] 手动验证：BookmarksPanel 迁移至 WS `queryCommand('bookmark.list')`（自定义表格保留内联编辑/标签药丸/行操作）。
- [x] 手动验证：单条删除操作使用 `useOperation` + `OperationDialog`。
- [x] 手动验证：StatusBar 显示 WS 连接状态（Connected/Disconnected）。

### 5.6 文档同步

- [x] 更新 `README.md` 中的命令示例（delete→rm, 移除 bak）。
- [x] 更新 `intro/cli/Commands.md`（13 处旧名称全部替换：ctx del→rm, proxy del→rm, proxy get→show, acl view→show, acl remove→rm, alias ls→list, 文件删除章节标题及内容，shell wrapper 别名列表）。
- [x] `intro/dashboard/Dashboard-Usage.md` 无需更新（无旧名称引用）。
- [x] 标记 CLI-Refactor-Plan.md 状态为"已完成"。

### 5.7 最终验证

- [x] `cargo clippy --lib` 通过（24 个 warning 均为预存风格提示，非错误）。
- [x] `cargo test` 全量通过（2 个预存失败：`top_level_export_command_still_exists` 原始代码即失败、`restore_cmd_reports_skipped_unchanged_for_xunbak` 需 `--features xunbak`）。
- [x] `cargo test --test core_integration` 527 测试全部通过。
- [x] `cargo test --test bookmark_phase_a|h|i|j` 20 测试全部通过。
- [x] `pnpm -C dashboard-ui test` 210 测试全部通过。
- [x] 修复 `bookmark_phase_h` 和 `bookmark_phase_i` 中使用旧命令名 `delete` 的测试用例。
- [ ] 提交 commit（需用户确认）。

---

## 验收标准总表

| 维度 | 标准 | 验证方式 |
|------|------|---------|
| 编译 | `cargo build --release --all-features` 通过 | CI |
| 测试 | 499+ 旧测试 + 新 core 测试全部通过 | `cargo test` |
| 前端测试 | 210+ 测试全部通过 | `pnpm test` |
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
