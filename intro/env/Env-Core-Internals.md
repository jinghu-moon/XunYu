# Env Core 内部结构导读

`env` 是这个项目里最像“子系统”的一块能力。CLI、Web API、Dashboard 看到的只是入口，真正的内核主要在 `src/env_core/`。

这篇文档不重复讲 `xun env` 怎么用，而是回答三个更关键的问题：

1. `env_core` 的公开能力是怎么组织出来的？
2. 写操作为什么能统一挂上锁、审计、事件和快照？
3. CLI / Web / TUI 又是怎么套在这个内核之上的？

## 1. 总体定位

从模块结构看，`src/env_core/mod.rs` 做了两件事：

- 对外公开真正稳定的子模块：`annotations`、`audit`、`config`、`dep_graph`、`diff`、`doctor`、`io`、`lock`、`notifier`、`profile`、`registry`、`schema`、`snapshot`、`template`、`types`、`uac`、`watch` 等
- 把 `manager`、`ops_*`、`write_guard` 收成内部实现细节，再统一导出 `EnvManager`

这意味着 `EnvManager` 才是 `env_core` 最核心的门面对象，而 `ops_*` 是它背后按职责拆开的实现片段。

## 2. 类型层：先把语义模型立住

`src/env_core/types/` 是整个子系统的数据字典。这里的拆分非常清晰。

### 2.1 `core.rs`

这里放的是 Env 子系统最基础的概念：

- `EnvResult<T>`：统一结果类型
- `EnvScope`：`user` / `system` / `all`
- `EnvVar`：环境变量实体
- `EnvVarKind`：变量值类型推断结果
- `ExportFormat` / `LiveExportFormat` / `ShellExportFormat`
- `ImportStrategy`、`ParsedImport`、`ImportApplyResult`
- `BatchResult`
- `EnvStatusSummary`
- `EnvDepTree`
- `EnvError`

可以把它理解为“任何 env 功能都会碰到的核心模型”。

### 2.2 `runtime.rs`

这里偏运行时行为与事件：

- `TemplateValidationReport`
- `TemplateExpandResult`
- `RunCommandResult`
- `EnvEventType`
- `EnvEvent`
- `EnvAuditEntry`
- `EnvWatchEvent`

这说明 `env_core` 不只是“变量读写器”，而是已经开始处理模板展开、命令运行、事件广播和审计记录。

### 2.3 `snapshot.rs`

这里定义快照相关模型：

- `SnapshotEntry`
- `Snapshot`
- `SnapshotMeta`

快照被当成一等能力，而不是某个命令顺手写个备份文件。

### 2.4 `profile.rs`

这里定义 profile 模型：

- `EnvProfile`
- `EnvProfileMeta`

说明 profile 是对一组环境变量状态的持久化表达，不只是 CLI 视角里的一个小功能。

### 2.5 `schema.rs`

这里放约束层模型：

- `EnvSchema`
- `SchemaRule`
- `SchemaViolation`
- `ValidationReport`
- `AnnotationEntry`

也就是说，`env_core` 不是只关心“有没有变量”，还关心“变量是否满足规则”。

### 2.6 `diff.rs`

这里放差异表达：

- `DiffChangeKind`
- `PathSegmentDiff`
- `DiffEntry`
- `EnvDiff`

无论是快照对比、profile 对比，还是 live diff，最后都能落回这一套模型。

### 2.7 `doctor.rs`

这里放诊断与修复结果：

- `DoctorIssueKind`
- `DoctorIssue`
- `DoctorReport`
- `DoctorFixResult`

这让 `doctor` 能独立地被 CLI、Web 和 Dashboard 消费，而不是只能输出文本。

## 3. `EnvManager`：薄门面 + 操作切片

`EnvManager` 的实现并没有堆在一个巨型文件里，而是拆成 7 组 `ops_*`。这是一种很典型的“门面 + 按能力分面”的组织方式。

## 3.1 `ops_read.rs`

读路径相关能力主要在这里：

- `list_vars`
- `search_vars`
- `get_var`
- `status_overview`
- `env_config_path`
- `env_config_show`
- `env_config_get`
- `watch_diff`
- `path_entries`
- `doctor_run`
- `check_run`

这组方法有个特点：**只做查询和汇总，不触发写门禁。**

## 3.2 `ops_write.rs`

写路径相关能力集中在这里：

- `env_config_set`
- `env_config_reset`
- `set_var`
- `delete_var`
- `path_add`
- `path_remove`
- `doctor_fix`
- `batch_set`
- `batch_delete`
- `batch_rename`
- `path_dedup`

这组方法是所有“会改系统状态”的核心入口。

## 3.3 `ops_snapshot.rs`

快照能力单独一组：

- `snapshot_create`
- `snapshot_list`
- `snapshot_prune`
- `snapshot_restore`
- `diff_live`
- `diff_since`

这说明快照不是附属功能，而是和普通读写并列的一条主线。

## 3.4 `ops_profile.rs`

profile 能力也独立成组：

- `profile_list`
- `profile_capture`
- `profile_delete`
- `profile_apply`
- `profile_diff`

profile 在架构上和 snapshot 很像，但语义不同：

- snapshot 更偏“时间点备份”
- profile 更偏“命名状态模板”

## 3.5 `ops_schema.rs`

规则、注解、审计查询相关能力集中在这里：

- `validate_schema`
- `schema_show`
- `schema_add_required`
- `schema_add_regex`
- `schema_add_enum`
- `schema_remove`
- `schema_reset`
- `annotate_set`
- `annotate_list`
- `annotate_get`
- `annotate_delete`
- `audit_list`

这里把“结构化约束”和“辅助说明信息”并排管理，说明 schema 并不只是校验器，还承担了一部分治理能力。

## 3.6 `ops_run.rs`

运行时相关能力集中在这里：

- `template_expand`
- `template_validate`
- `runtime_env`
- `render_shell_exports`
- `export_live`
- `merged_env_pairs`
- `notify_run_result`
- `dependency_tree`
- `run_command`

这一组非常关键，因为它把 `env_core` 从“配置管理器”推进成了“运行时上下文生成器”。

## 3.7 `ops_io.rs`

导入导出收口在这里：

- `export_vars`
- `export_bundle`
- `import_file`
- `import_content`

这样做的好处是：CLI、Web、Dashboard 不需要知道导入解析和应用细节，只要走 `EnvManager` 门面即可。

## 4. 写门禁：`with_write_guard()` 是真正的中枢

`src/env_core/write_guard.rs` 是整个 `env_core` 最关键的内部基础设施之一。

它提供的 `with_write_guard(scope, action, snapshot_before, op)`，把几乎所有写操作都串到了同一个流程里：

1. 校验 scope 是否允许写
2. 根据 scope 判断是否需要 UAC / 提权能力
3. 通过 `lock::try_with_lock(...)` 拿文件锁
4. 如果要求 `snapshot_before`，则先创建一次预快照
5. 执行真实写操作 `op`
6. 成功 / 失败都写入 audit
7. 必要时发出 `EnvEvent`

这个设计的价值很大：

- 写操作不会各自忘记加锁
- 审计不会散落在每个命令实现里
- 快照前置逻辑不会重复复制
- Web / CLI / TUI 共享同一套写安全策略

从工程角度看，这正是 `env_core` 能保持复杂但不混乱的关键之一。

## 5. 内核侧模块分工

除了 `EnvManager` 和类型层，`env_core` 还拆出了一批单职责模块。

## 5.1 `registry.rs`：和 Windows 环境变量真实存储打交道

`registry.rs` 是最底层的平台适配模块之一。它做的事情包括：

- 区分 `USER_ENV_SUBKEY` 与 `SYSTEM_ENV_SUBKEY`
- 读取和写入 Windows Registry 中的环境变量
- 处理 `REG_SZ` / `REG_EXPAND_SZ`
- 对 PATH 或含 `%...%` 的值做扩展字符串策略处理
- 在写后广播 `WM_SETTINGCHANGE`

也就是说，`env_core` 的“真实数据源”主要就在这里。

## 5.2 `config.rs`

这个模块负责 `env_core` 自身工作目录、配置文件和相关路径约定。它决定了快照、schema、annotation、profile、audit 等数据应该存在哪里。

## 5.3 `snapshot.rs`

这里负责快照文件的真正创建、读取、枚举、恢复等底层逻辑。`ops_snapshot.rs` 只是门面，真正的快照持久化在这一层完成。

## 5.4 `profile.rs`

这里负责 profile 文件的保存、加载、删除、capture 以及转为环境变量对等结构。

## 5.5 `schema.rs`

这里负责：

- schema 文件路径解析
- schema 读取与保存
- required / regex / enum 规则增删改
- 规则校验执行

也就是说，`ops_schema.rs` 只是提供 API 语义，真正规则落地由这里完成。

## 5.6 `template.rs`

这里负责模板展开和运行时导出：

- `template_expand`
- `template_validate`
- `build_runtime_env`
- `render_shell_exports`
- `render_live_export`

它把“变量集合”转成“可运行上下文”，是 `env_core` 非常偏 runtime 的一层。

## 5.7 `diff.rs`

这里提供 `diff_var_lists`、`diff_maps` 和 `format_diff`，把多个来源的变量差异归并成统一表达。

## 5.8 `dep_graph.rs`

这里通过 `build_tree` 构建变量依赖树，用来解释模板引用与变量依赖关系，而不是只告诉你“值不对”。

## 5.9 `doctor.rs`

这里负责：

- 诊断 PATH 缺失、重复、遮蔽等问题
- 生成 `DoctorReport`
- 尝试自动修复并返回 `DoctorFixResult`
- 文本化报告和退出码计算

## 5.10 `annotations.rs`

这个模块把“变量说明”从变量值本身里分离出来，适合承载注释、提示、备注这类治理信息。

## 5.11 `audit.rs`

这个模块把写操作历史落成持久化审计记录，提供追加和读取能力。

## 5.12 `watch.rs`

这里负责对比前后变量列表，生成 watch / refresh 类事件，是“实时刷新”和“差异通知”语义的基础层。

## 5.13 `notifier.rs`

这里把运行结果转成通知输出，属于运行态反馈的补充层。

## 5.14 `io/*`

`src/env_core/io/` 继续把导入导出拆细：

- `import_parse.rs`：负责解析外部输入
- `import_apply.rs`：负责把解析结果应用到当前环境
- `export_render.rs`：负责把当前状态渲染成目标导出格式

这套拆法很干净：**解析、应用、渲染三件事各司其职。**

## 6. 一条完整链路怎么走

从调用链看，可以把 `env_core` 理解成下面这套流转：

### 6.1 读链路

- 入口通过 `EnvManager::list_vars/get_var/status_overview/...`
- 门面调用 `registry`、`doctor`、`schema`、`snapshot`、`profile` 等模块
- 返回统一的类型层对象
- CLI / Web / Dashboard 再各自决定怎么显示

### 6.2 写链路

- 入口通过 `EnvManager::set_var/path_add/profile_apply/...`
- 先进入 `with_write_guard()`
- 加锁 / 提权校验 / 可选预快照 / 审计 / 发事件
- 再调用 `registry`、`profile`、`snapshot`、`schema` 等底层模块完成真正修改

### 6.3 运行链路

- `template` 负责展开模板和构建 runtime env
- `dep_graph` 负责依赖解释
- `ops_run` 负责把这些能力组合成可执行命令和导出结果
- `notifier` 负责运行后的反馈

## 7. 上层适配：CLI / Web / TUI 怎么接进来

`env_core` 不是直接面向用户，而是被多个适配层消费。

### 7.1 CLI：`src/commands/env/cmd/`

这里负责把命令行参数翻译成 `EnvManager` 的操作调用，再把结果格式化成 CLI 输出。

### 7.2 Web DTO：`src/commands/env/web_dto.rs`

这个文件定义了 Web 层的数据输入输出边界，例如：

- 查询与命令输入：`ScopeQuery`、`SetVarBody`、`PathUpdateBody`、`SnapshotCreateBody`、`DoctorBody`、`ImportBody`、`GraphQuery`、`RunBody` 等
- 响应载荷：`VarsPayload`、`SnapshotPayload`、`DoctorPayload`、`ImportPayload`、`DiffPayload`、`AuditPayload`、`ProfilesPayload`、`SchemaPayload`、`TemplatePayload`、`RunPayload`、`StatusPayload` 等

这层的意义是：**Web 不直接暴露内核类型，而是通过 DTO 做协议边界。**

### 7.3 TUI：`src/commands/env/tui/`

这里负责交互式界面层。它和 CLI / Web 一样，不应该重新实现 env 逻辑，而是复用 `env_core` 的门面能力。

## 8. 为什么这个结构重要

`env_core` 之所以值得单独写一篇文档，是因为它已经明显不是“几个 env 命令的集合”，而是一个完整子系统：

- 有稳定门面：`EnvManager`
- 有统一语义模型：`types/*`
- 有统一写门禁：`write_guard.rs`
- 有真实平台适配：`registry.rs`
- 有治理能力：`snapshot`、`profile`、`schema`、`doctor`、`audit`
- 有运行态能力：`template`、`dep_graph`、`run_command`
- 有多端适配：CLI / Web / TUI

这也是为什么 `env` 能同时支撑命令行操作、Dashboard 面板和实时更新，而不是一改需求就把代码撕裂开。

## 9. 推荐阅读顺序

如果你已经看过 `./Env-Modules.md`，建议继续这样往下钻：

1. `src/env_core/mod.rs`：先看模块边界和 `EnvManager` 出口
2. `src/env_core/types/`：把核心模型先建立起来
3. `src/env_core/ops_read.rs`、`ops_write.rs`：理解门面的两条主路径
4. `src/env_core/write_guard.rs`：看写安全与审计中枢
5. `src/env_core/registry.rs`：看平台真实读写层
6. `src/env_core/snapshot.rs`、`profile.rs`、`schema.rs`、`doctor.rs`：补齐治理能力
7. `src/env_core/template.rs`、`dep_graph.rs`：最后再理解运行态能力

这样你再回头看 CLI 或 Dashboard 的 env 面板时，就能很清楚地知道：它们只是 `env_core` 的不同适配壳。


