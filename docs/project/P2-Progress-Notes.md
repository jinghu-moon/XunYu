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
- `dashboard-ui/src/components/RecentTasksPanel.test.ts`

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





