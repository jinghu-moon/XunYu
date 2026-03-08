# Redirect 模块导读

本文档专门解释 `xun redirect` 这条命令树。它不是一个简单的“移动文件”命令，而是一个基于规则、支持 dry-run、plan/apply、undo、watch、审计与 Dashboard 管理的文件分类子系统。

## 1. `redirect` 在项目里的位置

从结构上看，`redirect` 由四层组成：

```text
xun redirect ...
  -> src/cli/redirect.rs          # CLI 参数定义
  -> src/commands/redirect/cmd/*  # 命令执行与模式分发
  -> src/commands/redirect/*      # 规则、计划、日志、撤销、监听
  -> dashboard-ui RedirectPanel   # 可视化配置与 dry-run
```

和 `env`、`acl` 类似，`redirect` 也已经是一个完整的子系统，而不是单个命令文件。

## 2. CLI 入口：`src/cli/redirect.rs`

当前 `RedirectCmd` 暴露的能力包括：

- `source`
- `--profile`
- `--explain`
- `--stats`
- `--confirm`
- `--review`
- `--log`
- `--tx`
- `--last`
- `--validate`
- `--plan`
- `--apply`
- `--undo`
- `--watch`
- `--status`
- `--simulate`
- `--dry-run`
- `--copy`
- `--yes`
- `--format`

只看参数面就能读出它支持的模式：

1. **配置校验**：`--validate`
2. **规则解释 / 模拟**：`--explain`、`--simulate`
3. **普通执行**：直接跑、可 `--dry-run`
4. **交互执行**：`--confirm`、`--review`
5. **计划文件**：`--plan` / `--apply`
6. **撤销与审计**：`--undo`、`--log`
7. **持续监听**：`--watch`、`--status`

这说明 `redirect` 的定位并不是“执行一次 move”，而是“可治理、可回放、可持续运行”的规则引擎。

## 3. 执行入口：`src/commands/redirect/cmd/mod.rs`

`cmd_redirect(args)` 的大致流程是：

1. 读取全局配置并拿到 `config.redirect.profiles`
2. 校验命令组合是否合法（例如 `--plan` 不能和 `--apply` 同时使用）
3. 解析并校验 profile
4. 按模式分流：
   - `validate`
   - `log`
   - `explain`
   - `simulate`
   - `undo`
   - `plan`
   - `watch`
   - `review`
   - `confirm`
   - 普通执行

也就是说，`redirect` 顶层命令本质上是一个**模式路由器**。

## 4. 模式层：`src/commands/redirect/cmd/modes/*`

这个目录很重要，因为它把“运行方式”拆开了：

- `mode_scan.rs`
- `mode_preview.rs`
- `mode_apply.rs`
- `mod.rs`

从 `cmd/mod.rs` 可见，顶层会调用这些模式相关函数：

- `run_explain`
- `run_log`
- `run_simulate`
- `run_undo`
- `run_watch`
- `run_review`
- `run_confirm`
- `run_ask_conflict`
- `run_simple`

这说明 `redirect` 很强调“同一规则引擎，多种执行界面 / 交互方式”。

## 5. 规则与配置层

### 5.1 `config.rs`

`src/commands/redirect/config.rs` 负责 profile 校验，核心工作包括：

- 校验 rule 的 match 条件是否为空
- 校验 regex 是否合法
- 校验 size / age 表达式是否合法
- 校验目标路径模板是否合法

这意味着 profile 不是“相信用户配置”，而是有明确 schema-like 校验的。

### 5.2 `matcher/*`

`matcher` 负责规则命中逻辑，包括：

- 扩展名匹配
- glob / regex
- size 表达式
- age 表达式

这是 `redirect` 的判定核心。

## 6. 计划与执行层

### 6.1 `plan.rs`

`src/commands/redirect/plan.rs` 定义了计划文件的数据结构：

- `PlanFile`
- `PlanItem`
- `PlanKind`
- `ConflictAction`
- `FileFingerprint`
- `ConflictInfo`

这几个结构说明 `redirect` 的计划文件不是简陋的“src -> dst 列表”，而是包含：

- move / copy 类型
- 规则来源
- 文件指纹
- 冲突信息
- 冲突策略

因此 `--plan` / `--apply` 更像“预生成执行计划并回放”，而不是简单脚本导出。

### 6.2 `engine/*`

`engine` 是真正的规则引擎，当前继续拆成：

- `scan.rs`
- `plan.rs`
- `run.rs`
- `ops.rs`
- `path.rs`
- `conflict.rs`
- `audit.rs`
- `apply/*`
- `process/*`

这能反推出它的主流程大致是：

```text
扫描文件
  -> 规则匹配
  -> 生成候选计划
  -> 冲突处理
  -> 执行 move/copy
  -> 写审计日志
```

其中 `conflict.rs`、`audit.rs`、`apply/*` 的存在，说明这个引擎在一开始就考虑了“执行不只是算出来，还要安全落地”。

## 7. 撤销、日志与持续监听

### 7.1 `redirect_log.rs`

它负责记录事务 / audit 信息，是 `--log` 和 `--undo` 的基础。

### 7.2 `undo/*`

撤销逻辑被单独拆出，说明“可回滚”不是附带功能，而是系统级能力。

### 7.3 `watch_core.rs` 与 `watcher/*`

`watch` 模式不是简单 while-loop，而是独立的 watcher 子模块，说明它已经被视作常驻模式，而不是一次性命令的加个开关。

## 8. Dashboard 对应层

`redirect` 不只有 CLI，还对外暴露了 Web API：

- `/api/redirect/profiles`
- `/api/redirect/profiles/{name}`
- `/api/redirect/dry-run`

对应前端组件是 `dashboard-ui/src/components/RedirectPanel.vue`。

这意味着 Dashboard 的 Redirect 页面并不是平行实现，而是复用了同一套 profile / dry-run 思路，只是换成了可视化编辑器。

## 9. 推荐阅读顺序

### 9.1 想先搞清产品能力

1. `src/cli/redirect.rs`
2. `src/commands/redirect/cmd/mod.rs`
3. `src/commands/redirect/config.rs`
4. `dashboard-ui/src/components/RedirectPanel.vue`

### 9.2 想先看引擎

1. `src/commands/redirect/engine/mod.rs`
2. `src/commands/redirect/matcher/*`
3. `src/commands/redirect/engine/*`
4. `src/commands/redirect/plan.rs`

### 9.3 想先看运维安全能力

1. `src/commands/redirect/redirect_log.rs`
2. `src/commands/redirect/undo/*`
3. `src/commands/redirect/watch_core.rs`
4. `src/commands/redirect/watcher/*`

## 10. 当前实现上的几个观察

- `redirect` 的核心不是文件操作，而是规则引擎。
- `plan/apply`、`undo`、`watch` 这些能力放在一起，说明它被设计成长期运维工具，而不是临时脚本。
- `cmd/modes/*` 的拆分很关键，它把“怎么运行”从“跑什么规则”里分离了出来。
- Dashboard Redirect 页面本质上是规则配置与 dry-run 的可视化壳层。
- 仓库里的 `*_monolith_backup_*` 说明 `redirect` 也正处于显式模块化拆分阶段。
