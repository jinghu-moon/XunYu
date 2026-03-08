# XunYu P0 实施方案

这份文档用于把前面讨论的 **P0 能力** 收敛成一套可执行方案。

这里的 P0，不是“再堆几个新命令”，而是优先补齐会影响整个平台气质的三件事：

1. **危险动作统一协议**
2. **任务中心 / Recipe MVP**
3. **诊断中心 MVP**

目标是把 `XunYu` 从“命令很多的本地工具箱”推进到“有统一操作链路的本地控制台”。

## 当前实施状态

- [x] Milestone 1 已完成：危险动作统一协议（Preview -> Confirm -> Execute -> Receipt）
- [x] 前后端统一 guarded 响应字段：`phase`、`status`、`guarded`、`dry_run`、`summary`
- [x] 前端共享组件已收口：`TaskToolCard`、`UnifiedConfirmDialog`、`TaskReceiptComponent`
- [x] 三重保护失败链路已补测试：预演失败不可确认、确认缺失拒绝执行、执行失败仍返回回执
- [x] Milestone 2 已完成：最近任务 / 任务中心 MVP
- [x] 后端新增最近任务接口：`/api/workspaces/tasks/recent`
- [x] 总览与统计页已接入最近任务面板，支持成功 / 失败 / Dry Run 过滤
- [x] 任务中心支持两类安全重放：普通任务直接重放，危险任务走“重新预演 -> 确认执行”
- [x] Milestone 3 已完成：Recipe MVP
- [x] 新增 /api/workspaces/recipes / preview / execute / save 接口
- [x] Statistics & Diagnostics 新增 Recipe 工作流面板，支持预演、确认执行、执行回执与本地副本保存
- [x] 首批内置 3 类 Recipe：文件清理、环境治理、代理诊断
- [x] Milestone 4 已完成：诊断中心 MVP
- [x] 后端新增聚合接口：`/api/workspaces/diagnostics/summary`
- [x] Statistics & Diagnostics 已接入诊断中心面板，集中展示 doctor 总览、审计时间线、最近失败任务与高风险动作回执
- [x] 诊断中心聚合接口对 doctor 加载失败做降级处理：保留页面其余统计并返回 `load_error`

---

## 1. 为什么先做 P0

当前仓库已经具备一些很关键的基础：

- Dashboard 已经有 8 工作台 / 任务卡片 / 确认对话框 / 回执组件骨架。
- 后端已经有 `dry-run`、`what-if`、`audit`、`doctor` 等能力，不是从零开始。
- `Env`、`Redirect`、文件类命令已经证明：**预演、确认、结果展示、审计** 这条链路是成立的。

但目前还缺一层“统一协议”：

- 危险动作的交互方式还不够统一。
- 任务执行结果还没有变成跨工作台复用的“任务中心语言”。
- `doctor` / `audit` / 运行态检查还没有提升成一个明确的诊断中心。

所以 P0 的重点不是扩张命令面，而是**收敛平台骨架**。

---

## 2. 现有基础（可直接复用）

### 2.1 前端

当前前端已经具备 P0 所需的几个关键构件：

- `dashboard-ui/src/workspace-tools.ts`
  - 已定义工作台任务模型、字段模型、`run` / `guarded` 两种任务模式。
- `dashboard-ui/src/components/TaskToolbox.vue`
  - 已具备工作台级任务容器。
- `dashboard-ui/src/components/TaskToolCard.vue`
  - 已具备任务卡片、参数表单、确认弹窗与回执显示整合能力。
- `dashboard-ui/src/components/UnifiedConfirmDialog.vue`
  - 可作为危险动作统一确认层。
- `dashboard-ui/src/components/TaskReceiptComponent.vue`
  - 可作为统一结果回执层。
- `dashboard-ui/src/components/workspaces/StatisticsDiagnosticsWorkspace.vue`
  - 可以作为诊断中心的现成落点。

### 2.2 后端

当前后端已经具备 P0 直接可用的接口与模式：

- `src/commands/dashboard/mod.rs`
  - 已挂载 `/api/audit`
  - 已挂载 `/api/env/doctor/run`
  - 已挂载 `/api/env/doctor/fix`
  - 已挂载 `/api/redirect/dry-run`
- 多个 CLI / API 能力已支持 `--dry-run` / `--what-if`
- 测试中已覆盖多类 dry-run 行为与 Dashboard API 行为

### 2.3 结论

P0 不需要从零发明新的交互体系，重点是：

- 把现有 `guarded task` 抽成稳定协议
- 把“结果回执”从单点 UI 变成统一任务语义
- 把 `doctor` / `audit` 聚成诊断中心 MVP

---

## 3. P0 边界

### 3.1 P0 包含什么

#### P0-A：危险动作统一协议（Guarded Action Protocol）

统一所有高风险操作的执行链路：

- Preview / Dry-run
- Confirm
- Execute
- Receipt
- Audit

首批纳入：

- `rm`
- `mv` / `ren`
- `redirect`
- `env doctor --fix`
- 后续可平滑扩展到 `acl` / `protect` / `encrypt`

#### P0-B：任务中心 / Recipe MVP

把现有任务卡片升级为“统一任务中心”的最小可用版本：

- 所有工作台任务使用统一结果模型
- 提供最近执行任务视图
- 支持“重复执行最近任务”
- 支持最小 Recipe 定义：把一组固定参数任务串起来执行

MVP 不追求复杂编排，只做：

- 单步任务
- 顺序任务
- 结果回执
- 失败即停

#### P0-C：诊断中心 MVP

把现在分散的 `doctor` / `audit` / 运行态检查收进一个明确入口：

- 环境诊断：复用现有 `env doctor`
- 审计时间线：复用现有 `/api/audit`
- 后续可接入端口、代理、文件健康检查

P0 只要求先把“环境诊断 + 审计总览”做稳，不在第一期塞满所有检查项。

### 3.2 P0 明确不做什么

P0 暂不包含：

- 云同步
- 远程节点
- 账号体系
- 插件系统
- 复杂规则引擎
- 跨机器任务编排
- 真正意义上的多阶段事务回滚

P0 的核心是**统一本地控制台骨架**，不是扩张产品边界。

---

## 4. P0 的统一模型

### 4.1 统一任务状态模型

P0 建议所有工作台任务都统一使用以下状态语义：

- `idle`
- `previewing`
- `awaiting_confirm`
- `running`
- `succeeded`
- `partially_succeeded`
- `failed`

### 4.2 统一回执模型

回执至少包含：

- 动作名
- 目标对象
- 是否 dry-run
- 开始时间 / 结束时间 / 耗时
- 执行摘要
- 成功数 / 失败数 / 跳过数
- 关键结果项
- 审计入口 / 日志入口
- 可选下一步建议

### 4.3 统一危险动作判定

P0 中只要满足以下任一条件，就必须走统一保护链路：

- 删除
- 覆盖
- 重命名 / 移动
- 写系统环境变量
- 修复型 doctor
- 权限 / 安全相关修改

---

## 5. 分阶段实施顺序

### Milestone 1：先做统一协议，不先做大而全任务中心

目标：让所有危险动作都说同一种“平台语言”。

建议优先改造：

- `workspace-tools.ts` 的 guarded task 定义
- `TaskToolCard.vue` 的 preview / confirm / execute 状态流
- 后端 guarded preview 响应结构
- `TaskReceiptComponent.vue` 的统一字段渲染

交付标准：

- 至少 3 类危险动作共用同一套确认与回执逻辑
- Preview 失败时不能进入确认态
- Execute 失败时必须有清晰回执

### Milestone 2：接入任务中心 MVP

目标：把“任务执行”从页面内临时动作提升为跨工作台可追踪对象。

建议最小能力：

- 最近任务列表
- 成功 / 失败 / dry-run 过滤
- 任务详情回执
- 重新执行最近任务

交付标准：

- 用户能在总览或诊断页看到最近任务
- 一个任务不依赖所在工作台也能被回看

### Milestone 3：接入 Recipe MVP

目标：让高频本地工作流可以复用，而不是反复手填。

首批 Recipe 只做 3 类：

- 文件清理类
- 环境治理类
- 代理 / 诊断类

建议最小定义能力：

- 名称
- 描述
- 顺序步骤
- 每步参数
- 是否允许 dry-run

交付标准：

- 能保存并执行至少一条顺序 Recipe
- 失败即停，结果进入任务中心

### Milestone 4：落地诊断中心 MVP

已完成。当前已通过 `/api/workspaces/diagnostics/summary` 聚合环境 doctor、审计时间线、最近失败任务与 guarded receipt，并由 `Statistics & Diagnostics` 工作台统一承载。

目标：让 `Statistics & Diagnostics` 真正成为平台入口，而不是“审计列表页”。

首批内容：

- 环境 doctor 结果总览
- 审计时间线
- 最近失败任务
- 最近高风险动作回执入口

交付标准：

- 用户进入诊断中心后，不需要切到其他工作台，也能看清系统当前最值得处理的问题

---

## 6. 后端实施建议

### 6.1 保持“域内 handler + 统一任务响应”

P0 不建议新起一套与现有 Dashboard 平行的协议层。

建议做法：

- 保留现有 `handlers/` 分域结构
- 在危险动作相关 handler 上统一返回“任务回执结构”
- 先从现有已有 `dry-run` 的接口开始对齐

### 6.2 优先统一响应结构，不急着统一所有命令实现

第一步更值得做的是：

- 让 Dashboard 的 guarded 操作返回稳定结构
- 而不是一上来改掉所有 CLI 输出

这样可以减少对现有 CLI 兼容性的冲击。

### 6.3 诊断中心优先消费现有接口

P0 诊断中心优先复用：

- `/api/env/doctor/run`
- `/api/env/doctor/fix`
- `/api/audit`

如果要新增接口，优先加聚合接口，不优先加很多零散小接口。

---

## 7. 前端实施建议

### 7.1 `TaskToolCard` 成为 P0 核心枢纽

P0 前端的关键不是继续铺更多页面，而是让 `TaskToolCard.vue` 变成统一任务入口：

- 参数输入
- 预演
- 确认
- 执行
- 回执

都通过同一条状态流完成。

### 7.2 总览页增加“最近任务”卡片

建议把任务中心 MVP 的入口先放在：

- `OverviewWorkspace`
- `StatisticsDiagnosticsWorkspace`

而不是先单独做一个新一级工作台。

### 7.3 Recipe 先用本地定义，不急着引入复杂编辑器

Recipe MVP 可以先用：

- 固定 JSON 定义
- 少量预置 Recipe
- Dashboard 只负责执行与展示

先不要一开始就做复杂拖拽编排器。

---

## 8. 测试门槛

### 8.1 后端

P0 每个新增 / 统一后的 guarded handler 至少要覆盖：

- preview 成功
- preview 失败
- execute 成功
- execute 失败
- dry-run 不产生副作用

### 8.2 前端

P0 前端至少要覆盖：

- Preview 失败时不能打开确认弹窗
- 确认后才进入执行态
- 执行后回执状态正确
- 回执能区分 dry-run / success / failed

### 8.3 平台约束测试

必须显式写出“绕过保护链路会失败”的测试。

换句话说，测试不只是验证能成功，也要验证：

- 不能跳过 preview
- 不能在 preview 失败后继续 execute
- 不能把危险动作当普通 run task 执行

---

## 9. 完成判定

P0 完成时，应同时满足：

- 危险动作统一走 Preview -> Confirm -> Execute -> Receipt -> Audit
- `TaskReceiptComponent` 不再只是局部 UI，而是稳定任务回执组件
- Dashboard 至少能查看最近任务和最近失败任务
- 至少存在 1 条可运行的 Recipe MVP
- `Statistics & Diagnostics` 能看 doctor 结果、审计时间线和任务失败入口

---

## 10. 推荐的实际开工顺序

如果现在就开始做，我建议顺序严格按下面来：

1. **先统一 guarded task 协议**
2. **再补任务中心最近任务视图**
3. **再接 Recipe MVP**
4. **最后补诊断中心聚合展示**

也就是说：

- 先把“动作语言”统一
- 再把“动作结果”沉淀下来
- 再让结果变成可复用工作流
- 最后把工作流和诊断放进总览视图

---

## 11. 当前一句话执行建议

如果只给一句执行建议，就是：

> **P0 先不要扩命令面，先把危险动作协议、任务回执、最近任务和诊断中心做成统一骨架。**

这样做完之后，后面的 P1 / P2 / P3 才能在同一套平台语言上继续长，而不会越做越散。



