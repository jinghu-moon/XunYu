# XunYu P1 完成说明

## 1. 本期目标

`P1` 的目标是把 `Files & Security` 从“Diff / Redirect 拼盘”推进成真正可操作的文件治理工作台。

本次已完成的重点不是新开一级页面，而是在现有 `Files & Security` 工作台里补齐以下主链：

- 文件管理器上下文 -> 任务卡预填
- 批量目标收集 -> 查找 / 备份骨架
- 工作台内最近任务收口
- 工作台内 Recipe 收口
- `Recent Tasks` 后端按 workspace 过滤

---

## 2. 已完成内容

### 2.1 Files & Security 工作台收口

已将 `dashboard-ui/src/components/workspaces/FilesSecurityWorkspace.vue` 升级为新的文件治理容器：

- 左侧继续保留 `DiffPanel` 与 `RedirectPanel`
- 右侧新增“文件上下文桥接”面板
- 右侧收入口径一致的 `RecentTasksPanel` 与 `RecipePanel`
- 下方保留全部文件与安全任务卡，但已支持来自文件管理器的一键预填

这意味着：

- 当前目录可以直接填入 `tree / find / bak`
- 当前选中文件可以直接填入 `rm / mv / ren / acl / protect / encrypt / decrypt`
- 不再需要手工复制路径到多个任务卡中反复粘贴

### 2.2 文件上下文桥接

已将 `DiffFileManager -> DiffPanel -> FilesSecurityWorkspace` 的状态链路打通：

- `DiffFileManager` 会向上发出当前目录与当前选中文件
- `DiffPanel` 仅做透传，不引入额外业务耦合
- `FilesSecurityWorkspace` 负责把这些状态转成任务预填映射

### 2.3 任务卡预填机制

已为任务卡系统补齐外部预填能力：

- `TaskToolbox.vue` 支持按 task id 透传预填数据
- `TaskToolCard.vue` 支持在 `presetVersion` 变化时应用外部表单值
- 原有 Triple-Guard、确认弹窗、回执组件保持不变

这次改动的价值是：

- 不改破坏性动作协议
- 不新造第二套任务执行体系
- 仅补充“工作台上下文 -> 任务表单”的桥梁

### 2.4 批量操作骨架

已在 `FilesSecurityWorkspace` 内提供最小可用批量骨架：

- 把当前选中文件加入批量队列
- 将批量队列一键填入 `find.paths`
- 将批量队列一键填入 `bak-create.include`
- 支持队列项移除与整体清空

本期仍然遵守 `YAGNI`：

- 先做“选中列表 + 批量预填”
- 不提前实现复杂拖拽编排器
- 不把危险批处理一次性做成不可控的大开关

### 2.5 工作台内 Recent Tasks / Recipe 收口

已补齐面板级筛选能力：

- `RecentTasksPanel.vue` 新增 `workspace` 维度
- 后端 `/api/workspaces/tasks/recent` 支持 `workspace` 查询参数
- `RecipePanel.vue` 新增 `category` 维度前端筛选
- `FilesSecurityWorkspace` 内只展示 `files-security` 域任务与 Recipes

这使得 `Files & Security` 工作台不再混入其它工作域的噪音。

---

## 3. 测试与验证

### 3.1 前端

已新增 / 更新以下测试：

- `dashboard-ui/src/components/TaskToolCard.test.ts`
- `dashboard-ui/src/components/RecentTasksPanel.test.ts`
- `dashboard-ui/src/components/RecipePanel.test.ts`
- `dashboard-ui/src/components/workspaces/FilesSecurityWorkspace.test.ts`

验证点覆盖：

- 任务卡外部预填只在版本变更时生效
- Recent Tasks 支持 workspace 过滤参数
- RecipePanel 支持 category 过滤
- Files 工作台可把目录 / 文件 / 批量队列同步为任务预填
- 危险动作依旧必须先 preview 再 confirm

### 3.2 后端

已新增 Rust 测试：

- `src/commands/dashboard/handlers/workspaces.rs`

新增验证点：

- `recent_tasks_endpoint_supports_workspace_filter`

### 3.3 实际执行结果

本次已通过：

- `npm run test`
- `npm run build`
- `cargo test --features dashboard --lib --tests`
- `cargo test recent_tasks_endpoint_supports_workspace_filter --features dashboard`

注意：

- `cargo test --features dashboard` 的 **doctest** 仍存在一个既有失败：`src/acl/privilege.rs` 中的示例无法直接编译
- 该问题与本次 `P1` 改动无关，本次未越权修复

---

## 4. 当前边界

本次 `P1` 已经让 `Files & Security` 具备“工作台”形态，但仍保留以下后续空间：

- 批量危险动作仍是“骨架”，尚未做独立批量 guarded 执行协议
- 目录节点尚未作为独立“选中对象”进入任务桥接，当前主要复用当前目录 + 当前文件两类上下文
- 诊断中心仍主要在 `Statistics & Diagnostics` 工作台承载，本期只做了任务与 Recipe 的域内收口

---

## 5. 建议的 P2 起点

在 `P1` 完成后，最自然的下一步是继续推进 `Phase 2`：

- `acl`
- `lock`
- `protect`
- `encrypt`
- `decrypt`

但重点不应是再加更多卡片，而是：

- 为高风险治理动作补更清晰的 preview 结果解释
- 视需要补工作台级批量 guarded 协议
- 把治理结果与审计时间线做更强联动
