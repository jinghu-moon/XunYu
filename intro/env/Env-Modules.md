# Env 模块导读

本文档专门解释 `xun env` 这一条命令树。相比普通子命令，`env` 更像一个独立产品：它同时拥有 CLI 定义层、执行层、领域层、Web DTO 和可选 TUI。

## 1. `env` 在项目里的位置

从结构上看，`env` 由五层组成：

```text
用户输入: xun env ...
  -> src/cli/env/*               # argh 子命令定义
  -> src/commands/env/cmd/*      # CLI 执行层
  -> src/env_core/*              # 领域与基础设施
  -> src/commands/env/web_dto.rs # Dashboard / API DTO
  -> src/commands/env/tui/*      # 可选 TUI
```

这意味着：

- `env` 不是简单的命令集合，而是一个垂直切分完整的子系统。
- CLI、Dashboard、TUI 三个入口复用的是同一套 `env_core` 领域能力。

## 2. 先看顶层入口

### 2.1 CLI 定义入口：`src/cli/env/mod.rs`

`EnvCmd` 是 `xun env` 的根命令，`EnvSubCommand` 继续往下分成：

- `status`
- `list` / `search` / `get` / `set` / `del`
- `path` / `path-dedup`
- `snapshot`
- `doctor`
- `profile`
- `batch`
- `apply`
- `export` / `export-all` / `export-live` / `env` / `import`
- `diff-live` / `graph` / `validate`
- `schema`
- `annotate`
- `config`
- `audit` / `watch`
- `template` / `run` / `tui`

### 2.2 CLI 执行入口：`src/commands/env/cmd/mod.rs`

执行入口 `cmd_env(args)` 会先创建一个 `EnvManager::new()`，再把子命令转发给对应子模块。

这条链路的重要信号是：

- CLI 执行层不直接操作底层文件；它首先依赖 `EnvManager`。
- 所有命令共享同一套配置、锁、快照、schema、审计和事件系统。

### 2.3 领域入口：`src/env_core/manager.rs`

`EnvManager` 是 `env_core` 的门面对象，当前至少承担两件事：

- 持有 `EnvCoreConfig`
- 作为 CLI / Web / TUI 调用领域能力的统一入口

它的存在说明 `env_core` 被有意抽成领域层，而不是把逻辑散在 CLI 命令文件里。

## 3. 定义层：`src/cli/env/*`

`src/cli/env/` 的组织方式很工整，基本等于“按命令域分文件”。

### 3.1 变量基本操作

- `vars.rs`
  - `list`
  - `search`
  - `get`
  - `set`
  - `del`

这是最基础的一层，面向环境变量的读、查、改、删。

### 3.2 PATH 专项操作

- `path.rs`
  - `path`
  - `path-dedup`
  - `add`
  - `rm`

PATH 被单独拆出来，说明项目刻意把“路径列表操作”和“普通变量操作”区分开。

### 3.3 快照与回滚

- `snapshot.rs`
  - `snapshot create`
  - `snapshot list`
  - `snapshot restore`
  - `snapshot prune`

这组命令让 `env` 具备了状态版本化能力。

### 3.4 诊断、审计与监听

- `doctor.rs`
  - `check`
  - `doctor`
  - `audit`
  - `watch`

这一组已经不是单纯“改变量”，而是在做健康检查、审计追踪和实时观察。

### 3.5 Profile / 模板化环境

- `profile.rs`
  - `profile list`
  - `profile capture`
  - `profile apply`
  - `profile diff`
  - `profile delete`

这说明环境配置不只是即时修改，还被建模成“可保存、可比较、可重放”的 profile。

### 3.6 批处理

- `batch.rs`
  - `batch set`
  - `batch delete`
  - `batch rename`

这一层是典型运维入口，适合脚本和批量调整。

### 3.7 导入导出与合并视图

- `import_export.rs`
  - `apply`
  - `export`
  - `export-all`
  - `export-live`
  - `env`
  - `import`

这里同时覆盖：

- 数据交换
- shell 可消费导出
- live 环境导出
- merged 视图

说明它已经兼顾“人读”和“机器用”两种场景。

### 3.8 差异、依赖图与校验

- `diff_graph.rs`
  - `diff-live`
  - `graph`
  - `validate`

这里聚焦的是“理解当前环境”的能力，而不是直接写值。

### 3.9 Schema 与注释

- `schema.rs`
  - `schema show`
  - `schema add-required`
  - `schema add-regex`
  - `schema add-enum`
  - `schema remove`
  - `schema reset`
- `annotations.rs`
  - `annotate set`
  - `annotate list`

这两组命令说明 `env` 已经不只是 K/V 存储，还开始管理“约束”和“元数据”。

### 3.10 子系统配置与交互运行

- `config.rs`
  - `config show`
  - `config path`
  - `config reset`
  - `config get`
  - `config set`
- `run.rs`
  - `template`
  - `run`
  - `tui`

这一层负责：

- 子系统自身配置
- 模板展开
- 注入环境后运行命令
- 进入可选 TUI

## 4. 执行层：`src/commands/env/cmd/*`

执行层与定义层几乎镜像对齐：

- `vars.rs` -> `cmd_list / cmd_search / cmd_get / cmd_set / cmd_del`
- `path.rs` -> `cmd_path / cmd_path_add / cmd_path_rm / cmd_path_dedup`
- `snapshot.rs` -> `cmd_snapshot_*`
- `doctor.rs` -> `cmd_check / cmd_doctor`
- `profile.rs` -> `cmd_profile_* / cmd_apply`
- `batch.rs` -> `cmd_batch_*`
- `import_export.rs` -> `cmd_export / cmd_export_all / cmd_export_live / cmd_env_merged / cmd_import`
- `diff_graph.rs` -> `cmd_diff_live / cmd_graph`
- `schema.rs` -> `cmd_validate / cmd_schema_*`
- `annotations.rs` -> `cmd_annotate_*`
- `config.rs` -> `cmd_env_config_*`
- `run.rs` -> `cmd_audit / cmd_watch / cmd_template / cmd_run`
- `status.rs` -> `cmd_status`

这种镜像组织有两个好处：

1. CLI 定义文件和执行文件能快速互相定位。
2. 后续继续拆分时，不容易把参数定义和执行逻辑弄乱。

## 5. 领域层：`src/env_core/*`

`env_core` 才是 `env` 子系统真正的核心引擎。从模块名可以看出它覆盖的能力面：

- `config`：子系统配置
- `snapshot`：快照
- `profile`：profile
- `schema`：schema 规则
- `annotations`：注释
- `audit`：审计
- `diff`：差异计算
- `dep_graph`：依赖图
- `doctor`：诊断
- `template`：模板展开
- `watch`：监听
- `batch`：批处理
- `io`：导入导出
- `registry`：注册表交互
- `lock` / `write_guard`：并发写保护
- `ops_*`：按场景拆开的应用服务层

如果用分层思路理解它，大致是：

```text
EnvManager
  -> ops_read / ops_write / ops_snapshot / ops_profile / ops_schema / ops_io / ops_run
  -> 复用 snapshot / profile / schema / diff / doctor / audit / watch / template / registry 等领域模块
```

这说明 `env_core` 的设计目标不是“一组零散函数”，而是一个较完整的领域服务集合。

## 6. `EnvCoreConfig` 管什么

从 `src/env_core/config.rs` 当前实现看，Env 子系统有一份独立配置，核心字段包括：

- `snapshot_dir`
- `profile_dir`
- `max_snapshots`
- `lock_timeout_ms`
- `stale_lock_secs`
- `notify_enabled`
- `allow_run`
- `snapshot_every_secs`

这能解释很多命令为什么存在：

- 有 `snapshot_dir` / `max_snapshots`，所以快照不只是临时文件。
- 有 `allow_run`，所以 `env run` 是被有意识地纳入治理范围的。
- 有 `snapshot_every_secs`，所以 Dashboard 能做自动快照调度。

## 7. Web 与 Dashboard 对应层

`env` 不是只给 CLI 用。它还额外有两层对外接口：

### 7.1 `src/commands/env/web_dto.rs`

这里定义了 Web API 的请求体和响应体，例如：

- `SetVarBody`
- `PathUpdateBody`
- `SnapshotCreateBody`
- `ImportBody`
- `RunBody`
- `SchemaAddRequiredBody`
- `GraphPayload`
- `DoctorPayload`
- `ValidatePayload`

这说明 Dashboard 的 Env 页面不是旁路实现，而是共享了 `env` 的领域模型，只是在 HTTP 边界多了一层 DTO。

### 7.2 `dashboard-ui` 中的 Env 组件族

前端对应的是：

- `EnvPanel.vue`
- `EnvVarsTable.vue`
- `EnvPathEditor.vue`
- `EnvSnapshotsPanel.vue`
- `EnvDoctorPanel.vue`
- `EnvDiffPanel.vue`
- `EnvProfilesPanel.vue`
- `EnvSchemaPanel.vue`
- `EnvAnnotationsPanel.vue`
- `EnvImportExportPanel.vue`
- `EnvTemplateRunPanel.vue`
- `EnvGraphPanel.vue`
- `EnvAuditPanel.vue`
- `EnvVarHistoryDrawer.vue`

也就是说，`env` 是少数真正同时拥有 CLI + Web + 可选 TUI 三种入口的能力域。

## 8. 可选 TUI 层

`src/commands/env/tui/mod.rs` 说明：

- 未开启 `tui` feature 时，`env tui` 会直接报错并提示重新编译。
- 开启 `tui` feature 后，会切到 `imp::run_env_tui`。

因此 `env tui` 不是默认能力，而是特性开关下的增强入口。

## 9. 推荐阅读顺序

### 9.1 想先搞清命令面

1. `src/cli/env/mod.rs`
2. `src/cli/env/*.rs`
3. `src/commands/env/cmd/mod.rs`

### 9.2 想搞清执行链

1. `src/commands/env/cmd/mod.rs`
2. 目标命令对应的 `src/commands/env/cmd/<group>.rs`
3. `src/env_core/manager.rs`
4. 对应的 `src/env_core/*`

### 9.3 想看 Dashboard 与 CLI 如何复用

1. `src/commands/env/web_dto.rs`
2. `src/commands/dashboard/handlers_env/*`
3. `dashboard-ui/src/components/EnvPanel.vue`

## 10. 当前实现上的几个观察

- `env` 是当前项目里最完整、最像“独立子产品”的模块。
- 定义层和执行层镜像对齐，结构非常利于继续拆分和维护。
- `env_core` 已经具备明显的领域边界，不建议把新逻辑直接塞回 CLI 文件。
- `schema / annotation / profile / snapshot / audit / run` 这些能力共存，说明项目对环境变量的定位不是“临时设置值”，而是“可治理的配置资产”。
- Dashboard 的 Env 页面本质上是 `env` 子系统的可视化外壳，而不是另一套平行实现。
