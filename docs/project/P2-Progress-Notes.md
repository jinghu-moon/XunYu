# XunYu P2 进展说明

## 1. 本次目标

`P2` 的第一刀先不急着铺开所有治理动作执行流，而是优先补齐“治理观察面”。

本次新增的是 `Files & Security` 工作台内的 **治理快照面板**，用于围绕当前选中文件集中展示：

- 锁占用快照
- 保护规则快照
- ACL 摘要

这样做的原因很简单：

- `P1` 已经把目录 / 文件上下文桥接到任务卡
- `P2` 要先让用户在同一工作台里看清治理对象当前状态
- 在状态可见的前提下，再推进 `protect / encrypt / decrypt / acl` 的更强执行闭环更稳妥

---

## 2. 已完成内容

### 2.1 新增治理快照面板

新增组件：

- `dashboard-ui/src/components/FileGovernancePanel.vue`

当前能力：

- 以当前选中文件为治理对象
- 手动刷新治理快照，避免每次选择都向最近任务写入噪音
- 聚合三类只读查询：
  - `lock who`
  - `protect status`
  - `acl view`
- 在工作台中直接查看命令输出与耗时
- 对未启用的 feature 给出明确提示

### 2.2 工作台集成

已接入：

- `dashboard-ui/src/components/workspaces/FilesSecurityWorkspace.vue`

当前 `Files & Security` 侧栏结构已经变成：

- 文件上下文桥接
- 治理快照
- 最近任务
- Recipes

这让 `Files & Security` 更接近“文件治理控制台”，而不再只是任务卡集合。

### 2.3 治理预演解释层

本轮继续沿着 `P2` 的建议方向推进，为文件治理里的高风险动作补了一层 **Dashboard 友好的解释视图**。

新增内容：

- `dashboard-ui/src/components/FileGovernanceSummary.vue`
- `dashboard-ui/src/components/file-governance-summary.ts`

当前已覆盖的动作：

- `protect:set`
- `protect:clear`
- `acl:purge`
- `acl:inherit`
- `acl:owner`
- `acl:repair`
- `acl:add`
- `acl:diff`
- `encrypt`
- `decrypt`

解释层输出不替代原始 stdout，而是在任务卡中额外补出：

- 当前治理对象
- 即将发生或已经完成的变更语义
- 关键参数（如 deny / require / principal / rights / 输出路径）
- 预演边界提示（例如 `encrypt / decrypt` 当前预演只做规则测试，不会真实改写文件）

这样用户在 Triple-Guard 的 preview 阶段，不需要先读完整原始输出，也能快速判断本次治理动作是不是自己想要的。

### 2.4 批量 protect Triple-Guard 闭环

本轮继续按“从底层到消费层”的顺序，把 `Files & Security` 里的批量治理从“队列骨架”推进到了第一版可执行闭环。

新增内容：

- `dashboard-ui/src/components/file-governance-batch.ts`
- `dashboard-ui/src/components/BatchGovernancePanel.vue`
- `dashboard-ui/src/components/file-governance-batch.test.ts`
- `dashboard-ui/src/components/BatchGovernancePanel.test.ts`
- `dashboard-ui/src/components/FileGovernanceSummary.test.ts`

当前批量治理范围：

- `protect:set`
- `protect:clear`
- `acl:purge`
- `acl:inherit`
- `acl:owner`
- `acl:repair`

闭环行为：

- 基于现有 guarded 单任务协议批量生成 preview 请求
- 对批量队列逐项做 dry-run 预演
- 汇总为一个统一确认弹窗
- 只有全部预演 ready 时才允许确认执行
- 执行阶段逐项回收 receipt / audit 信息

这样做的取舍是：

- 不臆造新的后端批量 API
- 先复用现有 `previewGuardedTask / executeGuardedTask`
- 先把最稳定的 `protect:set / clear` 做成闭环，再逐步扩到 ACL 与加解密治理

### 2.5 ACL 运维矩阵扩展

在批量 `protect` 闭环之后，本轮继续把 CLI 里已经具备、且适合非交互调用的 ACL 能力补到 Dashboard。

新增任务定义：

- `acl:effective`
- `acl:backup`
- `acl:copy`
- `acl:restore`
- `acl:purge`
- `acl:inherit`
- `acl:owner`
- `acl:repair`

对应补齐内容：

- 当前文件上下文预填会自动带入这些 ACL 任务
- `FileGovernanceSummary` 新增 ACL 覆盖 / 恢复 / 清理 / 继承 / Owner / 修复等摘要解释
- 批量治理面板已扩到：
  - `protect:set`
  - `protect:clear`
  - `acl:purge`
  - `acl:inherit`
  - `acl:owner`
  - `acl:repair`

本轮仍然遵守同一个取舍：

- 优先接入 CLI 已验证的非交互参数形态
- 不把依赖交互式选择的 `acl remove` 强行塞进 Dashboard
- 先把可 dry-run / 可确认 / 可回执的动作打通，再考虑更复杂的 ACL 编辑器

### 2.6 doctest / 文档示例体检

已统一扫描 `src/` 中 Rust 文档代码块。

结果：

- 真正的代码块很少，主要在 ACL 子模块
- 仅发现 `src/acl/privilege.rs` 的内部 helper 示例会触发不合理 doctest 编译
- 已将其从 `no_run` 调整为 `ignore`

这样做的原则是：

- 不为了 doctest 通过而扩大内部 API 可见性
- 只修正文档测试策略，不改变业务行为

### 2.7 最近任务治理解释层闭环

本轮继续把 `P2` 的治理解释层推进到消费端，避免治理任务在最近任务里退化回“只剩原始 stdout”。

新增内容：

- `dashboard-ui/src/components/recent-task-governance.ts`
- `dashboard-ui/src/components/RecentTasksPanel.vue`

当前行为：

- `RecentTasksPanel` 会对 `files-security` 的治理任务回放请求做参数还原
- 选中历史记录时，若动作属于治理摘要支持范围，会自动渲染 `FileGovernanceSummary`
- 重新执行后的 `runResult` 与 guarded `receipt` 也会复用同一解释层
- `acl:effective` 已纳入治理摘要支持范围

这样做的收益是：

- 用户在任务中心回看 ACL / Protect / Encrypt 结果时，不必重新手读完整原始输出
- Triple-Guard 的“预演 -> 执行 -> 回执”解释语言，在任务卡与最近任务之间保持一致

---

### 2.12 诊断中心聚合治理 / 失败 / 审计

本轮把 `Statistics & Diagnostics` 工作台继续往“诊断控制台”方向推进，补上一个统一的 **诊断中心面板**，把之前分散在最近任务、审计与 Doctor 输出里的关键信号先聚合起来。

新增内容：

- `dashboard-ui/src/components/DiagnosticsCenterPanel.vue`
- `dashboard-ui/src/components/DiagnosticsCenterPanel.test.ts`
- `dashboard-ui/src/components/statistics-diagnostics-focus.ts`

当前能力：

- 聚合展示 5 类诊断信息：
  - `Doctor`
  - `治理预警`
  - `失败任务`
  - `危险回执`
  - `审计时间线`
- 治理预警支持两层筛选：
  - 治理族：`all / acl / protect / crypt / other`
  - 状态：`all / failed / succeeded / previewed`
- 每条治理记录尽量复用已有的 `FileGovernanceSummary` 结构化摘要
- 对 Doctor 聚合结果补出概览卡片：
  - `紧急项`
  - `Doctor 问题`
  - `最近失败`
  - `危险回执`
  - `治理预警`
  - `审计条目`
- 导航条支持在同一面板内快速跳到目标诊断分区

这样做的取舍：

- 优先复用现有 Recent Tasks / Audit / Doctor API
- 不新增新的 CLI 交互形态，只做 Dashboard 诊断聚合
- 先把“看得见异常”做扎实，再往“可回跳、可重放、可追审计”推进

验证点：

- 诊断中心能正确拉起诊断摘要接口
- 治理预警可按治理族 / 状态筛选
- ACL / Protect / Encrypt 的治理摘要能在同一面板内并排查看
- 审计时间线与失败任务能共享同一套聚焦入口

### 2.13 打通诊断中心与 Recent Tasks / Audit 焦点联动

在 `2.12` 的聚合基础上，本轮继续把“看见问题”推进为“跳到消费面板继续处理”。

新增内容：

- `RecentTasksPanel` 支持 `focusRequest`
- `AuditPanel` 支持 `focusRequest`
- `StatisticsDiagnosticsWorkspace` 负责统一承接跨面板焦点事件

联动方式：

- 诊断中心可以把治理预警 / 失败任务 / 危险回执回跳到：
  - `最近任务`
  - `审计面板`
- 焦点请求会携带最小必要上下文：
  - `selected_task_id`
  - `target`
  - `audit_action`
  - `result`
  - `governance_family / governance_status`

这样做的原因：

- 不再要求用户自己手动复制 task id 或 audit action
- 让诊断面板真正成为“索引入口”，而不是只读仪表盘
- 保持 API 仍然简单，状态主要由工作台内的 focus request 传递

验证点：

- 诊断中心点击“回到最近任务”能准确定位到选中任务
- 点击“查看审计”能带着 `audit_action + target + result` 过滤审计列表
- 焦点切换不会打断现有 Recent Tasks / Audit 的独立刷新逻辑

### 2.14 打通 Recipe / TaskToolbox 到 Recent Tasks / Audit

本轮继续沿着“从底层到消费层”的顺序，把 Recipe 与任务卡的执行结果也纳入同一套统计与诊断消费流。

新增内容：

- 扩展 `StatisticsWorkspaceLinkPayload`，统一承载任务卡 / Recipe 的跨面板跳转事件
- `TaskToolCard` 在普通执行、guarded confirm 回执后，都能直接联动：
  - `回到最近任务`
  - `查看审计`
- `TaskToolbox` 向工作台继续透传 link 事件
- `RecipePanel` 在每个 step receipt 上补齐相同的跳转按钮

这样做之后：

- 不论入口来自单任务卡、批量治理、危险动作回执还是 Recipe 步骤
- 统计与诊断工作台都能消费统一的 link payload
- Triple-Guard 的 preview / confirm / receipt 路径与统计消费层保持一致

验证点：

- `TaskToolCard` 的执行结果区与回执区都会发出正确的 link payload
- `RecipePanel` 的 step receipt 能回跳到对应最近任务与审计记录
- `StatisticsDiagnosticsWorkspace` 能正确把不同来源的 link 统一路由到 Recent Tasks / Audit
### 2.15 Recent Tasks 聚焦与安全重放

在 `2.14` 的治理解释层稳定后，本轮继续把任务结果消费层做实，先把 `RecentTasksPanel` 从“被动列表”推进成可聚焦、可重放、可回链的工作台面板。

新增内容：

- `dashboard-ui/src/types.ts`
- `dashboard-ui/src/components/RecentTasksPanel.vue`
- `dashboard-ui/src/components/TaskToolCard.vue`
- `dashboard-ui/src/components/RecipePanel.vue`
- `dashboard-ui/src/components/RecentTasksPanel.test.ts`
- `dashboard-ui/src/components/TaskToolCard.test.ts`
- `dashboard-ui/src/components/RecipePanel.test.ts`
- `dashboard-ui/src/components/workspaces/StatisticsDiagnosticsWorkspace.test.ts`

当前效果：

- `RecentTasksPanel` 可以通过 `RecentTasksFocusRequest` 直接聚焦到 `status / dry_run / search / action`
- `TaskToolCard` 在任务执行完成后，可按 `action + target(search)` 生成最近任务跳转 payload
- `RecipePanel` 在 step 预演 / 执行后，也能把结果回链到最近任务
- `StatisticsDiagnosticsWorkspace` 持有本地 focus state，不需要引入全局 store

验证点：

- `RecentTasksPanel` 能正确消费 focus request 并刷新过滤状态
- 任务卡 / Recipe 发出的 `recent-tasks` 事件可在统计工作台内落地
- 安全重放仍然遵循既有的 run / guarded 协议，不绕过 Triple-Guard

---

### 2.16 Audit 聚焦与审计时间线回看

在最近任务聚焦稳定后，本轮继续把 `AuditPanel` 接入同一套 focus 语义，让“最近任务 -> 审计复盘”可以在工作台内闭环。

新增内容：

- `dashboard-ui/src/components/RecentTasksPanel.vue`
- `dashboard-ui/src/components/AuditPanel.vue`
- `dashboard-ui/src/components/RecentTasksPanel.test.ts`
- `dashboard-ui/src/components/AuditPanel.test.ts`

当前效果：

- `RecentTasksPanel` 支持在明细中直接跳向诊断中心
- `AuditPanel` 支持通过 `AuditFocusRequest` 聚焦到 `search / action / result`
- `AuditPanel` 在 focus request 变化后会自动重新加载数据
- 最近任务与审计时间线共享同一套“从结果追到原因”的消费方向

验证点：

- `RecentTasksPanel` 的 focus request 会同步筛选条件
- `AuditPanel` 的 focus request 会同步筛选条件并触发 reload
- 工作台内可以按失败结果或特定动作快速回看审计时间线

---

### 2.17 Recent Tasks / Audit 联动 Diagnostics Center

在 `2.16` 的面板内聚焦能力之上，本轮继续把消费层再向前推进一步：让最近任务和审计记录都可以直接把用户送到 `DiagnosticsCenterPanel`。

新增内容：

- `dashboard-ui/src/types.ts`
- `dashboard-ui/src/components/statistics-diagnostics-focus.ts`
- `dashboard-ui/src/components/DiagnosticsCenterPanel.vue`
- `dashboard-ui/src/components/RecentTasksPanel.vue`
- `dashboard-ui/src/components/AuditPanel.vue`
- `dashboard-ui/src/components/workspaces/StatisticsDiagnosticsWorkspace.vue`
- `dashboard-ui/src/components/DiagnosticsCenterPanel.test.ts`
- `dashboard-ui/src/components/RecentTasksPanel.test.ts`
- `dashboard-ui/src/components/AuditPanel.test.ts`
- `dashboard-ui/src/components/workspaces/StatisticsDiagnosticsWorkspace.test.ts`

当前效果：

- `StatisticsWorkspaceLinkPayload` 增加 `diagnostics-center` 语义
- `RecentTasksPanel` 可把失败任务、治理任务映射到诊断中心聚焦请求
- `AuditPanel` 可把治理审计条目映射到诊断中心聚焦请求
- `DiagnosticsCenterPanel` 支持通过 `focusRequest` 聚焦 `failed / guarded / governance / audit` 等视图
- `StatisticsDiagnosticsWorkspace` 统一编排 `diagnostics-center / recent-tasks / audit` 三类 focus state

验证点：

- `RecentTasksPanel` 的失败样例能 emit 到 `diagnostics-center`
- `AuditPanel` 的治理审计条目能 emit 到 `diagnostics-center`
- `DiagnosticsCenterPanel` 能消费反向 focus request 并落到对应子区
- `StatisticsDiagnosticsWorkspace` 能正确协调三个消费层之间的焦点切换

---

### 2.18 批量治理回填到文件任务中心

本轮继续把 `P2` 的消费闭环从统计工作台延伸回 `Files & Security` 本身，避免批量治理完成后还要手动去最近任务里重新筛动作与路径。

新增内容：

- `dashboard-ui/src/components/file-governance-batch.ts` 新增批量预演 / 回执 -> 最近任务 focus helper
- `dashboard-ui/src/components/BatchGovernancePanel.vue` 为预演项与执行回执增加“回到最近任务”入口
- `dashboard-ui/src/components/workspaces/FilesSecurityWorkspace.vue` 持有最近任务 focus state，并桥接批量治理事件

验证点：

- 批量预演项可按 `previewed + dry-run + action + path` 回填到 `RecentTasksPanel`
- 批量执行回执可按 `succeeded|failed + executed + action + path` 回填到 `RecentTasksPanel`
- 同工作台内可直接完成“批量治理 -> 最近任务复盘”的闭环，不必跨页手动重筛

### 2.19 批量治理直达审计 / 诊断消费层

本轮继续沿着“从底层到消费层”的方向，把 `BatchGovernancePanel` 从“只能回到最近任务”推进成能直接跳转到全局消费层。

新增内容：

- `dashboard-ui/src/components/file-governance-batch.ts` 新增批量预演 -> 诊断中心、批量回执 -> 审计筛选的 payload helper
- `dashboard-ui/src/components/BatchGovernancePanel.vue` 新增：
  - 预演项进入诊断中心
  - 回执项进入审计面板
- `dashboard-ui/src/components/workspaces/FilesSecurityWorkspace.vue` 补齐批量治理面板的 `link-panel` 桥接

当前效果：

- 批量预演项除了“回到最近任务”，还可以直接进入 `Statistics & Diagnostics` 的治理诊断视图
- 批量执行回执除了“回到最近任务”，还可以直接跳到 `AuditPanel` 的动作 / 结果筛选态
- `Files & Security` 工作台内完成批量治理后，不需要用户手工切页、手工筛 action / target

验证点：

- 批量预演能 emit `diagnostics-center` payload
- 批量执行回执能 emit `audit` payload
- `FilesSecurityWorkspace` 能继续本地消费 `focus-recent-tasks`，同时把 `link-panel` 上抛到全局桥

---

### 2.20 批量治理执行回执直达治理诊断

在 2.19 的基础上，继续把“执行后复盘”补到位：除了跳转 `AuditPanel`，批量治理的执行回执现在也能一键把上下文带到 `DiagnosticsCenterPanel` 的治理视图。

新增内容：

- `dashboard-ui/src/components/file-governance-batch.ts` 新增 `createDiagnosticsLinkFromBatchReceipt`
- `dashboard-ui/src/components/BatchGovernancePanel.vue` 在执行回执区新增“进入诊断中心”按钮
- `dashboard-ui/src/components/BatchGovernancePanel.test.ts` 增加执行回执 -> 诊断中心事件测试
- `dashboard-ui/src/components/workspaces/FilesSecurityWorkspace.test.ts` 增加工作台上抛治理诊断链接测试

当前效果：

- 同一批量执行回执可同时服务三类消费：最近任务、审计、治理诊断
- 用户可以从失败或成功回执直接进入治理诊断视图，不再手工转抄 `target / action / status`
- `Files & Security` -> `Statistics & Diagnostics` 的治理闭环进一步收口

验证点：

- helper 层正确生成 `diagnostics-center + governance` payload
- 面板层点击执行回执“进入诊断中心”会 emit 正确事件
- 工作台层继续保持 `link-panel` 透传，不破坏现有审计跳转

---
### 2.21 ACL 差异视图文案修复

补齐批量治理消费链后，顺手修复了 `AclDiffDetails.vue` 的乱码文案问题，避免 ACL 差异视图虽然有结构化数据、但界面标签不可读。

本轮调整：

- 统一恢复 `AclDiffDetails.vue` 中的中文标签、状态文案与空态提示
- 补充 `AclDiffDetails.test.ts` 对标题、表头、状态徽标、前后差异面板标题的断言
- 保持原有结构化 `acl_diff / acl_diff_transition` 渲染模型不变，只修复显示层

效果：

- `acl:diff`、`acl:copy` 等返回结构化 ACL 差异详情的任务，在 Dashboard 内可直接读懂
- 差异状态从“技术可用”提升为“面向操作的可读”

---
### 2.22 ACL 参考路径与治理差异快照

在批量治理消费链和差异组件文案修复之后，本轮继续把 `Files & Security` 的 ACL 治理入口往前收口：用户现在可以直接在工作台里指定一个 `ACL 参考路径`，并把当前选择同步到 `acl:diff / acl:copy` 任务卡，同时在治理快照里直接看到结构化 `acl:diff` 结果。

本轮调整：

- `dashboard-ui/src/components/workspaces/FilesSecurityWorkspace.vue`
  - 新增 `ACL 参考` 状态
  - 新增 `设为 ACL 参考` / `同步 ACL 对比` 按钮
  - 把当前选择 + 参考路径同步到 `acl-diff` 与 `acl-copy` 任务预设
- `dashboard-ui/src/components/FileGovernancePanel.vue`
  - 新增 `aclReferencePath` 输入上下文
  - 刷新治理快照时，在已有 `lock / protect / acl:view` 之外，额外运行 `acl:diff`
  - 优先渲染结构化 `AclDiffDetails`，不再退化为只看原始输出
- `dashboard-ui/src/components/FileGovernancePanel.test.ts`
  - 覆盖无参考路径与有参考路径两种刷新路径
- `dashboard-ui/src/components/workspaces/FilesSecurityWorkspace.test.ts`
  - 覆盖 ACL 参考设置与 `acl-diff / acl-copy` 预设同步

效果：

- 用户可以先把某个文件设为 ACL 基线，再切到另一个文件一键同步 `acl:diff / acl:copy`
- 治理快照不再局限于 ACL 摘要，而是能直接消费结构化 ACL 差异视图
- `工作台上下文 -> 差异对比 -> 高风险任务卡 -> 回执 / 审计` 形成更顺手的单工作台闭环

---
### 2.23 Guarded ACL 预期态差异详情

本轮把 `Files & Security` 中几类高风险 ACL 治理动作的结构化详情补到了 **后端 preview / receipt 层**，不再只让前端看到原始 stdout。

覆盖动作：

- `acl:add`
- `acl:purge`
- `acl:owner`
- `acl:inherit --disable`

后端调整：

- `src/commands/dashboard/handlers/workspaces.rs`
  - 为 guarded ACL 动作新增内部 forecast 结构
  - 在 preview 阶段读取目标 ACL 快照，并在内存里推导“预期状态”
  - 在 execute receipt 阶段读取实际 ACL，再输出 `acl_diff_transition`
- 继续复用既有结构：
  - `acl_diff`
  - `acl_diff_transition`
- 不新增前端协议，也不引入新的 details kind

效果：

- `acl:add / acl:purge / acl:owner / acl:inherit` 的 preview 不再只是“命令会执行什么”，还能直接看到“目标 ACL 与预期状态还差什么”
- receipt 阶段可同时展示“执行前差异 / 执行后差异”，更容易判断 Triple-Guard 是否真正收敛到目标状态
- 前端消费层零额外扩散，`AclDiffDetails` / `RecentTasksPanel` / `DiagnosticsCenterPanel` 直接复用既有结构

---

### 2.24 确认弹窗治理解释层接入

本轮把已经补齐的结构化治理解释层正式接到了 **确认阶段**，避免高风险动作在 Triple-Guard 的第二步仍退化成“只看原始 stdout”。

本轮调整：

- `dashboard-ui/src/components/UnifiedConfirmDialog.vue`
  - 新增 `preview-extra` 扩展槽
  - 确认弹窗支持在 preview 元信息下方插入结构化补充层
- `dashboard-ui/src/components/TaskToolCard.vue`
  - 单任务 guarded preview 现在会在确认弹窗中直接显示 `FileGovernanceSummary`
- `dashboard-ui/src/components/RecentTasksPanel.vue`
  - 最近任务 replay 的 guarded preview 也会在确认前显示治理解释层
- `dashboard-ui/src/components/BatchGovernancePanel.vue`
  - 批量治理确认弹窗新增逐项结构化治理摘要，确认前可逐项核对路径、状态与预期变更

验证点：

- `dashboard-ui/src/components/UnifiedConfirmDialog.test.ts` 覆盖扩展槽渲染
- `dashboard-ui/src/components/TaskToolCard.test.ts` 断言确认弹窗内存在单任务治理摘要
- `dashboard-ui/src/components/RecentTasksPanel.test.ts` 断言 replay 确认弹窗内存在 ACL Owner 预演摘要与差异提示
- `dashboard-ui/src/components/BatchGovernancePanel.test.ts` 断言批量确认弹窗内存在逐项治理摘要

效果：

- Triple-Guard 的 confirm 阶段不再只依赖原始命令输出，而是能直接消费结构化治理语义
- 单任务、最近任务重放、批量治理三条入口的确认体验保持一致
- 后续若继续扩展新的 guarded 工作流，只需复用 `preview-extra` 插槽即可

---

### 2.25 ACL Restore 结构化差异与批量加解密联动补齐

本轮继续按“从底层到消费层”的顺序，把上一轮已经接进确认弹窗的治理解释层再往前补齐一段：

- 后端补上 `acl:restore` 的结构化 forecast / receipt 差异
- 前端把批量治理里 `acl:restore / encrypt / decrypt` 的确认与回执消费链补成可测闭环

本轮调整：

- `src/commands/dashboard/handlers/workspaces.rs`
  - 为 `acl:restore` 增加基于备份快照的 forecast 推导
  - preview 阶段返回 `acl_diff`
  - execute receipt 阶段返回 `acl_diff_transition`
  - 补充 `guarded_acl_restore_receipt_includes_diff_transition` 测试
- `dashboard-ui/src/components/file-governance-summary.ts`
  - 修正 `ACL 恢复预演摘要` 的说明文案
  - 明确区分 CLI 只校验路径，与 Dashboard 会额外读取备份快照推导预期 ACL 的边界
- `dashboard-ui/src/components/BatchGovernancePanel.vue`
  - 为批量动态字段补齐稳定的 `batch-field-*` 测试钩子
  - 覆盖 `textarea / select / checkbox / input` 四类控件
- `dashboard-ui/src/components/BatchGovernancePanel.test.ts`
  - 补齐 `acl:restore` 在确认弹窗与执行回执中的结构化差异断言
  - 补齐 `encrypt / decrypt` 在批量工作流中的确认 / 回执摘要断言
- `dashboard-ui/src/components/FileGovernanceSummary.test.ts`
  - 新增 `acl:restore` 预演摘要测试，锁定说明文案与差异明细展示

效果：

- `acl:restore` 不再只暴露原始 stdout，而是和 `acl:copy / acl:add / acl:purge / acl:owner / acl:inherit` 一样具备结构化差异可视化
- 批量治理在 `acl:restore / encrypt / decrypt` 场景下，确认阶段与执行回执阶段都能稳定消费治理解释层
- Triple-Guard 在批量 ACL 恢复与批量加解密场景中的体验保持一致

验证结果：

- `npm run test -- BatchGovernancePanel`
- `npm run test -- FileGovernanceSummary`
- `npm run test`
- `npm run build`
- `cargo test --features dashboard`

---
---

## 3. 测试结果

本次已通过：

- `cargo test --features dashboard --doc`
- `cargo test --features dashboard`
- `npm run test`
- `npm run build`

前端新增覆盖：

- `dashboard-ui/src/components/FileGovernancePanel.test.ts`
- `dashboard-ui/src/components/FileGovernanceSummary.test.ts`
- `dashboard-ui/src/components/workspaces/FilesSecurityWorkspace.test.ts`
- `dashboard-ui/src/components/TaskToolCard.test.ts`
- `dashboard-ui/src/components/BatchGovernancePanel.test.ts`
- `dashboard-ui/src/components/file-governance-batch.test.ts`
- `dashboard-ui/src/components/DiagnosticsCenterPanel.test.ts`
- `dashboard-ui/src/components/RecentTasksPanel.test.ts`
- `dashboard-ui/src/components/AuditPanel.test.ts`
- `dashboard-ui/src/components/TaskToolbox.test.ts`
- `dashboard-ui/src/components/workspaces/StatisticsDiagnosticsWorkspace.test.ts`

验证点包括：

- 无选中文件时的占位与禁用态
- 治理快照刷新时会调用 `lock / protect / acl` 只读任务
- 高风险治理动作会在任务卡中渲染解释层，而不是只展示原始 stdout
- `protect:set / clear`、`acl:add`、`encrypt / decrypt` 的关键参数会被提炼为摘要
- `encrypt / decrypt` 的预演边界会被明确提示，防止把规则测试误解为真实执行
- 工作台集成后能正确接收当前治理对象路径

---

## 4. 下一步建议

`P2` 后续建议继续按下面顺序推进：

1. `ACL` 变更前后差异视图，而不只是原始输出
2. 治理回执与诊断中心的更强联动
3. 更完整的 ACL / Protect 批量治理工作流
4. Encrypt / Decrypt / ACL 的批量 Triple-Guard 闭环

如果继续推进，下一刀最值得做的是：

> **把 `acl:add / remove / purge / inherit / owner` 进一步收口成“治理计划 + 变更差异 + 回执审计”的完整闭环，而不再只是单任务卡。**









