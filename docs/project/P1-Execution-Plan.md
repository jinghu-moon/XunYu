# XunYu P1 实施方案

## 1. 文档目的

这份文档用于定义 `P1` 的实施范围、执行顺序和验收口径，供方案评审与后续落地使用。

如果说 `P0` 解决的是“平台骨架是否统一”，那么 `P1` 要解决的问题是：

> **如何把“文件与安全”工作台从现有的 Diff / Redirect 容器，升级成真正可用的本地文件治理控制台。**

配套文档：

- 路线图：`./Dashboard-Expansion-Roadmap.md`
- P0 实施方案：`./P0-Execution-Plan.md`
- P0 完成说明：`./P0-Completion-Notes.md`
- Dashboard 设计：`./Dashboard-Design.md`

---

## 2. 为什么先做 P1

在路线图里，`Phase 1` 和 `Phase 2` 都指向同一个收益最高的工作域：`Files & Security`。

原因很明确：

- 当前 Dashboard 已经有 `DiffPanel`、`RedirectPanel`、文件浏览、搜索、预览、转换、校验等基础能力。
- 文件相关命令本身是本地控制台价值最强的一组能力：`tree`、`find`、`bak`、`rm`、`lock`、`mv`、`renfile`、`protect`、`encrypt`、`decrypt`、`acl`。
- 这组能力天然适合复用 `P0` 已完成的 Triple-Guard、任务回执、最近任务、Recipe、诊断中心等平台骨架。
- 一旦这里收口成功，后面的 `路径与上下文`、`集成与自动化`、`媒体与转换` 都可以按同一种工作台语言继续扩展。

所以 `P1` 不建议平均发力，而应优先集中在：

- 文件工作流扩张
- 文件治理闭环
- 高风险文件动作统一纳入 Triple-Guard

---

## 3. P1 目标

### 3.1 总目标

把 `Files & Security` 工作台升级为“文件浏览 + 文件操作 + 文件治理 + 审计回执”一体化工作台。

### 3.2 阶段目标

#### P1-A：文件工作流补齐

优先纳入：

- `tree`
- `find`
- `bak`
- `rm`
- 文件批量操作骨架

目标不是增加更多页面，而是让文件工作流在一个工作台内形成完整链路：

- 观察
- 筛选
- 预览
- 执行
- 回执
- 审计

#### P1-B：文件治理闭环

在文件工作流稳定后，补齐治理动作：

- `acl`
- `lock`
- `mv`
- `renfile`
- `protect`
- `encrypt`
- `decrypt`

目标是让“查看文件”和“治理文件”不再分裂在 CLI 与 UI 两套语言中。

#### P1-C：与 P0 骨架深度集成

把本期所有高风险动作接入 P0 已完成的公共能力：

- Triple-Guard
- Task Receipt
- Recent Tasks
- Recipe
- Diagnostics / Audit

---

## 4. P1 范围边界

### 4.1 包含什么

本阶段明确包含：

- `Files & Security` 工作台的一体化容器整理
- 文件树、查找、备份、删除的 UI / API 收口
- 文件批处理骨架
- ACL / Protect / Encrypt / Move / Rename / Lock 的治理闭环设计与接入
- 高风险动作统一预演、确认、结果回执、审计
- 与最近任务、Recipe、诊断中心的联动

### 4.2 明确不做什么

本阶段明确不做：

- 不把文件相关每个 CLI 命令都做成一级导航
- 不做完整的“资源管理器替代品”
- 不做复杂拖拽式批处理编排器
- 不引入远程执行、云同步或多用户权限系统
- 不把媒体与转换、集成与自动化、路径与上下文提前混入本阶段

### 4.3 执行原则

P1 继续严格沿用 P0 已确立的原则：

- Workspace over Command
- Backend First, UI Second
- Triple-Guard for dangerous actions
- No blind direct execute for destructive operations
- Zero breakage for existing CLI behavior

---

## 5. 可直接复用的 P0 基础

### 5.1 前端

P1 可以直接复用的共享组件包括：

- `UnifiedConfirmDialog.vue`
- `TaskReceiptComponent.vue`
- `TaskToolCard.vue`
- `RecentTasksPanel.vue`
- `RecipePanel.vue`
- `DiagnosticsCenterPanel.vue`

### 5.2 后端

P1 可直接复用的接口与任务语义包括：

- guarded preview / execute 协议
- 最近任务查询与安全重放
- Recipe 预演 / 执行 / 保存
- 诊断中心聚合与审计流

### 5.3 现有工作台基础

当前 `Files & Security` 里已经具备可复用的观察型基础：

- `DiffPanel`
- `RedirectPanel`
- 文件浏览 / 搜索 / 预览 / 校验 / 转换能力

因此 P1 不应该另起一套“纯文件页”，而应在现有容器上继续收口。

---

## 6. 推荐实施顺序

## 6.1 Milestone 1：文件工作流主链落地

目标：先把最常用、最直观的文件动作纳入工作台。

本里程碑优先交付：

- `tree`
- `find`
- `bak`
- `rm`
- 文件选择与批量骨架

建议前端形态：

- 顶部保留文件浏览 / 预览容器
- 中部增加 `文件操作任务区`
- 右侧或底部增加 `最近文件任务 / 回执区`
- 批量操作先以“选中列表 + 批量动作卡片”实现，不做重编排器

建议交付标准：

- 用户能够在 `Files & Security` 内完成“查看 -> 选择 -> 预演 -> 执行 -> 回执”完整链路
- 文件工作台不再被误认为只是 Diff 页面

### 6.2 Milestone 2：高风险文件动作统一保护

目标：把文件治理动作全面接入 Triple-Guard。

本里程碑优先交付：

- `rm`
- `mv`
- `renfile`
- `protect`
- `encrypt`
- `decrypt`

要求：

- 所有 destructive / mutating 动作都必须先 preview
- preview 失败不得进入 confirm
- execute 失败必须返回 receipt
- 审计动作可追踪到具体目标与摘要

建议交付标准：

- 文件类高风险动作不再存在 UI 直达执行入口
- 回执、最近任务、审计流三者一致

### 6.3 Milestone 3：ACL / 锁 / 保护 / 加解密治理闭环

目标：把“文件与安全”里的治理能力真正做成闭环，而不是零散任务卡。

本里程碑优先交付：

- `acl`
- `lock`
- `protect`
- `encrypt`
- `decrypt`

建议前端形态：

- `ACL 治理面板`
- `保护规则面板`
- `加解密任务面板`
- `锁检测 / 冲突提示面板`

建议交付标准：

- 用户可在同一工作台内查看治理对象、做预演、确认执行并查看回执
- 高风险治理动作具备统一的 preview / confirm / receipt / audit 链路

### 6.4 Milestone 4：批处理与任务联动稳定化

目标：把本期能力接入 P0 的平台级基础设施，形成稳定闭环。

需要补齐：

- 批量动作回执聚合
- 最近任务可重放
- Recipe 可复用文件工作流
- 诊断中心可看到失败任务与高风险回执
- 文档与说明同步更新

建议交付标准：

- 任何文件治理动作完成后，用户都能在任务中心和诊断中心回看结果
- 高频文件工作流可以被 Recipe 固化

---

## 7. 后端设计建议

### 7.1 路由组织

P1 不建议继续把能力零散挂在过多旧路由上，建议按工作域收口。

推荐优先考虑的工作台级接口分组：

#### 文件工作流

- `POST /api/workspaces/files/tree/query`
- `POST /api/workspaces/files/find/query`
- `POST /api/workspaces/files/bak/preview`
- `POST /api/workspaces/files/bak/execute`
- `POST /api/workspaces/files/rm/preview`
- `POST /api/workspaces/files/rm/execute`
- `POST /api/workspaces/files/batch/preview`
- `POST /api/workspaces/files/batch/execute`

#### 文件治理

- `POST /api/workspaces/files/move/preview`
- `POST /api/workspaces/files/move/execute`
- `POST /api/workspaces/files/rename/preview`
- `POST /api/workspaces/files/rename/execute`
- `POST /api/workspaces/files/protect/preview`
- `POST /api/workspaces/files/protect/execute`
- `POST /api/workspaces/files/encrypt/preview`
- `POST /api/workspaces/files/encrypt/execute`
- `POST /api/workspaces/files/decrypt/preview`
- `POST /api/workspaces/files/decrypt/execute`
- `POST /api/workspaces/files/lock/check`
- `POST /api/workspaces/files/acl/preview`
- `POST /api/workspaces/files/acl/execute`

### 7.2 路由设计原则

- 优先返回统一任务语义，而不是为每个命令重新发明响应结构
- 高风险动作必须有 `preview` 和 `execute` 两段式接口
- 允许只读查询与变更动作分离
- 尽量保留 CLI 现有行为，不强迫修改命令输出协议

### 7.3 审计与回执

P1 应继续复用现有任务记录与审计能力，不建议再做一套平行日志。

要求：

- 文件操作回执进入最近任务
- 高风险动作写入审计流
- 失败任务能在诊断中心聚合展示

---

## 8. 前端设计建议

### 8.1 工作台结构

`Files & Security` 建议拆成 4 个稳定子区：

- 文件浏览与预览区
- 文件操作任务区
- 文件治理区
- 回执与最近任务区

### 8.2 组件策略

优先复用，不新造同类组件：

- 普通任务：继续走 `TaskToolCard`
- 危险动作：继续走 `UnifiedConfirmDialog` + `TaskReceiptComponent`
- 历史结果：继续走 `RecentTasksPanel`
- 高频工作流：继续走 `RecipePanel`

### 8.3 交互策略

- 观察型能力保留在面板中，例如浏览、搜索、预览、ACL 查看
- 工作流型能力收口到任务卡，例如删除、移动、重命名、加解密
- 批量动作优先走“先选中，再统一预演”的模式
- 任何 destructive action 都不得保留“单击立即执行”入口

---

## 9. 测试门槛

### 9.1 Rust 后端

P1 每个新接入或改造后的 handler 至少覆盖：

- preview 成功
- preview 失败
- execute 成功
- execute 失败
- dry-run / preview 不产生副作用
- 绕过 confirm 无法执行

对批量动作还应额外覆盖：

- 部分目标失败时的回执结构
- 失败即停或部分成功策略是否符合约定
- 审计记录是否完整写入

### 9.2 Vue 前端

P1 前端至少覆盖：

- preview 失败时不能进入确认态
- confirm 后才会执行危险动作
- receipt 能正确区分 success / failed / dry_run
- 批量动作的选中、预演、确认、回执链路完整
- 最近任务重放继续遵循安全约束

### 9.3 平台约束测试

必须显式测试“不能绕过 Triple-Guard”。

尤其在这些动作上必须写负向测试：

- `rm`
- `mv`
- `renfile`
- `protect`
- `encrypt`
- `decrypt`
- `acl`

---

## 10. 完成判定

当以下条件同时满足时，可认为 `P1` 达标：

- `Files & Security` 已经成为真正的文件治理工作台，而非 Diff 容器
- 文件操作主链支持查看、选择、预演、执行、回执、审计
- 高风险文件动作统一进入 Triple-Guard
- 批量动作已有最小可用骨架
- 最近任务、Recipe、诊断中心都能消费本阶段结果
- 文档、使用说明和工作台入口已经同步更新

---

## 11. 当前一句话执行建议

如果只给一句执行建议，就是：

> **P1 不要平均铺开所有工作台，而是优先把 `Files & Security` 做成真正的文件治理控制台，并把所有高风险文件动作纳入统一 Triple-Guard。**