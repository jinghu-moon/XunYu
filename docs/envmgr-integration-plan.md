# XunYu 集成 EnvMgr 功能实施方案（重写版）

> 状态：待评审
> 版本：v4.0
> 日期：2026-03-05
> 适用仓库：`D:/100_Projects/110_Daily/Xun`

---

## 1. 决策变更（本版关键）

本版将目标从“CLI 核心优先，TUI/Web 可选扩展”调整为：

1. **CLI 必须**
2. **完整 TUI 必须**
3. **完整 Web API + Dashboard 面板必须**

这不是能力堆叠，而是同一套 `env-core` 能力的三种适配器交付。

---

## 2. 目标与边界

### 2.1 目标（全部必须交付）

在不破坏 `xun` 现有命令体系和 dashboard 体系的前提下，集成 EnvMgr 级能力：

1. 注册表级环境变量管理（User/System）
2. 自动快照与恢复（防误操作）
3. 健康检查（doctor）与差异分析（diff）
4. 导入导出（json/env/reg/csv）
5. 完整 TUI 面板（可查看、编辑、修复、恢复）
6. 完整 Web API + Vue Dashboard 面板（含实时事件）

### 2.2 非目标（仍不做）

1. 不新增独立二进制（不做单独 `envmgr.exe`，统一收敛到 `xun`）
2. 不做云同步/多机协同
3. 不做 Secret/Vault 产品线
4. 不尝试“子进程直接修改父 Shell 环境”（继续使用现有 shell 注入机制）

---

## 3. 现状基线（xun 与 refer/envmgr）

### 3.1 xun 已具备的基础能力

1. CLI 基于 `argh`：`src/cli.rs:92`、`src/cli.rs:119`
2. 命令分发入口：`src/main.rs:75`、`src/commands/mod.rs:447`
3. Shell 环境注入协议：`__ENV_SET__` / `__ENV_UNSET__`
   - 发出：`src/commands/ctx/cmd.rs:497`、`src/commands/ctx/cmd.rs:504`
   - 消费：`src/commands/mod.rs:122`、`src/commands/mod.rs:314`
4. Dashboard（axum）已在线：
   - 路由骨架：`src/commands/dashboard/mod.rs:82`
   - `serve` 启动：`src/commands/dashboard/mod.rs:165`
   - WebSocket：`src/commands/dashboard/handlers.rs:2274`
5. Vue dashboard-ui 已存在可扩展面板体系：`dashboard-ui/src/App.vue`
6. `tui` 与 `dashboard` feature 已有：`Cargo.toml:23`、`Cargo.toml:25`

### 3.2 refer/envmgr 可借鉴模块（按价值）

1. Phase1 核心：`registry.rs`、`history.rs`、`import.rs`、`uac.rs`
2. Phase5 I/O：`export.rs`（多格式 + 自动识别导入）
3. Phase7 检查：`check.rs`（结构化报告 + CI 退出码）
4. Phase11 锁：`lock.rs`（跨进程写锁 + stale lock 清理）
5. Phase15：`config.rs`、`env_diff.rs`
6. Phase12/13/14：`env_merge.rs`、`schema.rs`、`annotations.rs`、`notifier.rs`

说明：`refer/envmgr` 是阶段参考，不是可直接整体编译工程（`phase1/Cargo.toml` 为空）。

---

## 4. 架构总策略（三端同核）

### 4.1 统一核心层（唯一事实源）

新增 `env-core` 领域层，承载所有业务规则，禁止在 CLI/TUI/Web 重复实现逻辑：

1. `registry`：读写注册表、REG 类型处理、广播
2. `snapshot`：快照创建/列出/恢复/裁剪
3. `doctor`：健康检查与修复建议
4. `io`：import/export 解析与渲染
5. `diff`：变量差异与 PATH 段差异
6. `lock`：并发写入保护
7. `config`：env 子系统配置
8. `events`：操作事件发布（供 TUI/Web 刷新）

### 4.2 三适配器层

1. CLI 适配器：`xun env ...`
2. TUI 适配器：`xun env tui`
3. Web 适配器：`/api/env/*` + dashboard-ui `EnvPanel.vue`

### 4.3 设计原则约束

1. KISS：先把核心路径做通，不做超前插件化
2. DRY：核心规则只写一次，三端只做展示/交互
3. YAGNI：不提前做“多后端存储抽象”
4. SOLID：
   - 单一职责：`registry/snapshot/doctor/io` 分模块
   - 依赖倒置：适配器依赖 `env-core` 抽象接口

---

## 5. 目标能力清单（必须项）

### 5.1 CLI 能力

```bash
xun env list [--scope user|system|all] [-f auto|table|tsv|json]
xun env get <NAME> [--scope user|system]
xun env set <NAME> <VALUE> [--scope user|system] [--no-snapshot]
xun env del <NAME> [--scope user|system] [--yes]
xun env path add <ENTRY> [--scope user|system] [--head|--tail]
xun env path rm <ENTRY> [--scope user|system]
xun env snapshot create [--desc <TEXT>]
xun env snapshot list
xun env snapshot restore [--id <ID> | --latest] [--yes]
xun env doctor [--scope user|system|all] [--fix] [--format text|json]
xun env export [--scope user|system] --format json|env|reg|csv [--out <FILE>]
xun env import <FILE> [--scope user|system] [--merge|--overwrite] [--dry-run]
xun env diff-live [--scope user|system] [--snapshot <ID>] [--color]
xun env tui
```

### 5.2 TUI 能力（完整）

1. 变量列表浏览（筛选/搜索/排序）
2. 变量新增/编辑/删除（含确认）
3. PATH 专区（分段展示、去重、无效项修复）
4. 快照面板（创建、预览、恢复）
5. doctor 面板（问题分类、逐项修复）
6. import/export 流程（预览与 dry-run）
7. 状态栏展示 scope、锁状态、最后操作结果

### 5.3 Web 能力（完整）

1. REST API：完整 env 管理面
2. Dashboard 新增 Env Panel（Vue）
3. WebSocket 推送 env 变更事件，前端实时刷新
4. 与现有 dashboard 风格与交互保持一致

---

## 6. Web API 设计（首版）

基于现有 `base_router()` 扩展，新增：

1. `GET /api/env/vars?scope=user|system|all`
2. `GET /api/env/vars/{name}?scope=user|system`
3. `POST /api/env/vars/{name}`（set）
4. `DELETE /api/env/vars/{name}?scope=user|system`
5. `POST /api/env/path/add`
6. `POST /api/env/path/remove`
7. `GET /api/env/snapshots`
8. `POST /api/env/snapshots`（create）
9. `POST /api/env/snapshots/restore`
10. `POST /api/env/doctor/run`
11. `POST /api/env/doctor/fix`
12. `POST /api/env/import`
13. `GET /api/env/export`
14. `GET /api/env/diff-live`
15. `GET /api/env/ws`（或沿用 `/ws` 多事件类型）

事件示例：

```json
{
  "type": "env.changed",
  "scope": "user",
  "name": "JAVA_HOME",
  "at": "2026-03-05T10:00:00Z"
}
```

---

## 7. 代码接线设计（xun 实际落点）

### 7.1 Rust 模块建议

```text
src/
├── cli/
│   └── env.rs
├── commands/
│   ├── env.rs
│   ├── env/
│   │   ├── cmd.rs
│   │   ├── tui.rs                 # #[cfg(feature = "tui")]
│   │   └── web_dto.rs
│   └── dashboard/
│       ├── mod.rs                 # route 接线
│       └── handlers_env.rs        # env API handlers
└── env_core/
    ├── mod.rs
    ├── types.rs
    ├── registry.rs
    ├── snapshot.rs
    ├── doctor.rs
    ├── io.rs
    ├── diff.rs
    ├── lock.rs
    ├── config.rs
    └── events.rs
```

### 7.2 关键接线点

1. `src/cli.rs`：注册 `Env(EnvCmd)`
2. `src/commands/mod.rs`：`dispatch()` 接 `SubCommand::Env(...)`
3. `src/commands/dashboard/mod.rs:82`：扩展 `/api/env/*` 路由
4. `dashboard-ui/src/api.ts`：新增 env API 封装
5. `dashboard-ui/src/App.vue`：新增 Env tab + 面板组件

---

## 8. 分阶段实施计划（全部必须完成）

## 8.1 M0：骨架与契约（1 天）

交付：

1. `env_core` 空模块与 trait 契约
2. `xun env --help` 与 `xun env tui --help`
3. dashboard 路由占位 `/api/env/ping`

验收：

1. `cargo check --all-features` 通过
2. CLI/TUI/Web 三入口均可达

## 8.2 M1：核心引擎（2-3 天）

交付：

1. registry/snapshot/lock/io/doctor/diff 可用
2. 写操作统一加锁 + 自动快照
3. UAC 与错误码模型打通

验收：

1. 核心模块单测覆盖关键分支
2. Windows 10/11 手工回归通过

## 8.3 M2：CLI 完整化（1.5-2 天）

交付：

1. 第 5.1 节所有 CLI 命令可用
2. `--format json` 与 CI 场景可用

验收：

1. 端到端脚本：set -> snapshot -> del -> restore
2. import/export round-trip 通过

## 8.4 M3：TUI 完整化（2-3 天）

交付：

1. 第 5.2 节全部面板功能
2. 键位帮助、确认弹窗、错误提示完整

验收：

1. `cargo run --features tui -- env tui` 可稳定运行
2. 典型操作全链路手测通过

## 8.5 M4：Web API 完整化（2-3 天）

交付：

1. 第 6 节 API 全部完成
2. WebSocket 事件推送可用

验收：

1. API 契约测试通过
2. 并发写入与刷新行为稳定

## 8.6 M5：Dashboard Env Panel（2-3 天）

交付：

1. `EnvPanel.vue` + 子组件完整交互
2. 与现有主题、反馈、命令面板风格统一

验收：

1. 前后端联调通过
2. 实时刷新、批量操作、错误回显可用

## 8.7 M6：收口与发布（1-1.5 天）

交付：

1. 文档、补全、帮助、迁移说明
2. 性能与稳定性回归

验收：

1. DoD 全项通过
2. 发布标签前 smoke test 通过

## 8.8 执行细化手册（详细版）

### M0 详细：骨架与契约

前置条件：

1. `src/cli.rs` 与 `src/commands/mod.rs` 当前编译通过
2. 本地可执行 `cargo check --all-features`

任务包：

1. CLI 接线：
   - 新建 `src/cli/env.rs`
   - 在 `src/cli.rs` 注册 `EnvCmd` 与 `SubCommand::Env`
2. 命令分发接线：
   - 新建 `src/commands/env.rs`（骨架）
   - 在 `src/commands/mod.rs` 注册 `env` 分发
3. 核心目录骨架：
   - 新建 `src/env_core/mod.rs` 与空模块文件
4. Web 占位接口：
   - 在 `src/commands/dashboard/mod.rs` 增加 `/api/env/ping`
5. TUI 占位入口：
   - `xun env tui` 返回明确占位提示，不 panic

验收命令：

```powershell
cargo check --all-features
cargo run -- env --help
cargo run --features dashboard -- serve --port 7071
```

完成判定：

1. CLI/TUI/Web 三入口可达
2. 无 feature 组合编译错误

---

### M1 详细：核心引擎

前置条件：

1. M0 已完成
2. Windows 开发环境具备注册表写权限（User 至少可写）

任务包：

1. `registry.rs`：
   - User/System 环境变量读写
   - `REG_SZ` 与 `REG_EXPAND_SZ` 写入策略
   - `WM_SETTINGCHANGE("Environment")` 广播
2. `snapshot.rs`：
   - `create/list/restore/prune`
   - 写操作前自动快照
3. `lock.rs`：
   - 跨进程锁文件
   - stale lock 检测与清理
4. `io.rs`：
   - `json/env/reg/csv` 导入导出
5. `doctor.rs`：
   - PATH 无效项、重复项、shadow 检查
6. `diff.rs`：
   - map diff 与 PATH segment diff

验收命令：

```powershell
cargo test env_core -- --nocapture
cargo run -- env list --scope user -f json
```

完成判定：

1. 核心模块具备单元测试
2. User scope 全链路可跑通

---

### M2 详细：CLI 完整化

前置条件：

1. M1 核心模块稳定

任务包：

1. 完成 `xun env` 命令树与参数校验
2. 对接 `env_core` 服务接口
3. 统一输出格式（auto/table/tsv/json）
4. 统一退出码（成功/参数错误/运行错误/权限错误）
5. 完成命令帮助与 completion 更新

验收命令：

```powershell
xun env set JAVA_HOME "C:\\Java\\jdk"
xun env get JAVA_HOME
xun env snapshot list
xun env doctor --scope user --format json
```

完成判定：

1. `xun env` 全部子命令可执行
2. import/export 与 snapshot 形成闭环

---

### M3 详细：TUI 完整化

前置条件：

1. M1/M2 功能稳定
2. `tui` feature 已在本地启用

任务包：

1. 结构与状态：
   - TUI `AppState`、事件循环、按键映射
2. 主界面：
   - 变量列表、筛选、搜索、排序
3. 编辑流：
   - 新增/编辑/删除确认
   - PATH 专区编辑（head/tail、去重）
4. 运维流：
   - doctor 结果页 + fix
   - snapshot 列表 + restore
5. I/O 流：
   - import 预览
   - export 目标格式与输出路径

验收命令：

```powershell
cargo run --features tui -- env tui
```

完成判定：

1. 关键键位可用（导航/编辑/确认/退出）
2. 不发生终端状态污染（退出后终端恢复正常）

---

### M4 详细：Web API 完整化

前置条件：

1. M1 核心模块稳定
2. `dashboard` feature 启用

任务包：

1. 新建 `handlers_env.rs`：
   - 实现 `/api/env/*` 接口
2. 路由接线：
   - 在 `base_router()` 注入 env routes
3. 事件机制：
   - 复用现有 `broadcast` 通道
   - 定义 `env.changed`/`env.snapshot`/`env.doctor` 事件
4. API 错误模型：
   - 标准 JSON 错误体（code/message/details）
5. API 契约测试：
   - 正常流与错误流

验收命令：

```powershell
cargo run --features dashboard -- serve --port 7071
curl "http://127.0.0.1:7071/api/env/vars?scope=user"
```

完成判定：

1. 全部 `/api/env/*` 可用
2. WebSocket 可收到 env 变更事件

---

### M5 详细：Dashboard Env Panel

前置条件：

1. M4 API 完整可用
2. `dashboard-ui` 能本地构建

任务包：

1. 前端 API 封装：
   - `dashboard-ui/src/api.ts` 增加 env 相关函数
2. 组件开发：
   - `EnvPanel.vue`
   - 必要子组件（列表、PATH 编辑、快照、doctor、导入导出）
3. App 集成：
   - `dashboard-ui/src/App.vue` 增加 Env tab
4. 实时更新：
   - 接入 WS 事件刷新
5. 交互一致性：
   - 复用现有 toast/loading/empty/error 组件模式

验收命令：

```powershell
cd dashboard-ui
pnpm build
```

完成判定：

1. Env 面板全流程可用
2. 与现有 dashboard 视觉与交互一致

---

### M6 详细：收口与发布

前置条件：

1. M0-M5 全部完成

任务包：

1. 文档收口：
   - 命令帮助、开发文档、测试手册
2. 回归测试：
   - `ctx`/`proxy`/dashboard 现有模块回归
3. 稳定性验证：
   - 长时间运行、并发写入、异常恢复
4. 发布准备：
   - 版本说明、迁移说明、已知限制

验收命令：

```powershell
cargo test --all-features
cargo check --all-features
```

完成判定：

1. DoD 全项满足
2. 无阻断级缺陷

---

## 9. Definition of Done（强制）

只有以下全部满足才算完成：

1. CLI/TUI/Web 三端功能对齐到同一能力集
2. 三端不含重复业务逻辑（核心逻辑仅在 `env_core`）
3. User/System 作用域行为一致且可回滚
4. doctor/diff/import/export 在三端均可触达
5. Web 实时事件可用且不会导致 UI 状态漂移
6. `ctx` / `proxy` / 现有 dashboard 功能无回归

---

## 10. 风险与应对

| 风险 | 级别 | 应对 |
|---|---|---|
| 三端并行导致重复实现 | 高 | 先做 `env_core`，适配器禁写业务规则 |
| Web/TUI 交付面过大导致延期 | 高 | M0 明确能力切片，按能力垂直切端到端 |
| System 写入权限失败 | 高 | UAC 前置检查 + 明确 fallback 提示 |
| 快照损坏导致恢复失败 | 高 | restore 前 JSON 校验 + 恢复前再备份当前状态 |
| 路由膨胀影响 dashboard 稳定性 | 中 | `handlers_env.rs` 独立文件与独立测试 |

---

## 11. 里程碑建议

1. **v0.2.0-env-core**：M0 + M1
2. **v0.3.0-env-cli**：M2
3. **v0.4.0-env-tui**：M3
4. **v0.5.0-env-web-api**：M4
5. **v0.6.0-env-panel**：M5 + M6

---

## 12. 官方参考资料

1. Rust `std::env::set_var`：<https://doc.rust-lang.org/std/env/fn.set_var.html>
2. Rust 2024 新增 unsafe 函数说明：<https://doc.rust-lang.org/edition-guide/rust-2024/newly-unsafe-functions.html>
3. Windows `SetEnvironmentVariableW`：<https://learn.microsoft.com/en-us/windows/win32/api/processenv/nf-processenv-setenvironmentvariablew>
4. Windows 环境变量进程模型：<https://learn.microsoft.com/en-us/windows/win32/procthread/environment-variables>
5. Windows `WM_SETTINGCHANGE`：<https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-settingchange>
6. `winreg` 文档：<https://docs.rs/winreg/latest/winreg/>
7. `axum` 文档：<https://docs.rs/axum/latest/axum/>
8. `ratatui` 文档：<https://docs.rs/ratatui/latest/ratatui/>
9. Vue 3 文档：<https://vuejs.org/guide/introduction.html>

---

## 13. 与 v3 差异

1. 将 TUI/Web 从“扩展项”升级为“必须交付项”
2. 里程碑重排为 `env_core -> CLI -> TUI -> Web API -> Web Panel`
3. 明确三端统一能力集与单一事实源约束
4. 新增 Web API 契约与事件模型
5. DoD 改为三端一致性验收，不接受仅 CLI 完成
