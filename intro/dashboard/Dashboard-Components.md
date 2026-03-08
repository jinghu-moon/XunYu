# Dashboard 组件导读

本文档按“组件一个一个看”的方式梳理 `dashboard-ui/src/components/**/*.vue` 当前 39 个 Vue 组件，并补上入口层与接口层的上下文，方便你从源码快速建立整体模型。

## 1. 入口与装配

- `dashboard-ui/src/main.ts`：前端入口。负责 `createApp(App)`、挂载 PrimeVue、把 Aura 主题扩展成项目自己的黑白系变量方案。
- `dashboard-ui/src/App.vue`：应用壳层。它维护当前 Tab、命令面板开关、全局错误监听，以及 `Ctrl/Cmd + K` 唤起命令面板的快捷键。
- `dashboard-ui/src/api.ts`：统一请求适配层。所有组件尽量不直接 `fetch`，而是走这里；这里顺手统一了 loading 计数和错误 toast。
- `dashboard-ui/src/types.ts`：前后端契约类型。组件的 props、面板状态和 API 返回结构都围绕这里展开。

## 2. 组件树总览

```text
App
├─ CapsuleTabs
├─ DensityToggle
├─ ThemeToggle
├─ CommandPalette
├─ GlobalFeedback
└─ Active Panel
   ├─ HomePanel
   ├─ BookmarksPanel
   ├─ PortsPanel
   ├─ ProxyPanel
   ├─ ConfigPanel
   ├─ EnvPanel
   │  ├─ EnvVarsTable
   │  ├─ EnvVarHistoryDrawer
   │  ├─ EnvTemplateRunPanel
   │  ├─ EnvSnapshotsPanel
   │  ├─ EnvSchemaPanel
   │  ├─ EnvProfilesPanel
   │  ├─ EnvPathEditor
   │  ├─ EnvImportExportPanel
   │  ├─ EnvGraphPanel
   │  ├─ EnvDoctorPanel
   │  ├─ EnvDiffPanel
   │  ├─ EnvAuditPanel
   │  └─ EnvAnnotationsPanel
   ├─ RedirectPanel
   ├─ AuditPanel
   └─ DiffPanel
      ├─ DiffOptions
      ├─ DiffFileManager
      │  └─ DiffFilePreview
      ├─ DiffConvertPanel
      ├─ DiffStats
      ├─ ConfigDiffTree
      ├─ CodeDiffPanel
      │  └─ DiffViewer
      ├─ LineDiffPanel
      │  └─ DiffViewer
      └─ FileBrowser（当前未接入主链）
```

## 3. 壳层与通用组件

- `CapsuleTabs.vue`：Tab 导航组件。支持动画指示器、键盘导航、等宽 / 拉伸 / 网格布局，是 `App.vue` 顶部导航的核心交互部件。
- `CommandPalette.vue`：全局命令面板。通过 `modelValue` 控制显示，支持关键字检索、上下键选择、Enter 执行、Teleport 到 `body`。
- `ThemeToggle.vue`：主题切换器。管理 `system / light / dark` 三态，持久化到 `localStorage`，并在可用时使用 View Transition API 做切换动画。
- `DensityToggle.vue`：密度开关。只做一件事：切换根节点 class（`density-compact` / `density-spacious`）并写入 `localStorage`。
- `GlobalFeedback.vue`：全局反馈渲染层。把 `ui/feedback` 中的 loading 计数和 toast 队列可视化出来，属于全局状态的最终出口。
- `SkeletonTable.vue`：通用骨架屏。通过 `rows` / `columns` 生成占位表格，复用于多个面板的初次加载阶段。
- `button/Button.vue`：统一按钮基元。封装 `preset`、`size`、`square`、`loading`、`disabled` 等视觉与交互状态，减少业务组件内重复按钮样式。

## 4. 顶层业务面板

- `HomePanel.vue`：总览页。并行拉取书签、端口、代理和审计摘要，把它们压成 KPI 卡片、端口快照、代理健康和最近审计四块内容，更像“Dashboard 首页 / 烟雾测试面板”。
- `BookmarksPanel.vue`：书签管理主面板。负责书签的查询、创建、删除、重命名、标签编辑、搜索、标签过滤、排序、目录分组、批量删/批量加减标签，以及复制路径、打开路径、导出。
- `PortsPanel.vue`：端口观察与进程终止面板。围绕 `fetchPorts` 和 `killPid` 展开，额外提供协议过滤、开发端口过滤、按 PID 分组、自动刷新和导出。
- `ProxyPanel.vue`：代理配置面板。把“保存默认配置”和“立即应用 / 删除 / 测试代理”拆开处理，同时展示当前代理一致性，适合作为多工具代理状态的控制台。
- `ConfigPanel.vue`：轻量全局配置编辑器。当前只暴露 `tree.defaultDepth` 和 `tree.excludeNames`，它不是完整配置中心，而是一个针对常用树形配置的窄入口。
- `EnvPanel.vue`：环境变量系统的总控面板。它不是简单展示组件，而是 Env 全量状态的编排器：持有 scope、status、loading、WebSocket、历史、快照、schema、profile 等状态，并把事件分发给 13 个子组件。
- `RedirectPanel.vue`：重定向规则编辑器。围绕 profile 管理、规则草稿、拖拽排序、dirty 检测、离开保护、dry-run 预演、规则校验而设计，明显属于“规则编辑工作台”。
- `AuditPanel.vue`：审计日志面板。支持搜索、按 action / result 过滤、分页、详情查看和导出，是偏运营 / 排障的只读界面。
- `DiffPanel.vue`：Diff 工作台。维护旧文件 / 新文件路径、diff 选项、转换面板开关、运行状态、结果视图，并在配置文件场景下额外构建“语义树差异”。

## 5. Env 组件族（`EnvPanel` 子树）

- `EnvVarsTable.vue`：变量主表。承担变量列表、搜索、快速填写、设置、删除、scope 切换和“查看历史”入口，是 Env 子系统的第一触点。
- `EnvVarHistoryDrawer.vue`：变量历史抽屉。只在选中某个变量时打开，专注展示该变量的审计历史，职责非常单一。
- `EnvTemplateRunPanel.vue`：模板 / 导出 / 执行面板。把模板展开、live export、运行命令三件事合并在一个操作面板里，并允许 schema 检查、通知和最大输出限制。
- `EnvSnapshotsPanel.vue`：快照面板。负责列快照、创建快照、裁剪历史、恢复快照，是 Env 回滚能力的主入口。
- `EnvSchemaPanel.vue`：Schema 规则与校验面板。支持 required / regex / enum 三类规则的增删、重置和执行验证。
- `EnvProfilesPanel.vue`：Profile 面板。负责把当前环境捕获为 profile，以及 apply / delete / diff profile，偏“版本快照模板”的角色。
- `EnvPathEditor.vue`：PATH 专用编辑器。聚焦 PATH entries 的增删和插入到头部，与一般变量编辑刻意拆开。
- `EnvImportExportPanel.vue`：导入导出面板。支持 `json/env/reg/csv`，并提供 merge / overwrite / dry-run，以及文本粘贴 / 拖拽导入。
- `EnvGraphPanel.vue`：依赖图面板。以某个变量名为根、加上最大深度，触发依赖树计算并渲染结果。
- `EnvDoctorPanel.vue`：诊断面板。只负责两类动作：运行 doctor、执行 fix，并展示报告与修复结果。
- `EnvDiffPanel.vue`：环境差异面板。比较 live 环境和快照 / 时间点，提供筛选和变更统计，是 Env 版的 diff 摘要视图。
- `EnvAuditPanel.vue`：Env 审计列表。专注展示 Env 域下的审计事件，不承担编辑逻辑。
- `EnvAnnotationsPanel.vue`：变量注释面板。给变量附加说明文字，定位是轻量元数据管理。

## 6. Diff 组件族（`DiffPanel` 子树）

- `diff/DiffOptions.vue`：Diff 选项表单。通过 `defineModel` 直接和父级共享 `mode / algorithm / context / whitespace flags / force_text`。
- `diff/DiffFileManager.vue`：文件管理主面板。它是 Diff 子系统里最重的组件，负责目录树、搜索、深搜、虚拟列表、选中文件、上下文菜单、自动刷新、Diff WebSocket 接入，以及和 `oldPath/newPath` 的双向绑定。
- `diff/DiffFilePreview.vue`：文件预览器。按选中文件加载基本信息、分块文本内容和语法校验结果，支持增量加载和跳转到校验失败行。
- `diff/DiffConvertPanel.vue`：格式转换面板。读取原文、预览转换结果、并支持直接写回目标格式，适合配置文件清洗与迁移。
- `diff/DiffViewer.vue`：底层 diff 渲染器。把 hunks 渲染成 unified / split 两种表格视图，是 `CodeDiffPanel` 和 `LineDiffPanel` 的共同基础设施。
- `diff/CodeDiffPanel.vue`：代码 diff 包装器。会先过滤 unchanged hunks，再补一层 symbol summary，更偏代码阅读体验。
- `diff/LineDiffPanel.vue`：行级 diff 包装器。很薄，只是把行级结果转交给 `DiffViewer`。
- `diff/ConfigDiffTree.vue`：配置语义树渲染器。递归展示配置节点的增删改未变状态，用于 `DiffPanel` 的 config semantic 模式。
- `diff/DiffStats.vue`：统计摘要卡。负责展示 added / removed / modified / unchanged 总量。
- `diff/FileBrowser.vue`：独立的目录浏览弹层。能力上是一个简化版路径选择器，但当前代码里没有发现主链引用，更像预留 / 备用组件。

## 7. 组件与后端接口映射

| UI 区域 | 主要组件 | 主要接口 |
| --- | --- | --- |
| 总览 | `HomePanel` | `/api/bookmarks`、`/api/ports`、`/api/proxy/status`、`/api/proxy/config`、`/api/audit` |
| 书签 | `BookmarksPanel` | `/api/bookmarks`、`/api/bookmarks/{name}`、`/api/bookmarks/{name}/rename`、`/api/bookmarks/batch` |
| 端口 | `PortsPanel` | `/api/ports`、`/api/ports/kill-pid/{pid}` |
| 代理 | `ProxyPanel` | `/api/proxy/status`、`/api/proxy/config`、`/api/proxy/set`、`/api/proxy/del`、`/api/proxy/test` |
| 配置 | `ConfigPanel` | `/api/config` |
| Redirect | `RedirectPanel` | `/api/redirect/profiles`、`/api/redirect/profiles/{name}`、`/api/redirect/dry-run` |
| 审计 | `AuditPanel` | `/api/audit` |
| Env | `EnvPanel` + Env 子组件 | `/api/env/*`、`/api/env/ws` |
| Diff | `DiffPanel` + Diff 子组件 | `/api/diff`、`/api/files`、`/api/files/search`、`/api/info`、`/api/content`、`/api/convert`、`/api/validate`、`/ws` |

## 8. 读代码时的建议顺序

1. 先看 `App.vue`，明确顶层导航和全局交互模型。
2. 再看 `api.ts`，知道每个业务区到底调了哪些后端接口。
3. 接着看顶层面板：`HomePanel` → `BookmarksPanel` → `PortsPanel` → `ProxyPanel` → `ConfigPanel` → `RedirectPanel` → `AuditPanel`。
4. 然后看两个复合系统：`EnvPanel`、`DiffPanel`。
5. 最后再展开它们的子组件；这时你已经不会把“编排组件”和“展示组件”混在一起。

## 9. 当前实现上的几个观察

- `App.vue` 采用“单页 + Tab 状态”而不是路由；这很适合本地工具型 Dashboard，切换快、状态简单。
- `EnvPanel` 和 `DiffPanel` 都是明显的“容器组件 / 编排组件”，后续扩展时尽量继续把复杂状态留在父层，子组件保持输入清晰、事件明确。
- `ConfigPanel` 是一个刻意收窄的编辑器，当前并不打算覆盖整个 `GlobalConfig`。
- `FileBrowser.vue` 暂未接入主链；如果后续要加入独立的“路径选择”体验，这是现成的切入点。
