# XunYu P3-P5 完成说明

## 1. 本期目标

`P3` 到 `P5` 的目标不是把更多 CLI 子命令平铺成新页面，而是把既有能力稳定收口到 8 个工作台体系中，并补齐跨工作台协作链路。

本轮完成重点分为三类：

- 为“路径与上下文”“集成与自动化”“媒体与转换”补齐任务卡、Recent Tasks 与 Recipe 收口
- 为多个工作台补齐“本地 Recent Tasks -> 全局统计与诊断”的跳转桥
- 让 `Statistics & Diagnostics` 成为真正的跨域消费层，而不是只承载单一审计列表

---

## 2. 已完成内容

### 2.1 8 工作台路由拓扑稳定

前端 `dashboard-ui/src/App.vue` 已稳定采用 8 个工作台作为唯一一级导航：

- `overview`
- `paths-context`
- `network-proxy`
- `environment-config`
- `files-security`
- `integration-automation`
- `media-conversion`
- `statistics-diagnostics`

对应的工作台定义、任务分组与展示文案统一集中在：

- `dashboard-ui/src/workspace-tools.ts`

这带来的收益是：

- 顶层导航不再膨胀成命令墙
- 新能力优先进入“工作台 + 任务卡 + Recipe + 最近任务”框架
- `dry-run / guarded / preview / receipt` 协议继续复用同一套任务模型

### 2.2 P3：路径与上下文工作台补齐

`dashboard-ui/src/components/workspaces/PathsContextWorkspace.vue` 已补齐下列工作台级能力：

- `BookmarksPanel` 继续承载路径资产浏览
- `TaskToolbox` 承载 `ctx / ws / recent / stats / dedup / check / gc / keys / all / fuzzy`
- `RecentTasksPanel` 作为本地最近任务消费层
- `RecipePanel` 作为本域顺序工作流入口

同时，路径工作台内的联动语义已经稳定：

- `recent-tasks` 事件优先在本工作台内聚焦
- `audit / diagnostics-center` 事件统一上抛到 `App.vue`
- `App.vue` 负责切换到 `statistics-diagnostics`

### 2.3 P4：集成与自动化工作台落地

`dashboard-ui/src/components/workspaces/IntegrationAutomationWorkspace.vue` 已收口以下能力：

- shell 初始化：`init`
- 补全脚本：`completion` / `__complete`
- alias 治理：`alias`
- 批量重命名：`brn`
- 本地最近任务与 Recipe 闭环

这让“新 shell 初始化 / 自动化脚本支持 / 批量命名治理”不再停留在 CLI-only。

### 2.4 P5：媒体与转换工作台落地

`dashboard-ui/src/components/workspaces/MediaConversionWorkspace.vue` 已承载：

- `img` 图像转换 / 压缩
- `video probe`
- `video compress`
- `video remux`
- 本地 `RecentTasksPanel`
- 本域 `RecipePanel`

媒体工作台同样复用了与其他工作台一致的跳转约定：

- 本地回看最近任务
- 审计 / 诊断事件跳转到统计与诊断工作台

### 2.5 全局统计与诊断跳转桥

这轮最关键的收口，是把“任务卡 / Recipe / 最近任务 -> 统计与诊断”的跨工作台桥补齐。

核心文件：

- `dashboard-ui/src/App.vue`
- `dashboard-ui/src/components/workspaces/StatisticsDiagnosticsWorkspace.vue`
- `dashboard-ui/src/types.ts`

当前交互语义：

- 任意工作台只要 emit `link-panel`
- `App.vue` 统一接收 `StatisticsWorkspaceLinkPayload`
- 如果目标是跨工作台诊断消费，`App.vue` 自动切到 `statistics-diagnostics`
- `StatisticsDiagnosticsWorkspace.vue` 通过 `externalLink` 立即消费 payload，并聚焦到：
  - `RecentTasksPanel`
  - `AuditPanel`
  - `DiagnosticsCenterPanel`

这让：

- 工作台之间不需要互相直接依赖
- 统计与诊断真正成为全局消费层
- “本地闭环优先、跨域诊断统一收口”的策略得以落地

### 2.6 内置 Recipe 覆盖 P3-P5 工作域

后端 `src/commands/dashboard/handlers/recipes.rs` 已补齐以下内置 Recipe：

- `paths-context-health`
- `integration-shell-bootstrap`
- `media-video-probe-compress`

新增分类：

- `paths-context`
- `integration-automation`
- `media-conversion`

这保证对应工作台在 UI 中不再只有任务卡，也具备可直接复用的顺序工作流模板。

### 2.7 统计与诊断工作台成为跨域消费层

`dashboard-ui/src/components/workspaces/StatisticsDiagnosticsWorkspace.vue` 当前承载：

- `DiagnosticsCenterPanel`
- `RecentTasksPanel`
- `RecipePanel`
- `AuditPanel`
- `TaskToolbox` 中的统计类任务（如 `cstat`）

并且已经支持：

- 本地发起的 `recent-tasks / audit / diagnostics-center` focus
- `App.vue` 外部注入的 `externalLink` focus
- 首次切换进入工作台时立即消费外部跳转请求

这意味着统计与诊断工作台已经是：

- 失败任务回看中心
- 高风险治理审计入口
- 最近任务安全重放入口
- Recipe 执行复盘入口
- 代码 / 目录统计入口

---

## 3. 关键验证结果

### 3.1 前端测试

本轮新增或补齐的覆盖包括：

- `dashboard-ui/src/App.test.ts`
- `dashboard-ui/src/components/workspaces/PathsContextWorkspace.test.ts`
- `dashboard-ui/src/components/workspaces/IntegrationAutomationWorkspace.test.ts`
- `dashboard-ui/src/components/workspaces/MediaConversionWorkspace.test.ts`
- `dashboard-ui/src/components/workspaces/NetworkProxyWorkspace.test.ts`
- `dashboard-ui/src/components/workspaces/StatisticsDiagnosticsWorkspace.test.ts`
- `dashboard-ui/src/components/workspaces/FilesSecurityWorkspace.test.ts`

重点验证：

- `App.vue` 能把任意工作台的 `link-panel` 正确转发到统计与诊断工作台
- `StatisticsDiagnosticsWorkspace` 能消费 `externalLink` 并正确聚焦目标面板
- `paths-context / integration-automation / media-conversion` 的 `recent-tasks` 事件会留在本工作台内闭环
- `audit / diagnostics-center` 事件会统一上抛到全局桥

### 3.2 后端测试

后端新增验证点：

- `src/commands/dashboard/handlers/recipes.rs`
- `builtin_recipe_catalog_covers_phase3_to_phase5_workspaces`

验证目标：

- P3-P5 对应工作台都能在内置 Recipe 目录中找到归属
- 不会出现任务卡存在，但 Recipe 面板长期空白的断层

### 3.3 实际通过的命令

本轮已通过：

- `npm run test`
- `npm run build`
- `cargo test --features dashboard`

---

## 4. 路线图完成口径

按照 `docs/project/Dashboard-Expansion-Roadmap.md` 的分期口径，当前可以认定：

- `Phase 0`：工作台骨架、统一确认弹窗、任务回执、最近任务、Recipe、诊断中心已完成
- `Phase 1`：文件与安全工作台已具备文件管理、上下文桥接、任务区、最近任务与 Recipe 收口
- `Phase 2`：文件治理观察面、治理摘要、批量治理和统计与诊断联动已完成第一阶段闭环
- `Phase 3`：路径与上下文工作台已完成任务化收口
- `Phase 4`：集成与自动化工作台已完成任务化收口
- `Phase 5`：媒体与转换工作台已完成落地，统计与诊断工作台已完成跨域消费层收口

也就是说，**路线图定义的 Phase 0 - Phase 5 已全部有代码落点，并且当前验证通过。**

---

## 5. 当前建议

接下来不建议再回到“按命令加一级页”的方向，而应继续沿着以下原则迭代：

- 工作台优先，不把 CLI 帮助翻译成按钮墙
- 本地闭环优先，跨域诊断统一归并到 `Statistics & Diagnostics`
- 危险动作继续坚持 `Preview -> Confirm -> Receipt + Audit`
- Recipe 继续作为高频流程固化入口，而不是补第二套编排系统

---

## 6. P3-P5 最终收尾记录

在主线功能完成后，本轮又按 `docs/project/P3-P5-Remaining-Tasks.md` 做了最终收尾，目标是把 P3-P5 从“功能已可用”推进到“文案、体验、命名、测试都收干净”的状态。

### 6.1 Priority 1：内置 Recipe 文案清理完成

`src/commands/dashboard/handlers/recipes.rs` 已统一清理以下内置 Recipe 的中文文案与步骤摘要：

- `paths-context-health`
- `integration-shell-bootstrap`
- `media-video-probe-compress`
- `proxy-diagnostics`

同时补充了三个场景化 Recipe：

- `media-video-remux-validate`
- `media-image-batch-convert`
- `statistics-cstat-review`

为避免文案回退，后端新增了两个直接断言内置目录质量的测试：

- `builtin_recipe_catalog_uses_readable_chinese_labels`
- `builtin_recipe_catalog_includes_finalization_recipes`

### 6.2 Priority 2：`ws` 命名口径统一完成

本轮确认并固化了如下规则：

- CLI 子命令继续使用真实命令名 `ws`
- UI 与文档对外统一说明为“工作区批量打开（`ws`）”
- 不再把过时静态补全项写成 `workspace`

本次统一覆盖了：

- shell completion 静态列表
- `xun init` fallback completion 列表
- `Paths & Context` 工作台文案
- `workspace-tools.ts` 内任务标题与描述

### 6.3 Priority 3：Shell Integration 闭环完成

前端新增 `dashboard-ui/src/components/ShellIntegrationGuidePanel.vue`，不再只输出脚本，而是补齐了安装闭环：

- shell 选择：`powershell / bash / zsh`
- profile 路径提示
- profile 写入片段展示
- `init` / `completion` 命令一键复制
- `__complete` 验证命令一键复制
- 一键把推荐参数填入任务卡

`dashboard-ui/src/components/workspaces/IntegrationAutomationWorkspace.vue` 已接入该向导，并通过 `taskPresets + presetVersion` 把预设值送入现有 `TaskToolbox` / `TaskToolCard`，形成“指引 -> 任务执行 -> 最近任务回看”的完整链路。

### 6.4 Priority 4：alias 全链路可视化完成

`dashboard-ui/src/workspace-tools.ts` 现已覆盖剩余 alias 操作：

- `alias:setup`
- `alias:add`
- `alias:rm`
- `alias:export`
- `alias:import`
- `alias:app-add`
- `alias:app-rm`
- `alias:app-ls`
- `alias:app-scan`
- `alias:app-which`
- `alias:app-sync`

其中删除类动作已继续遵守 Triple-Guard：

- `alias:rm`：先 `alias which <name>`，再执行 `alias rm <name>`
- `alias:app-rm`：先 `alias app which <name>`，再执行 `alias app rm <name>`

这保证了别名治理能力没有退回到 CLI-only，也没有绕开高风险确认链。

### 6.5 Priority 5：`img` 高级参数可视化完成

媒体工作台已把 CLI 侧高频高级参数暴露进 UI：

- `svg_method`
- `svg_diffvg_iters`
- `svg_diffvg_strokes`
- `jpeg_backend`
- `png_lossy`
- `png_dither_level`
- `webp_lossy`
- `threads`
- `avif_threads`

这让 `img` 任务不再只停留在“输入 / 输出 / 格式 / 质量”的基础层，而能覆盖更接近真实生产使用场景的编码控制。

### 6.6 前端文案污染一并清理

收尾过程中顺带清理了前端少量残留乱码与重复字段，主要涉及：

- `dashboard-ui/src/components/RecentTasksPanel.vue`
- `dashboard-ui/src/components/TaskToolCard.vue`
- `dashboard-ui/src/components/TaskToolCard.test.ts`

目标是确保“测试通过”和“实际界面可读”同时成立，不留下已知脏点。

### 6.7 收尾验证结果

本轮收尾完成后再次通过：

- `cargo test --features dashboard`
- `npm run test`
- `npm run build`

至此可以认为：

- `P3` 到 `P5` 的主线能力已完成
- `P3-P5-Remaining-Tasks.md` 中列出的 5 个优先级已全部落地
- Dashboard 在这些工作域上已进入“可持续迭代，而不是继续救火收尾”的状态
