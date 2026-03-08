# Dashboard DiffFileManager 导读

这篇文档专门拆 `dashboard-ui/src/components/diff/DiffFileManager.vue`。

`DiffFileManager` 是 `DiffPanel` 左侧边栏的核心组件，但它的职责远不止“选文件”。它同时承担了：

- 目录树浏览
- 已加载树的本地过滤
- 当前目录下的递归深搜
- 虚拟列表渲染
- 选中文件预览
- 右键菜单动作
- Diff WebSocket 驱动的自动刷新

所以它本质上是一个 **文件浏览 / 搜索 / 预览 / 选取一体化工作台侧栏**。

## 1. 输入输出边界

## 1.1 Props

它只接收两个外部输入：

- `oldPath`
- `newPath`

也就是说，它自己不拥有 diff 左右路径的最终真相，而是从容器层接收当前绑定结果。

## 1.2 Emits

它向父层发出 4 类事件：

- `update:oldPath`
- `update:newPath`
- `runDiff`
- `openConvert`

所以它的定位很清楚：

- 自己负责“怎么挑文件”
- 父组件负责“路径最终是什么”和“什么时候真正跑 diff”

## 2. 内部数据模型

这个组件内部定义了几组关键数据结构。

## 2.1 树节点模型

- `TreeNode`
  - `name`
  - `path`
  - `isDir`
  - `size?`
  - `expanded?`
  - `loaded?`
  - `loading?`
  - `children?`
- `TreeRow`
  - `node`
  - `depth`

这说明“目录树”在内部是先存嵌套结构，再在展示层拍平成带深度信息的行。

## 2.2 虚拟列表模型

- `VirtualListItem`
  - `kind: 'tree'`
  - 或 `kind: 'search'`
- `VirtualRenderRow`
  - `virtual`
  - `item`

这点非常关键：`DiffFileManager` 并不是只支持目录树，它在列表层面统一抽象了两种来源：

- 树浏览结果
- 深搜结果

然后交给同一套虚拟列表去渲染。

## 2.3 右键菜单模型

- `ContextMenuState`
  - `visible`
  - `x`
  - `y`
  - `path`

右键菜单是一个非常轻量的绝对定位状态机，而不是单独子组件。

## 3. 状态模型

## 3.1 浏览状态

- `currentPath`
- `pathInput`
- `roots`
- `selectedPath`
- `loading`
- `refreshing`
- `error`

这组状态负责“当前目录树是什么、当前选中了什么、当前目录是否正在刷新”。

## 3.2 搜索状态

- `searchTerm`
- `deepSearch`
- `searchBusy`
- `searchError`
- `searchHits`
- `searchSeq`

这里已经能看出它有两种搜索模式：

- 普通过滤：对已加载树做本地过滤
- 深搜：对当前目录做后端递归搜索

## 3.3 自动刷新与 WebSocket 状态

- `autoRefresh`
- `refreshTimer`
- `wsConnected`
- `wsStatus`
- `previewRefreshKey`
- `wsStopFn`
- `wsReconnectTimer`
- `wsRefreshTimer`
- `wsManualStop`

这说明 `DiffFileManager` 自己就具备很强的“实时浏览器”属性，而不是被动等父容器刷新。

## 3.4 列表与虚拟化状态

- `listViewportRef`
- `rowVirtualizer`
- `virtualRows`
- `virtualTotalSize`

这套状态服务于大列表性能优化。

## 4. 派生结果：双模列表是核心

## 4.1 树浏览模式

- `visibleRows = flattenTree(roots)`
- `activeRows = flattenFilteredTree(roots, term)`

如果不开深搜，组件会对当前已加载树做过滤后显示。

## 4.2 深搜模式

- `useDeepSearch = deepSearch && searchTerm 非空`
- `activeItems = searchHits.map(...)`

一旦进入深搜模式，列表数据源就从树切换成搜索命中集合。

## 4.3 统一渲染层

`activeItems` 最终统一变成：

- 树行项
- 或搜索命中项

再交给虚拟列表。

所以这个组件最厉害的地方之一，是把“树”和“搜索结果”收口成一层统一渲染模型。

## 5. 路径与树构建基础函数

这个组件有一批很关键的路径基础函数：

- `normalizeDirectory(path)`
- `trimTrailingSeparators(path)`
- `resolveSeparator(path)`
- `joinPath(base, name)`
- `normalizePathForCompare(path)`
- `pathExt(path)`
- `isConvertiblePath(path)`

它们共同解决：

- Windows / 非 Windows 路径差异
- 根路径与驱动器路径的边界情况
- 路径比较时的归一化
- 某个文件是否支持 Convert 面板

因此这里不是简单字符串拼接，而是有一层相当明确的路径适配逻辑。

## 6. 目录树工作流

## 6.1 初始打开目录

`loadDirectory(path)` 会：

1. 标准化路径
2. 清空错误并关闭菜单
3. 调 `fetchFiles(normalized)`
4. 把结果转成 `roots`
5. 同步 `currentPath` 与 `pathInput`

这是“切换当前目录”的真正入口。

## 6.2 刷新当前树

`refreshCurrentTree()` 做得比 `loadDirectory()` 更细：

- 先把旧树拍成 `prevMap`
- 拉最新目录项
- 用 `toTreeNodes(..., prevMap)` 尽量保留旧节点的展开状态
- 对已展开目录递归补子节点 `hydrateExpandedChildren()`
- 如果刷新后当前选中文件消失，则清空 `selectedPath`

所以刷新不是粗暴重建，而是尽量保留用户当前浏览上下文。

## 6.3 展开 / 折叠

- `toggleDirectory(node)`：按需懒加载子目录
- `collapseAll()`：递归收起所有已展开目录
- `goUp()`：上移到父目录

这里的设计说明它是一个真正可操作的树浏览器，而不是一次性平铺目录列表。

## 7. 搜索工作流：本地过滤与深搜并存

## 7.1 本地过滤

当 `deepSearch` 关闭时，`searchTerm` 只用于过滤已加载树。

优点是：

- 快
- 不需要额外请求
- 适合在已展开树内缩小范围

## 7.2 深搜

当 `deepSearch` 开启且 `searchTerm` 非空时：

- `scheduleDeepSearch()` 做 250ms debounce
- `runDeepSearch()` 调 `fetchFileSearch({ root, query, limit })`
- `searchSeq` 用于丢弃过期请求结果

这说明它不是“边打字边无限请求”的粗暴实现，而是带有：

- 防抖
- 竞态保护
- 限制条数

的一套更稳妥搜索机制。

## 7.3 清空与滚动复位

- `clearSearch()`
- `clearDeepSearchState()`
- `resetListScroll()`

输入条件变化时，组件还会把虚拟列表滚动位置重置到顶部，保持结果切换体验一致。

## 8. 选择、预览与动作

## 8.1 选择文件

- `selectNode(node)`：仅文件可选，目录不可直接作为目标路径
- `onNodeClick(node)`：目录就展开，文件就选中
- `onSearchHitClick(hit)`：深搜命中若是目录则直接跳目录，若是文件则选中

这说明“点击目录”和“点击文件”是两套明确分开的交互。

## 8.2 预览联动

模板里直接挂了：

- `DiffFilePreview :path="selectedPath" :refresh-key="previewRefreshKey"`

所以 `DiffFileManager` 不只是树浏览器，它还是预览宿主。选中文件后，右侧预览区会同步显示文件信息、内容和校验结果。

## 8.3 选择结果输出

围绕 `selectedPath`，组件提供了：

- `setSelectedAsOld()`
- `setSelectedAsNew()`
- `setSelectedAsOldAndRun()`
- `setSelectedAsNewAndRun()`
- `openSelectedInConvert()`

这些动作最终只是 emit 给父层，自己不直接修改 `DiffPanel` 的最终状态。

## 9. 右键菜单：把高频动作压缩到上下文菜单

右键菜单支持：

- Preview
- Set as Old
- Set as New
- Use as Old + Run Diff
- Use as New + Run Diff
- Open Convert

它的特点是：

- 只对文件开放，目录右键无效
- 会根据 `oldPath/newPath` 是否已就绪控制某些动作是否禁用
- 对不支持转换的文件禁用 `Open Convert`

这使得文件选择工作流变得非常高效。

## 10. WebSocket 与自动刷新

## 10.1 WebSocket 刷新策略

`connectWs()` 使用 `connectDiffWs(...)` 建立连接，并维护：

- `connecting`
- `connected`
- `retrying`
- `closed`

几种状态。

收到事件时：

- `connected`：更新在线状态
- `refresh`：延迟 200ms 刷新树，并刷新当前预览
- `file_changed`：只有变更路径与当前目录 / 当前选中文件有关时才刷新

这说明它不是“任何变化都全刷”，而是做了路径相关性判断 `shouldRefreshForPath(...)`。

## 10.2 自动刷新

除了 WS，它还支持本地 `Auto Refresh (5s)`：

- `startAutoRefresh()`
- `stopAutoRefresh()`
- watch `autoRefresh`

这为无 WS 或不稳定场景提供了兜底刷新机制。

## 11. 虚拟列表

它使用 `@tanstack/vue-virtual` 做大列表虚拟化：

- 树浏览时按 `treeRowHeight`
- 深搜结果按 `searchRowHeight`
- 统一 `overscan`

并在 `activeItems` 变化后调用 `rowVirtualizer.measure()` 重新测量。

因此这个侧栏不是简单 `v-for`，而是明确按大目录场景做过性能设计。

## 12. 视图结构

整体结构可以概括为：

1. 头部：标题、WS 状态、Up、Refresh
2. 路径区：目录路径输入 + Open
3. 工具区：搜索、Deep Search、Clear、Collapse、Auto Refresh
4. 当前选中区：Selected 路径 + 一组动作按钮
5. 文件预览区：`DiffFilePreview`
6. 列表区：虚拟化树 / 深搜结果
7. 右键菜单：Teleport 到 body 的上下文动作菜单

这说明它更像一个小型文件管理器，而不是普通文件选择器。

## 13. 组件特征总结

`DiffFileManager.vue` 的核心特点有 5 个：

1. **树浏览与深搜双模并存**。
2. **选取、预览、转换入口合一**。
3. **通过 emit 把最终 diff 动作留给父容器**。
4. **通过 WS + 自动刷新保持目录实时性**。
5. **通过虚拟列表支撑大目录场景性能**。

## 14. 推荐阅读顺序

建议这样读：

1. 先看状态定义和 `activeItems`
2. 再看 `loadDirectory()`、`refreshCurrentTree()`、`toggleDirectory()`
3. 然后看 `runDeepSearch()` 和相关 watch
4. 最后看 `setSelected*`、右键菜单和模板结构

读完之后你会更容易理解：`DiffFileManager` 不是 `DiffPanel` 的一个小配件，而是 Diff 工作台里最重的输入侧组件之一。
