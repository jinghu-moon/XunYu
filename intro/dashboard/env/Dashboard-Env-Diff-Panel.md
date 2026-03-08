# Dashboard EnvDiffPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/EnvDiffPanel.vue`。

`EnvDiffPanel` 是 Env 工作台里的差异比较工作台。它的核心不是编辑变量，而是围绕“当前环境相对某个基线发生了什么变化”来组织视图。

这个组件处理了两类基线：

- 基于某个快照 `snapshot`
- 基于某个时间点 `since`

因此它更像一个实时差异面板，而不是普通的结果列表。

## 1. 组件定位

这个组件负责：

- 选择 diff 基线
- 展示增删改数量摘要
- 按类型过滤差异条目
- 展示普通变量差异和 PATH 片段差异

它不是 diff 计算器，真正的 diff 结果已经由父层通过 `diff` prop 提供。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `diff: EnvDiffResult | null`
- `snapshots: EnvSnapshotMeta[]`
- `snapshotId: string | null`
- `since: string | null`
- `scope: EnvScope`
- `loading?: boolean`

这说明它同时消费两类输入：

- 差异结果本身
- 差异基线状态

### 2.2 Emits

它向外抛出：

- `refresh`
- `snapshot-change`
- `since-change`

也就是说，它自己不重新请求 diff，只负责告诉容器：

- 我现在想以哪个快照为基线
- 我现在想以哪个时间点为基线
- 我需要刷新当前 diff

## 3. 本地状态与派生逻辑

### 3.1 本地状态

它内部维护：

- `filter`
- `selected`
- `selectedSince`

其中：

- `filter` 控制当前看 `all/added/removed/changed`
- `selected` 是本地快照选择框镜像
- `selectedSince` 是本地时间点输入镜像

### 3.2 `watch` 同步父层状态

组件会 watch：

- `props.snapshotId`
- `props.since`

并把变化回填到本地输入状态。

这意味着它不是只在首次渲染时读取 prop，而是和容器层保持同步，避免 UI 选项与真实基线脱节。

### 3.3 `totalChanges`

`totalChanges` 是一个非常典型的摘要派生：

- `added.length + removed.length + changed.length`

它负责把 diff 结果压缩成头部总览数字。

### 3.4 `filteredEntries`

这个派生会根据 `filter` 选择：

- 只看 added
- 只看 removed
- 只看 changed
- 或把三组结果合并并按 `name` 排序

所以过滤并不是额外请求，而是纯前端派生。

## 4. 基线切换逻辑

这个组件最值得注意的地方，是它对两种基线模式做了互斥处理。

### 4.1 `onSnapshotSelect()`

当选择快照基线时：

- 如果当前有选中的快照，就先清空 `since`
- 再 emit `snapshot-change`

### 4.2 `onSinceApply()`

当应用时间点基线时：

- 如果有合法输入，就先清空 `snapshot`
- 再 emit `since-change`
- 如果输入为空，就把 `since` 设回 `null`

也就是说，`snapshot` 和 `since` 这两种比较模式在 UI 上是互斥的，而不是叠加的。

## 5. 模板结构

### 5.1 头部基线工具条

头部包含：

- 快照下拉框
- `since` 文本输入
- `Apply Since`
- `Refresh`

默认快照基线是：

- `baseline: latest snapshot`

这说明“不指定 snapshot”并不等于没有基线，而是落回“最新快照”这个默认语义。

### 5.2 摘要区

摘要文本会展示：

- 当前 `scope`
- `changes` 总数
- 增删改三类明细计数

### 5.3 过滤条

过滤条提供 4 个按钮：

- `All`
- `Added`
- `Removed`
- `Changed`

这使得差异查看可以快速收缩到某一类变更。

### 5.4 差异列表

列表里每条记录会展示：

- `kind`
- `name`
- `old_value`
- `new_value`
- 可选的 `path_diff`

其中 `path_diff` 非常关键，它说明这个组件不只展示普通变量值变化，也能可视化 PATH 这种分段结构的差异。

## 6. 架构意义

`EnvDiffPanel` 的意义在于，它把“基线选择”和“结果过滤”都收进了一张卡片里，但仍然不接管 diff 计算本身。

这带来两个好处：

- 视图层可以保持足够强的交互性
- 容器层仍然完全控制数据来源和刷新时机

从职责划分看，它是一个典型的工作台式组件：**交互相对丰富，但业务执行仍然在父层。**

## 7. 组件特征总结

一句话概括 `EnvDiffPanel.vue`：

- **它是一个围绕快照/时间点基线展开的实时差异工作台。**

最值得关注的点有四个：

- `snapshot` 和 `since` 两种模式互斥
- diff 过滤完全在前端完成
- PATH 差异有独立片段展示
- 组件管视图交互，不管 diff 计算

## 8. 推荐阅读顺序

建议按这个顺序读：

1. 先看 `props + emits`，确认它只控制基线与过滤
2. 再看 `watch`、`totalChanges`、`filteredEntries`
3. 接着看 `onSnapshotSelect()` 和 `onSinceApply()` 的互斥逻辑
4. 最后看模板里的基线选择区、过滤条和差异列表

读完后再回到 `EnvPanel` 看 `onDiffSnapshotChange()`、`onDiffSinceChange()`，会更容易理解这块工作台怎么被容器驱动。
