# XunYu 大文件拆分实施说明（Refactor Split Plan）

## 1. 目标与边界

本方案用于拆分以下 6 组超大文件，目标是降低耦合、提升可维护性，同时保持行为不变：

1. `src/commands/dashboard/handlers.rs`
2. `src/commands/env/cmd.rs`
3. `src/commands/env/tui.rs`
4. `src/env_core/mod.rs`
5. `src/commands/dashboard/handlers_env.rs`
6. `src/commands/acl.rs`

强约束：

1. 本轮只做结构重构，不改业务逻辑和外部契约。
2. Env 业务逻辑仍必须位于 `src/env_core/*`，CLI/TUI/Web 仅适配。
3. 每个子阶段都要通过编译和基础回归后再进入下一个子阶段。

---

## 2. 通用拆分规则（所有目标文件适用）

1. 迁移原则：
- 先复制函数到新模块并接线，再删除旧实现。
- 函数签名、返回值、错误语义保持不变。
- 路由路径、CLI 命令字、JSON 字段保持不变。

2. 可见性原则：
- 默认用 `pub(super)`，避免扩大模块暴露面。
- 仅在确有跨模块需要时使用 `pub(crate)`。

3. 公共代码原则：
- 重复 helper 必须下沉到 `common.rs`（或模块级 `utils.rs`）。
- 禁止拆分后复制同一段解析/错误映射代码。

4. 验收门禁（每个拆分目标完成后执行）：
- `cargo check --all-features`
- 涉及 dashboard 接口时：`pnpm -C dashboard-ui build`
- 涉及 env_core 时：`cargo test env_core -- --nocapture`

---

## 3. 拆分目标一：`src/commands/dashboard/handlers.rs`

### 3.1 当前文件功能说明

该文件当前承载了 Dashboard 的多资源处理逻辑，职责混杂，主要包含：

1. Bookmarks 资源：
- 列表、导入导出、增删改、重命名、批量操作。

2. Ports 资源：
- 端口列表、按端口/进程 kill、图标读取与缓存。

3. Proxy 资源：
- 代理状态读取、配置读写、设置/删除、连通性测试。

4. Config 资源：
- 全局配置 PATCH/PUT/GET、字段解析与校验 helper。

5. Audit 资源：
- 审计日志读取、tail、统计、CSV 处理。

6. Redirect 资源：
- dry-run、profile 列表、upsert、delete、校验。

7. 文件工作台资源：
- 文件树列举、全文搜索、diff 元信息、内容读取、格式转换、校验。

8. WS 通道：
- WebSocket 连接入口与消息转发循环。

### 3.2 目标结构

```text
src/commands/dashboard/handlers/
├── mod.rs
├── bookmarks.rs
├── ports.rs
├── proxy.rs
├── config.rs
├── audit.rs
├── files.rs
└── ws.rs
```

### 3.3 各新文件职责

1. `bookmarks.rs`：
- 仅包含书签相关 handler 与 DTO 适配。

2. `ports.rs`：
- 端口管理和图标相关逻辑（含缓存 helper）。

3. `proxy.rs`：
- 代理状态、配置、设置与测试。

4. `config.rs`：
- 全局配置读写与字段解析 helper。
- redirect profile 的配置类接口可先临时放在本文件（后续可独立 `redirect.rs`）。

5. `audit.rs`：
- 审计日志读取、tail、统计、CSV escape。

6. `files.rs`：
- 文件列举/搜索/内容读取/diff 信息/格式转换/文件校验。

7. `ws.rs`：
- dashboard WebSocket 入口与 session 循环。

8. `mod.rs`：
- 仅导出以上子模块 API。
- 放共享 helper（如响应 header 插入）及共享类型别名（若确有必要）。

### 3.4 迁移顺序

1. 先迁移 `bookmarks.rs`、`ports.rs`、`proxy.rs`（低耦合）。  
2. 再迁移 `config.rs`、`audit.rs`（中耦合）。  
3. 最后迁移 `files.rs`、`ws.rs`（函数量最大，耦合最多）。  
4. `src/commands/dashboard/mod.rs` 改为从 `handlers::...` 调用。

### 3.5 验收

1. `cargo check --all-features`
2. `pnpm -C dashboard-ui build`

---

## 4. 拆分目标二：`src/commands/env/cmd.rs`

### 4.1 当前文件功能说明

该文件同时承担：

1. CLI 子命令分发（`cmd_env` 主 `match`）。
2. 具体命令执行逻辑：
- status/list/search/get/set/del
- path/path-dedup
- snapshot create/list/restore/prune
- doctor/check
- profile capture/apply/diff/delete
- batch set/delete/rename
- export/export-all/export-live/env
- import
- diff-live/graph
- schema/validate
- annotate
- config
- audit/watch
- template/run
3. 公共解析与交互：
- scope/format/key-value 解析
- confirm 提示
- EnvError 到 CliError 映射

### 4.2 目标结构

```text
src/commands/env/cmd/
├── mod.rs
├── status.rs
├── vars.rs
├── path.rs
├── snapshot.rs
├── doctor.rs
├── profile.rs
├── batch.rs
├── import_export.rs
├── diff_graph.rs
├── schema.rs
├── annotations.rs
├── config.rs
└── run.rs
```

> 建议额外增加 `common.rs`（解析/错误映射/confirm），但不强制。

### 4.3 各新文件职责

1. `status.rs`：`status` 命令文本/JSON 输出。
2. `vars.rs`：`list/search/get/set/del`。
3. `path.rs`：`path add/rm` 与 `path-dedup`。
4. `snapshot.rs`：`snapshot create/list/restore/prune`。
5. `doctor.rs`：`doctor/check` 及 fix 输出。
6. `profile.rs`：`profile` 子命令与 `apply`。
7. `batch.rs`：`batch set/delete/rename`。
8. `import_export.rs`：`import/export/export-all/export-live/env`。
9. `diff_graph.rs`：`diff-live` 与 `graph`。
10. `schema.rs`：`validate/schema`。
11. `annotations.rs`：`annotate`。
12. `config.rs`：`env config`。
13. `run.rs`：`template/run/audit/watch`（或将 audit/watch 单独再拆）。
14. `mod.rs`：仅保留 `cmd_env` 分发，不放业务细节。

### 4.4 迁移顺序

1. 先迁移无状态命令（`status`, `vars`, `path`）。  
2. 再迁移 `snapshot`, `doctor`, `profile`, `batch`。  
3. 最后迁移 `import_export`, `diff_graph`, `schema`, `annotations`, `config`, `run`。  
4. 最后收敛公共 helper 到 `common.rs`，删除重复代码。

### 4.5 验收

1. `cargo check --all-features`
2. `xun env --help`
3. 关键冒烟：`list/get/set/snapshot/import/diff-live/graph`

---

## 5. 拆分目标三：`src/commands/env/tui.rs`

### 5.1 当前文件功能说明

该文件当前混合了 TUI 的三层职责：

1. App 状态管理：
- 面板状态、列表状态、选中项、过滤状态、快照/profile 缓存、undo 栈等。

2. 事件循环与按键分发：
- 主循环、快捷键处理、各 panel `handle_*_key`。

3. UI 渲染与交互输入：
- 主布局渲染、多个 panel 渲染、弹窗输入（新增/编辑/导入导出确认）。

### 5.2 目标结构（MVC 风格）

```text
src/commands/env/tui/
├── mod.rs
├── app_state.rs
├── event_loop.rs
├── keymap.rs
├── prompts.rs
└── render/
    ├── mod.rs
    └── panels/
        ├── vars.rs
        ├── path.rs
        ├── snapshots.rs
        ├── doctor.rs
        ├── profiles.rs
        ├── history.rs
        └── io.rs
```

### 5.3 各新文件职责

1. `app_state.rs`：
- `App` 与其状态方法（refresh/filter/selection/current item）。

2. `event_loop.rs`：
- terminal 生命周期、tick、主循环控制。

3. `keymap.rs`：
- 面板切换键、每个 panel 的按键 handler、undo 入口。

4. `prompts.rs`：
- 所有交互输入函数（文本输入、yes/no、导入导出源选择）。

5. `render/mod.rs`：
- 总体布局、panel 分发渲染、共享渲染工具。

6. `render/panels/*.rs`：
- 每个 panel 的渲染和局部格式化逻辑。

### 5.4 迁移顺序

1. 先拆 `app_state.rs`。  
2. 再拆 `prompts.rs`（低风险）。  
3. 再拆 `render/*`。  
4. 最后拆 `event_loop.rs` 与 `keymap.rs`，完成入口收敛。

### 5.5 验收

1. `cargo check --all-features`
2. `xun env tui` 手测基础路径：列表浏览、变量编辑、快照恢复、导入导出

---

## 6. 拆分目标四：`src/env_core/mod.rs`

### 6.1 当前文件功能说明

该文件是 Env 核心门面，当前包含：

1. `EnvManager` 构造与配置读取。
2. 读操作：
- list/search/get/status/template/runtime env/config get/show 等。

3. 写操作：
- set/del/path add/remove/path dedup/batch/import/profile apply/doctor fix 等。

4. 快照与回滚：
- create/list/prune/restore/diff since/live。

5. schema/annotations/audit/watch/run/dependency graph。

6. 内部共享流程：
- `with_write_guard`（lock -> snapshot -> write -> audit/event）。

### 6.2 目标结构

```text
src/env_core/
├── mod.rs
├── manager.rs
├── ops_read.rs
├── ops_write.rs
├── ops_snapshot.rs
├── ops_profile.rs
├── ops_schema.rs
├── ops_io.rs
├── ops_run.rs
└── write_guard.rs
```

### 6.3 各新文件职责

1. `manager.rs`：
- `EnvManager` 结构体、`new/default/config/with_event_callback`。

2. `ops_read.rs`：
- list/search/get/status/template validate/runtime env/config get/show/audit read 等只读方法。

3. `ops_write.rs`：
- set/delete/path/batch/import/doctor fix 等写方法。

4. `ops_snapshot.rs`：
- snapshot create/list/prune/restore/diff since/live。

5. `ops_profile.rs`：
- profile capture/list/apply/diff/delete。

6. `ops_schema.rs`：
- schema add/remove/reset/validate + annotations 增删查改（可视团队习惯再细拆）。

7. `ops_io.rs`：
- export/export bundle/import content/import file 入口。

8. `ops_run.rs`：
- run command、notify、shell export、merged env 输出。

9. `write_guard.rs`：
- `with_write_guard` 相关流程与审计/事件封装。

10. `mod.rs`：
- 只负责 module 声明与 `pub use`。

### 6.4 迁移顺序

1. 先迁移 `manager.rs` 与 `write_guard.rs`。  
2. 再迁移只读 `ops_read.rs`。  
3. 再迁移快照 `ops_snapshot.rs`。  
4. 最后迁移写路径与运行路径（`ops_write/io/run/profile/schema`）。

### 6.5 验收

1. `cargo check --all-features`
2. `cargo test env_core -- --nocapture`
3. Env 冒烟：`status/list/set/snapshot/restore/import/diff-live/run`

---

## 7. 拆分目标五：`src/commands/dashboard/handlers_env.rs`

### 7.1 当前文件功能说明

该文件承载全部 Env Web API 适配逻辑，主要包括：

1. 公共适配器：
- `manager()`、event sender、scope 解析、错误映射、`ApiSuccess/ApiError` 包装。

2. vars/path/snapshot/doctor/import-export/diff/graph 接口。

3. audit/history/profiles/schema/annotations/template/run 接口。

4. Env WebSocket（连接、广播、ping/pong、lag 处理）。

### 7.2 目标结构

```text
src/commands/dashboard/handlers_env/
├── mod.rs
├── common.rs
├── vars.rs
├── snapshot.rs
├── doctor.rs
├── schema.rs
├── profile.rs
├── annotations.rs
├── run.rs
└── ws.rs
```

### 7.3 各新文件职责

1. `common.rs`：
- `manager()`、`resolve_scope()`、`map_env_error()`、`ok()`、`parse_set_pairs()`、`run_api_enabled()`。

2. `vars.rs`：
- `env_ping/env_status/list/get/set/delete/path add/remove`。

3. `snapshot.rs`：
- list/create/prune/restore + diff/graph（可与 vars 拆分后再细分）。

4. `doctor.rs`：
- doctor run/fix + validate。

5. `schema.rs`：
- schema show/add/remove/reset。

6. `profile.rs`：
- list/capture/apply/diff/delete。

7. `annotations.rs`：
- list/get/set/delete + var_history + audit_list（视资源边界可把 audit 单独文件）。

8. `run.rs`：
- import/export/export_all/export_live/template_expand/run_command。

9. `ws.rs`：
- `env_ws` 与 socket loop。

10. `mod.rs`：
- 统一对外导出供 router 使用。

### 7.4 迁移顺序

1. 先抽 `common.rs`。  
2. 再抽 `vars/snapshot/profile/schema`。  
3. 最后抽 `run/annotations/ws`。

### 7.5 验收

1. `cargo check --all-features`
2. `tools/envmgr-dashboard-chain-smoke.ps1 -Port 7073`
3. 手工验证：`GET /api/env/status`、`GET /api/env/ws`

---

## 8. 拆分目标六：`src/commands/acl.rs`

### 8.1 当前文件功能说明

该文件当前混合 ACL 全量命令：

1. 公共运行时配置/审计工具。
2. 查看与编辑命令：
- `view/add/remove/purge/diff/effective/copy/inherit/owner`。

3. 批量与恢复：
- `batch/backup/restore`。

4. 维护与诊断：
- `orphans/repair/audit/config`。

### 8.2 目标结构

```text
src/commands/acl_cmd/
├── mod.rs
├── common.rs
├── view.rs
├── edit.rs
├── batch.rs
├── repair.rs
├── audit.rs
└── config.rs
```

### 8.3 各新文件职责

1. `common.rs`：
- 运行时配置加载、错误映射、交互确认、输出辅助。

2. `view.rs`：
- `view/diff/effective`。

3. `edit.rs`：
- `add/remove/purge/inherit/owner/copy`。

4. `batch.rs`：
- `batch/backup/restore`。

5. `repair.rs`：
- `orphans/repair`。

6. `audit.rs`：
- ACL 审计输出与查询命令。

7. `config.rs`：
- ACL 配置读写命令。

8. `mod.rs`：
- `cmd_acl` 分发，不承载具体业务逻辑。

### 8.4 迁移顺序

1. 先抽 `common.rs`。  
2. 再抽 `view/edit`。  
3. 再抽 `batch/repair`。  
4. 最后抽 `audit/config` 与入口分发。

### 8.5 验收

1. `cargo check --all-features`
2. ACL 常用命令回归：`view/add/remove/batch/repair/config`

---

## 9. 执行顺序建议（跨文件）

建议按风险从低到高执行：

1. `dashboard/handlers.rs`
2. `dashboard/handlers_env.rs`
3. `commands/env/cmd.rs`
4. `commands/env/tui.rs`
5. `commands/acl.rs`
6. `env_core/mod.rs`（最后做，核心影响最大）

---

## 10. 每阶段检查清单（复制即用）

1. [ ] 新增目录与 `mod.rs` 接线完成  
2. [ ] 函数迁移后旧文件无重复实现  
3. [ ] 可见性收敛为 `pub(super)` 优先  
4. [ ] `cargo check --all-features` 通过  
5. [ ] dashboard 改动时 `pnpm -C dashboard-ui build` 通过  
6. [ ] env_core 改动时 `cargo test env_core -- --nocapture` 通过  
7. [ ] 文档与任务清单同步更新

---

## 11. 第二梯队拆分任务（后续任务重点）

以下文件也纳入后续重点，建议在前 6 组完成后按优先级推进。

### 11.1 `src/cli/env.rs`

当前功能：

1. 仅承载 Env CLI 命令树定义，但结构体/枚举过多（`EnvSubCommand` + 多层子命令参数）。
2. 包含 vars/path/snapshot/profile/batch/schema/run/template 等全部参数模型，阅读成本高。

建议目标结构：

```text
src/cli/env/
├── mod.rs
├── status.rs
├── vars.rs
├── path.rs
├── snapshot.rs
├── doctor.rs
├── profile.rs
├── batch.rs
├── import_export.rs
├── diff_graph.rs
├── schema.rs
├── annotations.rs
├── config.rs
└── run.rs
```

说明：

1. `mod.rs` 仅聚合 `EnvCmd` / `EnvSubCommand`。
2. 各参数结构体按命令域下沉，避免单文件滚动查找。

### 11.2 `src/commands/completion/candidates.rs`

当前功能：

1. 管理补全候选的核心策略：静态候选、flags、value flags、positionals。
2. 包含动态补全来源：bookmarks/profiles/ctx profiles/transactions/config keys。
3. 目前 flags 策略与动态数据源耦合在同一文件。

建议目标结构：

```text
src/commands/completion/candidates/
├── mod.rs
├── flags.rs
├── values.rs
├── positionals.rs
├── dynamic_bookmarks.rs
├── dynamic_profiles.rs
└── dynamic_config.rs
```

说明：

1. flags/value/positionals 的纯规则逻辑独立。
2. 动态数据源按来源拆分，降低 feature 条件编译复杂度。

### 11.3 `src/commands/proxy_old.rs`

当前功能：

1. 旧版 proxy 命令完整链路：`on/off/detect/status/exec`。
2. 包含网络探测、地址解析、proxy targets、cargo/msys2 配置读写。
3. 环境变量输出与多系统配置写入逻辑耦合。

建议目标结构：

```text
src/commands/proxy_old/
├── mod.rs
├── detect.rs
├── set_del.rs
├── status.rs
├── exec.rs
├── probe.rs
└── targets.rs
```

说明：

1. 若决定保留旧实现，需先模块化再评估与 `src/commands/proxy/*` 合并。
2. 若决定淘汰，需先补 deprecate 迁移说明后删除。

### 11.4 `src/alias/mod.rs`

当前功能：

1. alias 子命令总分发（setup/add/rm/ls/find/which/sync/export/import）。
2. app alias 子命令（app add/rm/ls/scan/sync）。
3. `AliasCtx` 负责 config 加载/保存与 shim 同步，命令逻辑与上下文耦合。

建议目标结构：

```text
src/alias/
├── mod.rs
├── context.rs
├── shell_alias_cmd.rs
├── app_alias_cmd.rs
├── query.rs
└── sync.rs
```

说明：

1. `AliasCtx` 抽到 `context.rs`。
2. shell/app 两套命令链分离，避免单文件多分支。

### 11.5 `src/alias/shim_gen.rs`

当前功能：

1. shim 生成、删除、全量同步、模板部署。
2. 包含模板发现、原子写入、文件替换、GUI 子系统 patch、PATH 查找。
3. shell alias 与 app alias 的 shim 渲染都在一个文件。

建议目标结构：

```text
src/alias/shim_gen/
├── mod.rs
├── classify.rs
├── render.rs
├── sync.rs
├── template.rs
├── io.rs
└── pe_patch.rs
```

说明：

1. PE/GUI patch 属于高风险逻辑，必须独立并附专门测试。
2. I/O 原子写入与模板选择应与渲染逻辑解耦。

### 11.6 `src/commands/ports.rs`

当前功能：

1. 端口查询（`cmd_ports`）、kill（`cmd_kill`）、进程查询（`cmd_ps`）、批量 kill（`cmd_pkill`）。
2. 输出渲染（ports/processes table）和 kill 执行耦合。
3. 含端口范围解析、端口分类、字符串截断等 helper。

建议目标结构：

```text
src/commands/ports/
├── mod.rs
├── query.rs
├── kill.rs
├── process.rs
├── render.rs
└── common.rs
```

说明：

1. 业务动作（query/kill）与 UI 渲染（table 输出）拆开。
2. helper 下沉 `common.rs` 供子模块复用。

### 11.7 `src/commands/ctx/cmd.rs`

当前功能：

1. ctx 命令分发与实现同文件：`set/list/show/del/rename/use/off`。
2. 每个命令都含校验、配置读写、输出渲染，重复模式明显。

建议目标结构：

```text
src/commands/ctx/
├── cmd/
│   ├── mod.rs
│   ├── set.rs
│   ├── list.rs
│   ├── show.rs
│   ├── delete.rs
│   ├── rename.rs
│   ├── use_ctx.rs
│   └── off.rs
└── common.rs
```

说明：

1. 保留 `cmd_ctx` 为入口，子命令实现下沉。
2. 公共配置读写与格式化函数统一放 `common.rs`。

### 11.8 `src/find/walker/dir_windows.rs`

当前功能：

1. Windows 目录扫描双路径实现：`scan_dir_fast` 与 `scan_dir_nt`。
2. 包含 WIN32_FIND_DATA 解析、FastEntry 构建、条目过滤评估、FILETIME 转换。
3. 底层 API 细节与业务过滤条件耦合。

建议目标结构：

```text
src/find/walker/dir_windows/
├── mod.rs
├── fast_scan.rs
├── nt_scan.rs
├── entry.rs
├── eval.rs
└── time.rs
```

说明：

1. 扫描引擎（fast/nt）与条目评估分离，便于后续性能优化。
2. FILETIME 与 wide string 处理归入独立工具模块，降低重复。

---

## 12. 后续任务重点优先级（含第二梯队）

### P0（立即）

1. 先完成第 3-8 节定义的 6 组主拆分目标。

### P1（主拆分完成后优先）

1. `src/cli/env.rs`
2. `src/commands/completion/candidates.rs`
3. `src/commands/proxy_old.rs`

### P2（持续治理）

1. `src/alias/mod.rs`
2. `src/alias/shim_gen.rs`
3. `src/commands/ports.rs`
4. `src/commands/ctx/cmd.rs`
5. `src/find/walker/dir_windows.rs`

### 完成标准

1. 上述文件全部完成模块化拆分。
2. 无行为变化（CLI/API 输出契约保持兼容）。
3. 全量构建与关键冒烟通过。

---

## 13. 第三梯队：全量冗长/杂糅文件清单（自动扫描纳入）

> 扫描基线：`src/` 下单文件行数 >= 420（2026-03-06）。  
> 原则：以下项目全部纳入拆分 backlog，不再逐个二次确认。

### 13.1 已在主线/第二梯队覆盖（状态追踪）

1. `src/commands/dashboard/handlers.rs`（主线）
2. `src/commands/env/cmd.rs`（主线）
3. `src/commands/acl.rs`（主线）
4. `src/commands/env/tui.rs`（主线）
5. `src/env_core/mod.rs`（主线）
6. `src/cli/env.rs`（第二梯队）
7. `src/commands/dashboard/handlers_env.rs`（主线）
8. `src/commands/proxy_old.rs`（第二梯队）
9. `src/commands/completion/candidates.rs`（第二梯队）
10. `src/alias/mod.rs`（第二梯队）
11. `src/alias/shim_gen.rs`（第二梯队）
12. `src/commands/ports.rs`（第二梯队）
13. `src/find/walker/dir_windows.rs`（第二梯队）
14. `src/commands/ctx/cmd.rs`（第二梯队）

### 13.2 新增第三梯队（文件级）

1. `src/img/vector/visioncortex.rs`
- 现状：图像向量化链路、后端选择、参数分支、流程控制耦合。
- 建议拆分：
```text
src/img/vector/visioncortex/
├── mod.rs
├── backend.rs
├── pipeline.rs
├── cluster.rs
├── convert.rs
└── options.rs
```

2. `src/env_core/types.rs`
- 现状：Env 全域 DTO（scope/vars/snapshot/doctor/diff/schema/profile/events/run）集中。
- 建议拆分：
```text
src/env_core/types/
├── mod.rs
├── core.rs
├── snapshot.rs
├── doctor.rs
├── diff.rs
├── schema.rs
├── profile.rs
└── runtime.rs
```

3. `src/img/vector/bezier.rs`
- 现状：贝塞尔曲线拟合、采样、误差评估混合。
- 建议拆分：`fit.rs`、`sample.rs`、`error.rs`、`geom.rs`。

4. `src/ports.rs`
- 现状：端口采样、进程映射、平台细节与模型转换耦合。
- 建议拆分：`collect.rs`、`model.rs`、`process_map.rs`、`filter.rs`。

5. `src/commands/cstat/tui.rs`
- 现状：状态管理、事件循环、渲染/输入混合。
- 建议拆分：`app_state.rs`、`event_loop.rs`、`render.rs`、`keymap.rs`。

6. `src/commands/redirect/undo.rs`
- 现状：undo 计划解析、执行、日志恢复逻辑混合。
- 建议拆分：`plan.rs`、`executor.rs`、`history.rs`、`report.rs`。

7. `src/commands/redirect/matcher.rs`
- 现状：规则匹配、路径规范化、优先级决策耦合。
- 建议拆分：`rule_match.rs`、`path_norm.rs`、`score.rs`。

8. `src/commands/mod.rs`
- 现状：全局命令分发与子模块桥接集中。
- 建议拆分：
```text
src/commands/dispatch/
├── mod.rs
├── core.rs
├── env.rs
├── dashboard.rs
└── misc.rs
```

9. `src/commands/proxy/ops.rs`
- 现状：proxy 配置装配、系统应用、检测逻辑耦合。
- 建议拆分：`state.rs`、`apply.rs`、`detect.rs`、`format.rs`。

10. `src/windows/handle_query.rs`
- 现状：底层查询、结构解析、输出模型混合。
- 建议拆分：`query.rs`、`parse.rs`、`types.rs`、`convert.rs`。

11. `src/env_core/io.rs`
- 现状：导入解析、格式识别、导出渲染、apply 策略集中。
- 建议拆分：`import_parse.rs`、`import_apply.rs`、`export_render.rs`、`bundle.rs`。

12. `src/config.rs`
- 现状：全局配置 schema、load/save、默认值、兼容处理集中。
- 建议拆分：`model.rs`、`load_save.rs`、`defaults.rs`、`compat.rs`。

13. `src/commands/delete/tree.rs`
- 现状：删除树遍历、策略决策、输出展示混合。
- 建议拆分：`walk.rs`、`plan.rs`、`execute.rs`、`render.rs`。

14. `src/commands/redirect/cmd/modes.rs`
- 现状：mode 分发、参数校验、执行流程混合。
- 建议拆分：`mode_scan.rs`、`mode_apply.rs`、`mode_preview.rs`。

15. `src/img/vector/potrace.rs`
- 现状：轮廓提取、路径拟合、参数转换耦合。
- 建议拆分：`trace.rs`、`path.rs`、`approx.rs`、`options.rs`。

16. `src/commands/bookmarks/maintenance.rs`
- 现状：维护任务（清理/校验/修复）与 I/O 操作混合。
- 建议拆分：`check.rs`、`repair.rs`、`cleanup.rs`、`report.rs`。

17. `src/acl/export.rs`
- 现状：ACL 导出格式化、序列化、文件写入集中。
- 建议拆分：`format.rs`、`writer.rs`、`schema.rs`。

18. `src/acl/writer.rs`
- 现状：ACL 写入策略、权限应用、错误处理混合。
- 建议拆分：`apply.rs`、`inheritance.rs`、`error_map.rs`。

### 13.3 新增第三梯队（模块级）

1. `src/commands/redirect/`（38 文件，约 5543 行）
- 现状：功能完整但模块边界不够清晰，跨 `cmd/engine/watcher` 仍有耦合。
- 目标：目录级收敛为 `domain + engine + adapters`，减少跨层调用。

2. `src/commands/delete/`（34 文件，约 3381 行）
- 现状：删除策略、回收、UI 展示、WinAPI 交织。
- 目标：统一 `plan -> execute -> report` 分层。

3. `src/img/vector/`（7 文件，约 2972 行）
- 现状：多算法后端共存，参数与流程重复。
- 目标：抽象共享 pipeline，算法后端插件化。

---

## 14. 自动纳入规则（以后不再单独询问）

从本文件生效后，符合以下任一条件的文件，自动进入拆分 backlog：

1. 单文件行数 >= 420。
2. 单文件包含 3 个以上不同职责域（如“命令分发 + 业务执行 + 渲染/序列化”）。
3. 同一目录累计 > 2500 行且跨 20+ 文件并存在跨层直接调用。
4. 出现明显重复 helper（解析、错误映射、响应包装）且复制到 2 个以上文件。

执行策略：

1. 自动归类到主线/第二梯队/第三梯队。
2. 在 `docs/refactor-split-plan.md` 增补目标结构与验收项。
3. 拆分时默认“行为不变重构”，禁止顺带功能改写。
