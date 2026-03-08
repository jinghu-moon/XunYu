# Dashboard EnvPanel 容器导读

这篇文档专门拆 `dashboard-ui/src/components/EnvPanel.vue`。

和 `BookmarksPanel`、`ProxyPanel` 这种单体工作台不同，`EnvPanel` 本质上是一个**超大容器组件**。它自己并不直接渲染所有 env 功能细节，而是负责：

- 维护 Env 域的共享状态
- 编排十多个 Env 子组件
- 统一调用 `api.ts` 中的 env 能力
- 通过 WebSocket 把整套 Env 工作台保持在实时刷新状态

因此理解 `EnvPanel` 的关键，不是盯着某一个子功能，而是看它怎么把整个 Env 子系统装配起来。

## 1. 容器定位

从 imports 就能看出 `EnvPanel` 的容器属性很强。它同时引入了：

- `EnvVarsTable`
- `EnvPathEditor`
- `EnvSnapshotsPanel`
- `EnvDoctorPanel`
- `EnvDiffPanel`
- `EnvGraphPanel`
- `EnvProfilesPanel`
- `EnvSchemaPanel`
- `EnvAnnotationsPanel`
- `EnvTemplateRunPanel`
- `EnvImportExportPanel`
- `EnvAuditPanel`
- `EnvVarHistoryDrawer`

这意味着 `EnvPanel` 不是单一功能面板，而是 Env 领域的总壳层。

## 2. API 边界：几乎覆盖整个 Env 域

`EnvPanel.vue` 直接消费了大量 env API，基本覆盖了前端侧的全部 Env 能力：

- 变量：`fetchEnvVars`、`setEnvVar`、`deleteEnvVar`
- PATH：`addEnvPath`、`removeEnvPath`
- 快照：`fetchEnvSnapshots`、`createEnvSnapshot`、`pruneEnvSnapshots`、`restoreEnvSnapshot`
- doctor：`runEnvDoctor`、`fixEnvDoctor`
- diff：`fetchEnvDiff`
- graph：`fetchEnvGraph`
- audit：`fetchEnvAudit`、`fetchEnvVarHistory`
- profiles：`fetchEnvProfiles`、`captureEnvProfile`、`applyEnvProfile`、`deleteEnvProfile`、`fetchEnvProfileDiff`
- schema：`fetchEnvSchema`、`addEnvSchemaRequired`、`addEnvSchemaRegex`、`addEnvSchemaEnum`、`removeEnvSchemaRule`、`resetEnvSchema`、`runEnvValidate`
- annotation：`fetchEnvAnnotations`、`setEnvAnnotation`、`deleteEnvAnnotation`
- template/run：`expandEnvTemplate`、`exportEnvLive`、`runEnvCommand`
- import/export：`exportEnv`、`exportEnvBundle`、`importEnvContent`
- 实时：`connectEnvWs`

这组 import 本身就是一个强信号：**EnvPanel 基本就是 Dashboard 中 Env 子系统的前端门面。**

## 3. 状态模型

`EnvPanel` 的状态很多，但可以清楚分层。

## 3.1 共享上下文状态

- `scope`
- `loading`
- `wsConnected`

这三项是整个 Env 工作台共享的全局上下文：

- 当前作用域是什么
- 当前是否有异步操作进行中
- 当前 WebSocket 是否在线

## 3.2 基础数据状态

- `vars`
- `snapshots`
- `statusSummary`
- `auditEntries`
- `profiles`
- `schema`
- `annotations`

这些是各子面板会共同依赖或独立消费的主数据。

## 3.3 派生结果状态

- `doctorReport`
- `doctorFixResult`
- `diff`
- `depTree`
- `validation`
- `profileDiff`
- `templateResult`
- `runResult`

这些状态不是基础源数据，而是“执行某项动作后得到的结果”。

## 3.4 辅助状态

- `diffSnapshotId`
- `diffSince`
- `historyVar`
- `historyEntries`
- `historyLoading`

这组状态用于 diff 的对比来源切换，以及变量历史抽屉的打开与加载。

## 4. 共享派生：容器如何把基础数据转成可消费形态

## 4.1 `statusKpis`

`statusSummary` 会被压缩成一组 KPI 项，作为顶部状态条展示。

这意味着各子组件并不自己重复算总览指标，而由容器统一提供“总况视角”。

## 4.2 `pathEntries`

`pathEntries` 是从 `vars` 中抽出来的 PATH 片段，交给 `EnvPathEditor`。

这说明 PATH 编辑器不是自己再重新请求一份 PATH，而是复用变量主数据的一部分。

## 5. 刷新策略：按子域拆分，但统一收口

`EnvPanel` 没有把所有刷新堆在一个 `load()`，而是拆成一组 `refresh*`：

- `refreshVars`
- `refreshStatus`
- `refreshSnapshots`
- `refreshDiff`
- `refreshAudit`
- `refreshProfiles`
- `refreshSchema`
- `refreshAnnotations`
- `refreshAll`

这种拆法很重要，因为 Env 功能面很多：

- 某个子组件可以只刷新自己需要的数据
- 容器也能在必要时做一次全量刷新

## 5.1 `withLoading(...)`

所有会触发异步动作的 handler 都尽量套进 `withLoading(...)`。

这让容器层对 loading 的管理比较统一，避免每个子面板各自处理一套 loading 细节。

## 6. 事件处理：容器作为统一动作入口

`EnvPanel` 把绝大多数子组件事件都接成 `on*` handler，然后在里面调 API，再刷新相关状态。

## 6.1 变量与 PATH

- `onScopeChange`
- `onSetVar`
- `onDeleteVar`
- `onPathAdd`
- `onPathRemove`

这些是最基础的 env 读写动作。

## 6.2 快照与 doctor

- `onCreateSnapshot`
- `onPruneSnapshots`
- `onRestoreSnapshot`
- `onRunDoctor`
- `onFixDoctor`

其中 `onFixDoctor()` 在 fix 之后会再次 `runEnvDoctor()`，说明容器更关心“修完之后的最新状态”，而不是只显示修复结果。

## 6.3 导入导出与运行态

- `onExport`
- `onExportBundle`
- `onImport`
- `onTemplateExpand`
- `onExportLive`
- `onRunCommand`

这条线把 Env 从“配置台”推进到了“运行时上下文工作台”。

## 6.4 profiles / schema / annotations

- `onCaptureProfile`
- `onApplyProfile`
- `onDeleteProfile`
- `onDiffProfile`
- `onSchemaAddRequired`
- `onSchemaAddRegex`
- `onSchemaAddEnum`
- `onSchemaRemove`
- `onSchemaReset`
- `onRunValidate`
- `onSetAnnotation`
- `onDeleteAnnotation`

这些动作基本对应了 Env 子系统治理线的全部主能力。

## 6.5 历史与图

- `onShowHistory`
- `onCloseHistory`
- `onDiffSnapshotChange`
- `onDiffSinceChange`
- `onRunGraph`

这里体现出容器层一个很典型的职责：**统一协调多个互相关联的辅助状态**。

例如：

- 选择 snapshot 时会清空 `since`
- 选择 `since` 时会清空 snapshot id

这类互斥关系放在容器层处理最合适。

## 7. WebSocket：EnvPanel 的实时刷新中枢

`onMounted(...)` 时，`EnvPanel` 会调用 `connectEnvWs(...)`：

- 收到 `connected` 事件时，把 `wsConnected` 设为 `true`
- 收到其他事件时，直接 `refreshAll()`
- 断开时把 `wsConnected` 设为 `false`

这说明容器层把 WebSocket 当成“全域刷新信号”，而不是只让某个局部子组件单独感知。

因此 `EnvPanel` 的实时性不是零散的，而是全域统一协调的。

## 8. 模板结构：一张完整的 Env 工作台地图

模板层面，`EnvPanel` 的结构非常有层次。

## 8.1 顶部头部与状态条

最上方有：

- 标题 `Env Manager`
- 当前 `scope`
- WebSocket 在线状态
- `Reload`
- `statusSummary` 派生出的 KPI 条和备注

这块相当于整个 Env 工作台的总状态头。

## 8.2 主变量区

第一块主组件是：

- `EnvVarsTable`

它是整个 Env 工作台的主列表入口，负责变量展示和最基础的写操作入口。

## 8.3 网格区 1：PATH + Snapshots

第一组双栏：

- `EnvPathEditor`
- `EnvSnapshotsPanel`

一个偏当前 PATH 操作，一个偏时间点状态管理。

## 8.4 网格区 2：Doctor + Diff

第二组双栏：

- `EnvDoctorPanel`
- `EnvDiffPanel`

一个偏健康诊断，一个偏状态差异比较。

## 8.5 Graph 单独区

- `EnvGraphPanel`

依赖图是独立一块，说明它被视为比普通卡片更重的一类分析能力。

## 8.6 网格区 3：Profiles + Schema

第三组双栏：

- `EnvProfilesPanel`
- `EnvSchemaPanel`

这两块对应 Env 治理线里最“结构化”的两项能力。

中间如果存在 `profileDiff`，还会插入一条提示文本，显示 profile diff 的增删改摘要。

## 8.7 网格区 4：Annotations + Template/Run

第四组双栏：

- `EnvAnnotationsPanel`
- `EnvTemplateRunPanel`

一个偏元数据说明，一个偏运行态与模板能力。

## 8.8 收尾区：导入导出、审计、历史抽屉

最后三块是：

- `EnvImportExportPanel`
- `EnvAuditPanel`
- `EnvVarHistoryDrawer`

其中 `EnvVarHistoryDrawer` 是典型的“由容器状态驱动的浮层组件”。

## 9. 组件特征总结

`EnvPanel.vue` 的核心特征有 5 个：

1. **超大容器**：自己不深做单项 UI，而是编排整套 Env 子组件。
2. **全域共享状态**：`scope`、`loading`、`statusSummary`、`wsConnected` 等由容器统一维护。
3. **动作统一入口**：子组件只发事件，真正调用 API 的是容器层。
4. **分域刷新 + 全量刷新并存**：既能局部刷新，也能在 WS 事件下全量刷新。
5. **实时驱动**：WebSocket 被用作整个 Env 工作台的刷新中枢。

## 10. 推荐阅读顺序

建议这样读这份源码：

1. 先看 imports，建立“这是个总容器”的直觉
2. 再看状态定义，按基础数据 / 派生结果 / 辅助状态分层理解
3. 然后看 `refresh*` 和 `on*` handlers
4. 最后看模板里各子组件的编排顺序
5. 如果你要继续细读编辑与分析链，建议按 `Dashboard-Env-Vars-Table.md` → `Dashboard-Env-Path-Editor.md` → `Dashboard-Env-Snapshots-Panel.md` → `Dashboard-Env-Doctor-Panel.md` → `Dashboard-Env-Diff-Panel.md` → `Dashboard-Env-Graph-Panel.md` 的顺序读
6. 如果你要继续细读治理与收尾链，建议按 `Dashboard-Env-Profiles-Panel.md` → `Dashboard-Env-Schema-Panel.md` → `Dashboard-Env-Annotations-Panel.md` → `Dashboard-Env-Template-Run-Panel.md` → `Dashboard-Env-Import-Export-Panel.md` → `Dashboard-Env-Audit-Panel.md` → `Dashboard-Env-Var-History-Drawer.md` 的顺序读

读完后你会比较清楚：`EnvPanel` 不是某个功能面板，而是 Dashboard 中 Env 子系统的前端装配层。


