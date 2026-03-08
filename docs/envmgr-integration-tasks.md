# XunYu envmgr 集成 — 分阶段任务清单

> 依据：`./envmgr-integration-plan.md`（v4.0）  
> 标记：`[ ]` 待办　`[-]` 进行中　`[x]` 完成  
> 目标：CLI + 完整 TUI + 完整 Web API/Dashboard 全量交付

---

## 0. 执行规则

- [x] 所有业务逻辑必须进入 `src/env_core/*`，禁止在 CLI/TUI/Web 重复实现
- [x] 每个 Phase 完成后必须通过对应验收命令再进入下一阶段
- [x] Windows-only 能力必须有非 Windows 清晰错误提示（编译可过）
- [x] 所有环境变量写操作必须统一经过：`lock -> snapshot -> write -> broadcast`

> 注：环境变量相关写操作已统一走该流水线；`schema/annotations/config` 仍为文件侧写入路径，后续可继续收敛。

---

## Phase 0：骨架与契约（M0）

### P0.1 CLI 与命令分发骨架

- [x] 新建 `src/cli/env.rs`，定义 `EnvCmd` + 子命令骨架
- [x] `src/cli.rs` 注册 `mod env;` 与 `pub use env::*;`
- [x] `src/cli.rs` `SubCommand` 增加 `Env(EnvCmd)`
- [x] 新建 `src/commands/env.rs`，提供 `cmd_env` 入口
- [x] `src/commands/mod.rs` 注册 `pub(crate) mod env;`
- [x] `src/commands/mod.rs` `dispatch()` 增加 `SubCommand::Env(...)`

### P0.2 env_core 骨架

- [x] 新建 `src/env_core/mod.rs`
- [x] 新建 `src/env_core/types.rs`
- [x] 新建 `src/env_core/registry.rs`
- [x] 新建 `src/env_core/snapshot.rs`
- [x] 新建 `src/env_core/doctor.rs`
- [x] 新建 `src/env_core/io.rs`
- [x] 新建 `src/env_core/diff.rs`
- [x] 新建 `src/env_core/lock.rs`
- [x] 新建 `src/env_core/config.rs`
- [x] 新建 `src/env_core/events.rs`

### P0.3 TUI/Web 占位入口

- [x] `xun env tui` 子命令接线（占位实现）
- [x] `src/commands/dashboard/mod.rs` 增加 `GET /api/env/ping`
- [x] 为 `/api/env/ping` 增加最小 handler 与响应模型

### P0.4 验收

- [x] `cargo check --all-features`
- [x] `cargo run -- env --help`
- [x] `cargo run --features dashboard -- serve --port 7071`
- [x] 访问 `http://127.0.0.1:7071/api/env/ping` 返回 200

---

## Phase 1：核心引擎（M1）

### P1.1 类型与错误模型

- [x] 在 `src/env_core/types.rs` 定义：
  - [x] `EnvScope`（User/System/All）
  - [x] `EnvVar`（name/raw_value/reg_type）
  - [x] `Snapshot`/`SnapshotMeta`
  - [x] `DoctorIssue`/`DoctorReport`
  - [x] `DiffEntry`/`EnvDiff`
- [x] 定义统一错误类型与错误码映射

### P1.2 注册表读写（registry）

- [x] User/System 作用域 key 打开与权限检查
- [x] `list/get/set/del` 基础 CRUD
- [x] PATH 读写与分段处理
- [x] `REG_SZ` 与 `REG_EXPAND_SZ` 写入策略
- [x] 广播 `WM_SETTINGCHANGE("Environment")`

### P1.3 快照（snapshot）

- [x] 快照目录与路径策略（默认 `~/.xun.env.snapshots`）
- [x] `create/list/restore` 实现
- [x] 快照裁剪（保留上限）
- [x] restore 前校验 JSON 完整性
- [x] restore 前创建当前状态保护快照

### P1.4 并发锁（lock）

- [x] 锁文件路径与创建策略
- [x] stale lock 检测
- [x] `try_with_lock` helper
- [x] 写操作强制走锁

### P1.5 导入导出（io）

- [x] 导出支持 `json/env/reg/csv`
- [x] 导入自动识别格式（content heuristic）
- [x] dry-run 解析结果输出
- [x] merge/overwrite 策略

### P1.6 健康检查与差异（doctor/diff）

- [x] doctor：PATH 无效目录
- [x] doctor：PATH 重复项（大小写不敏感）
- [x] doctor：User shadow System 同名变量
- [x] diff：变量 added/removed/changed
- [x] diff：PATH 段级差异

### P1.7 验收

- [x] `cargo test env_core -- --nocapture`
- [x] `cargo run -- env list --scope user -f json`
- [x] `cargo run -- env snapshot create --desc smoke`
- [x] `cargo run -- env doctor --scope user --format json`

---

## Phase 2：CLI 完整化（M2）

### P2.1 命令树实现

- [x] `list/get/set/del`
- [x] `path add/path rm`
- [x] `snapshot create/list/restore`
- [x] `doctor`（含 `--fix`）
- [x] `import/export`
- [x] `diff-live`
- [x] `tui`

### P2.2 输出与交互

- [x] 支持 `--format auto|table|tsv|json`
- [x] 危险操作确认：`del/restore/import(overwrite)` 默认确认
- [x] `--yes` 跳过确认
- [x] 错误输出统一结构化

### P2.3 Completion 与 help

- [x] `xun init` 生成脚本加入 `env` 子命令补全
- [x] `src/commands/completion/*` 同步 `env` 子命令
- [x] 命令帮助示例完善

### P2.4 验收

- [x] `xun env set JAVA_HOME "C:\\Java\\jdk"`（已用临时变量 `XUN_ENVMGR_SMOKE_WRITE_*` 等价验证 set 链路并清理）
- [x] `xun env get JAVA_HOME`（已用临时变量 `XUN_ENVMGR_SMOKE_WRITE_*` 等价验证 get 链路）
- [x] `xun env export --scope user --format json`
- [x] `xun env import .\\.tmp\\env.json --scope user --dry-run`

---

## Phase 3：TUI 完整化（M3）

### P3.1 TUI 基础框架

- [x] 新建 `src/commands/env/tui.rs`
- [x] AppState/事件循环/键盘映射
- [x] Header/List/Detail/Status 布局
- [x] 异常退出终端恢复

### P3.2 变量管理面板

- [x] 变量列表分页/滚动/搜索
- [x] 新增/编辑/删除流程
- [x] scope 切换（user/system/all）

### P3.3 PATH 专区

- [x] PATH 分段显示
- [x] add/remove/move（head/tail）
- [x] 去重与无效项标注

### P3.4 快照与恢复面板

- [x] 快照列表
- [x] 快照详情预览
- [x] 恢复确认与结果反馈

### P3.5 doctor 与 import/export 面板

- [x] 问题分类展示
- [x] 一键 fix（可回滚）
- [x] 导入预览与 dry-run 显示
- [x] 导出格式与路径选择

### P3.6 验收

- [x] `cargo run --features tui -- env tui`
- [x] 手测路径：set -> snapshot -> del -> restore
- [x] 手测路径：doctor -> fix -> diff-live

> 最小人工执行清单见：`docs/envmgr-manual-checklist.md`

---

## Phase 4：Web API 完整化（M4）

### P4.1 API handlers 与路由接线

- [x] 新建 `src/commands/dashboard/handlers_env.rs`
- [x] `src/commands/dashboard/mod.rs` 注册：
  - [x] `GET /api/env/vars`
  - [x] `GET /api/env/vars/{name}`
  - [x] `POST /api/env/vars/{name}`
  - [x] `DELETE /api/env/vars/{name}`
  - [x] `POST /api/env/path/add`
  - [x] `POST /api/env/path/remove`
  - [x] `GET /api/env/snapshots`
  - [x] `POST /api/env/snapshots`
  - [x] `POST /api/env/snapshots/restore`
  - [x] `POST /api/env/doctor/run`
  - [x] `POST /api/env/doctor/fix`
  - [x] `POST /api/env/import`
  - [x] `GET /api/env/export`
  - [x] `GET /api/env/diff-live`

### P4.2 事件推送

- [x] 定义 `env.changed` 事件
- [x] 定义 `env.snapshot` 事件
- [x] 定义 `env.doctor` 事件
- [x] 接入现有 WS 广播通道

### P4.3 API 契约与安全

- [x] 统一成功响应结构
- [x] 统一错误响应结构（code/message/details）
- [x] 参数校验与边界保护
- [x] 明确 localhost-only 约束（沿用现有 dashboard 模式）

### P4.4 验收

- [x] 启动 `xun serve` 后接口全部可访问
- [x] curl/postman 契约测试通过
- [x] WS 能收到 env 变更事件

---

## Phase 5：Dashboard Env Panel（M5）

### P5.1 前端 API 层

- [x] `dashboard-ui/src/api.ts` 新增 env API 方法
- [x] 添加 env DTO 类型定义（可拆到 `types.ts`）
- [x] 统一错误处理与 toast 反馈

### P5.2 面板与组件

- [x] 新建 `dashboard-ui/src/components/EnvPanel.vue`
- [x] 新建 `EnvVarsTable.vue`（变量列表）
- [x] 新建 `EnvPathEditor.vue`（PATH 编辑）
- [x] 新建 `EnvSnapshotsPanel.vue`
- [x] 新建 `EnvDoctorPanel.vue`
- [x] 新建 `EnvImportExportPanel.vue`

### P5.3 App 集成与实时刷新

- [x] `dashboard-ui/src/App.vue` 增加 Env tab
- [x] 订阅 WS env 事件并局部刷新
- [x] 与现有主题/密度/反馈机制一致

### P5.4 验收

- [x] `pnpm -C dashboard-ui build`
- [x] `xun serve` + 页面联调通过
- [x] 面板全链路可用（CRUD、snapshot、doctor、import/export、diff）（基于 `tools/envmgr-dashboard-chain-smoke.ps1` API 链路自动化验证）

> 页面联调最小人工执行清单见：`docs/envmgr-manual-checklist.md`

---

## Phase 6：收口发布（M6）

### P6.1 文档与说明

- [x] 更新 `docs/README.md` 索引
- [x] 增加 env 使用手册与 FAQ
- [x] 补充 Windows 权限与风险说明

### P6.2 回归与稳定性

- [x] `ctx` 回归测试
- [x] `proxy` 回归测试
- [x] dashboard 现有 API 回归
- [x] 并发写入压力测试（锁 + 快照）

### P6.3 发布准备

- [x] 版本说明（新增能力、破坏性变更、迁移建议）
- [x] 已知限制清单
- [x] smoke 脚本与结果归档

### P6.4 验收

- [x] `cargo test --all-features`
- [x] `cargo check --all-features`
- [x] DoD 全项通过

---

## Phase 7：对标增量（M7）

### P7.1 Status Overview（对标 refer Phase 17）

- [x] CLI 新增：`xun env status --scope user|system|all --format text|json`
- [x] `env_core` 新增状态聚合模型（变量计数、快照、profiles、schema、annotations、audit）
- [x] Web API 新增：`GET /api/env/status?scope=user|system|all`

### P7.2 验收

- [x] `cargo check --all-features`
- [x] `xun env status --scope all --format text`
- [x] `GET /api/env/status` 返回 200 且包含状态摘要

## Phase 8：对标增量（M8）

### P8.1 Export-All ZIP（对标 refer Phase 17）

- [x] `env_core` 新增 ZIP 打包导出（json/env/reg/csv）
- [x] CLI 新增：`xun env export-all --scope user|system|all --out <zip>`
- [x] Web API 新增：`GET /api/env/export-all?scope=user|system|all`

### P8.2 Dashboard Status / Export 收口

- [x] Env 面板接入 `/api/env/status`，显示状态摘要（vars/snapshots/profiles/schema/audit）
- [x] Env Import/Export 面板新增 `Export ZIP` 按钮
- [x] Import 输入区支持拖拽文件填充内容（`.env/.json/.reg/.csv`）

### P8.3 验收

- [x] `cargo check --all-features`
- [x] `xun env export-all --scope user --out ./.tmp/xun-env-user.zip`
- [x] `GET /api/env/export-all?scope=user` 返回 ZIP 下载流
- [x] `pnpm -C dashboard-ui build`

## Phase 9：对标增量（M9）

### P9.1 Variable Type Inference（对标 refer Phase 16）

- [x] `env_core` 新增变量类型推断模块（url/path/path_list/boolean/secret/json/email/version/integer/float）
- [x] `EnvVar` 输出增加 `inferred_kind`（可选字段，向后兼容）
- [x] `registry/profile/snapshot` 相关读取链路统一填充 `inferred_kind`

### P9.2 API 与 Dashboard 消费

- [x] `GET /api/env/vars`、`GET /api/env/vars/{name}` 返回 `inferred_kind`
- [x] Env 变量表新增类型徽章（沿用 dashboard 现有配色变量）
- [x] 前端类型定义同步 `EnvVarKind`

### P9.3 验收

- [x] `cargo check --all-features`
- [x] `xun env list --scope user -f json` 包含 `inferred_kind`
- [x] `GET /api/env/vars?scope=user` 可见 `inferred_kind`
- [x] `pnpm -C dashboard-ui build`

## Phase 10：对标增量（M10）

### P10.1 Diff Since Date（对标 refer Phase 17）

- [x] CLI：`xun env diff-live --since <DATE>`（支持 RFC3339 / `YYYY-MM-DD` / `YYYY-MM-DD HH:MM:SS`）
- [x] `--snapshot` 与 `--since` 互斥校验（CLI + API）
- [x] Web API：`GET /api/env/diff-live?since=<DATE>`
- [x] Dashboard Diff 面板新增 since 输入框，和 snapshot 互斥

### P10.2 验收

- [x] `cargo check --all-features`
- [x] `xun env diff-live --scope user --since 2026-03-01 --format json`
- [x] `GET /api/env/diff-live?scope=user&since=2026-03-01` 返回 200
- [x] `GET /api/env/diff-live?scope=user&snapshot=latest&since=2026-03-01` 返回 400（`env.invalid_input`）

## Phase 18：对标增量（M18）

### P18.1 Import from stdin + UAC 提示（对标 refer Phase 18）

- [x] CLI：`xun env import --stdin --scope user|system`
- [x] CLI：`import` 输入源互斥校验（`<file>` 与 `--stdin` 不能同时使用）
- [x] `env_core` 新增 `uac` 检测模块（Windows token elevation）
- [x] system/all 写操作前置权限提示（`PermissionDenied`，含管理员重启指引）
- [x] TUI 在 system scope 增加权限警示文案

### P18.2 验收

- [x] `cargo check --all-features`
- [x] `xun env import --help` 显示 `--stdin`
- [x] `echo "X=1" | xun env import --stdin --scope user --dry-run`
- [x] `xun env import ./.tmp/env-smoke.json --stdin --scope user --dry-run` 返回输入互斥错误

## Phase 19：对标增量（M19）

### P19.1 Variable Dependencies Graph（对标 refer Phase 19）

- [x] `env_core` 新增 `dep_graph` 模块（`%VAR%` 依赖提取、缺失节点、循环检测、ASCII tree）
- [x] CLI 新增：`xun env graph <NAME> --scope user|system|all --max-depth N --format text|json`
- [x] Web API 新增：`GET /api/env/graph?scope=...&name=...&max_depth=...`
- [x] Dashboard 新增 `Dependency Graph` 面板（输入 root + depth，展示依赖树）

### P19.2 验收

- [x] `cargo check --all-features`
- [x] `cargo test dep_graph -- --nocapture`
- [x] `xun env graph PATH --scope user --max-depth 3 --format json`
- [x] `pnpm -C dashboard-ui build`

## Phase 20：对标增量（M20）

### P20.1 Snapshot Prune（对标 refer Phase 20）

- [x] `env_core` 新增 `snapshot::prune_snapshots(keep)`（按时间裁剪旧快照）
- [x] CLI 新增：`xun env snapshot prune --keep <N>`
- [x] Web API 新增：`DELETE /api/env/snapshots?keep=<N>`
- [x] Dashboard Snapshots 面板新增 prune 输入与按钮

### P20.2 Scheduled Snapshots

- [x] web 模式后台定时快照（`snapshot_every_secs`）已接入
- [x] 配置项 `general.snapshot_every_secs` 已增加（兼容 `snapshot_every_secs`）

### P20.3 验收

- [x] `cargo check --all-features`
- [x] `xun env snapshot prune --keep 99999`
- [x] `pnpm -C dashboard-ui build`

### P20.4 质量收敛

- [x] 清理 Env 相关无效导出/残留函数（`with_config/ensure_fixable_scope/snapshot_vars/has_issues`）
- [x] 修复 Windows FFI 声明冲突（`NtQuerySystemInformation` 签名统一）
- [x] `cargo check --all-features` 当前为零 warning

### 本轮验证记录（2026-03-06）

- `cargo check --all-features`：PASS
- `cargo test --all-features`：PASS
- `cargo test env_core -- --nocapture`：PASS
- `cargo run -- env --help`：PASS
- `cargo run -- env list --scope user -f json`：PASS
- `xun env set/get/del XUN_ENVMGR_SMOKE_WRITE_* --scope user`：PASS（临时变量写入验证，已清理）
- `cargo run -- env snapshot create --desc smoke`：PASS
- `cargo run -- env doctor --scope user --format json`：PASS
- `cargo run -- env export --scope user --format json --out ./.tmp/env-smoke.json`：PASS
- `cargo run -- env import ./.tmp/env-smoke.json --scope user --dry-run`：PASS
- `pnpm -C dashboard-ui build`：PASS
- `xun serve --port 7071` + `/api/env/ping|vars|schema|annotations|template/expand|export-live`：PASS
- `WS /api/env/ws` 连接首帧：PASS（`{"type":"connected","channel":"env"}`）
- `tools/envmgr-smoke.ps1 -SkipTests -VerifyWsChanged`：PASS（WS 收到 `type=changed` 事件）
- `tools/envmgr-concurrency-smoke.ps1 -Workers 4 -Iterations 10`：PASS（并发写入 120 次操作）
- `tools/envmgr-dashboard-chain-smoke.ps1 -Port 7073`：PASS（EnvPanel 核心 API 链路）
- `manual checklist (TUI + Dashboard)`：PASS（含清理）
- `xun env status --scope all --format text`：PASS
- `GET /api/env/status?scope=all`：PASS（200）
- `cargo check --all-features`（latest）：PASS（0 warnings）
- `xun env config set/get general.snapshot_every_secs`：PASS
- `xun serve --port 7098`（短时运行）+ `xun env snapshot list`：PASS（出现 `auto-snapshot`，并持续自动裁剪到 `max_snapshots` 上限）

---

## 依赖关系（执行顺序）

```text
P0 -> P1 -> P2 -> P3
          -> P4 -> P5
P3 + P5 -> P6
```

---

## 风险闸门（进入下一阶段前必须满足）

- [x] Gate-A（P1 后）：核心模块单测通过，且写操作可回滚
- [x] Gate-B（P2 后）：CLI 全命令可用，退出码稳定
- [x] Gate-C（P3 后）：TUI 不污染终端，关键链路可操作
- [x] Gate-D（P4 后）：API 契约稳定，WS 事件正常
- [x] Gate-E（P5 后）：前后端联调稳定，无阻断 bug

> Gate-C / Gate-E 验证步骤见：`docs/envmgr-manual-checklist.md`

