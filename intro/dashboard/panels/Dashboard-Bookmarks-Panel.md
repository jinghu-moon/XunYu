# Dashboard 书签面板导读

这篇文档专门拆 `dashboard-ui/src/components/BookmarksPanel.vue`。

如果说 `../Dashboard-Components.md` 解决的是“书签面板在组件树里处于什么位置”，那这篇文档解决的是：**它内部到底管理了哪些状态、支持哪些工作流、以及它如何和后端书签接口对齐。**

## 1. 组件定位

`BookmarksPanel.vue` 是 Dashboard 里偏“工作台型”的组件，而不是纯展示组件。

它同时承担了：

- 书签列表加载
- 搜索、标签过滤、排序
- 新增书签
- 单项删除
- 行内重命名
- 行内标签编辑
- 批量加标签 / 批量删标签 / 批量删除
- 导出 CSV / JSON
- 按目录分组查看
- 复制路径 / “Open”快捷动作

所以它的定位更接近一个 **书签管理控制台**，而不是简单表格。

## 2. 数据契约与接口边界

## 2.1 数据模型：`Bookmark`

`dashboard-ui/src/types.ts` 里，书签项结构非常直接：

- `name`
- `path`
- `tags: string[]`
- `visits`
- `last_visited`

这说明前端把书签看成“名称 + 路径 + 标签 + 使用统计”的组合，而不是只有路径。

## 2.2 API 边界

`BookmarksPanel.vue` 对接的接口都来自 `dashboard-ui/src/api.ts`：

- `fetchBookmarks()`：加载全部书签
- `upsertBookmark(name, path, tags)`：新增或覆盖单条书签
- `deleteBookmark(name)`：删除单条书签
- `renameBookmark(name, newName)`：重命名单条书签
- `bookmarksBatchDelete(names)`：批量删除
- `bookmarksBatchAddTags(names, tags)`：批量加标签
- `bookmarksBatchRemoveTags(names, tags)`：批量删标签

这个组件的特点是：**书签所有主要操作都已经被 API 层显式建模出来了**，前端不用手工拼各种不透明请求。

## 3. 状态模型

这个组件内部状态不少，但分工是清楚的。

### 3.1 数据与筛选状态

基础数据状态：

- `bookmarks`：当前书签列表
- `search`：全文搜索词
- `tagFilter`：标签筛选输入
- `viewMode`：`list` / `group`
- `sortKey`：`name` / `path` / `tags` / `visits`
- `sortDir`：`asc` / `desc`
- `columns`：列显示开关（`path/tags/visits`）

这部分状态共同决定了“当前用户看到的那一版书签表”。

### 3.2 编辑状态

新增和行内编辑被分成两类状态：

- 新增表单：`showForm`、`form`
- 行内编辑：`editingNameKey`、`editingNameValue`、`editingTagsKey`、`editingTagsValue`

这说明组件没有把“新增”和“编辑”强行统一成一个复杂弹窗，而是：

- 新增用顶部展开表单
- 编辑用行内就地编辑

这符合书签场景的高频、轻量特性。

### 3.3 批量操作状态

- `selected`：按书签名索引的选中映射
- `batchTags`：批量标签输入
- `batchDeleteBusy`

选中状态不是存在数组里，而是用 `Record<string, boolean>` 做键控，这让勾选和局部更新都比较直接。

### 3.4 忙碌与确认状态

- `busy`：整体加载忙碌
- `editBusy`：行内编辑忙碌
- `deleteBusyName`：单条删除忙碌项
- `confirmKey` + `confirmRemaining` + `confirmTimer`

其中最值得注意的是删除确认机制：

- 单条删除和批量删除共用一套“3 秒确认窗”逻辑
- 第一次点击只会进入 armed 状态
- 计时结束自动取消

这让危险动作有了最小必要的保护，但又不需要引入额外弹窗。

## 4. 计算属性：这个面板的大脑

`BookmarksPanel.vue` 的真正复杂度主要在一组 `computed` 上，而不是在副作用逻辑上。

## 4.1 标签与搜索

- `tagFilters`：把 `tagFilter` 解析成去重后的标签数组
- `allTags`：从所有书签中聚合可选标签，用于 `datalist`
- `filtered`：同时叠加全文搜索和标签过滤

这里的标签过滤规则不是“任一命中”，而是：**输入的多个标签必须全部命中**。

也就是说：

- `search` 是模糊匹配名称 / 路径 / 标签
- `tagFilter` 是更严格的交集过滤

## 4.2 排序与选择

- `sorted`：基于 `filtered` 再排序
- `selectedNames`：把映射转回当前选中名称数组
- `hasSelection`：控制批量条是否显示
- `allVisibleSelected`：只判断“当前可见结果”是否全选
- `selectAllLabel`：动态切换为 `Select All` / `Clear`

这里有个很关键的交互语义：**全选只针对当前过滤和排序后的可见书签**，而不是针对原始全集。

## 4.3 视图派生

- `listColspan`：根据列开关动态计算表格跨列数
- `grouped`：把 `sorted` 结果按目录分组

`grouped` 的分组依据是 `dirname(b.path)`，并且先把反斜杠规范成正斜杠后再取目录。

这意味着“分组模式”不是另外一套数据源，而是对同一份排序结果的二次组织。

## 5. 核心工作流

## 5.1 加载

`load()` 做三件事：

1. `fetchBookmarks()` 拉全量数据
2. 重置 `selected`
3. 重置行内编辑状态

这表示组件每次刷新后都回到一个相对干净的交互状态，避免旧选择和旧编辑态污染新数据。

## 5.2 新增书签

`onAdd()` 的流程很薄：

1. 校验 `name` 和 `path` 非空
2. 把 `form.tags` 按逗号拆分成数组
3. 调 `upsertBookmark(...)`
4. 清空表单、关闭表单、重新加载

这里用的是 `upsertBookmark`，不是单独的 `createBookmark`。这说明后端把“新增 / 覆盖”统一成了幂等写入语义。

## 5.3 单项删除与批量删除

单项删除 `onDelete(name)` 与批量删除 `onBatchDelete()` 都遵循同一套路：

- 先检查是否已经 armed
- 未 armed 时只启动 3 秒确认
- 已 armed 时才真正发删除请求
- 成功后重新 `load()`

这种模式比确认弹窗更轻，也比直接删更安全。

## 5.4 行内编辑

组件支持两类行内编辑：

- 名称编辑：`startEditName()` → `saveEditName()`
- 标签编辑：`startEditTags()` → `saveEditTags()`

交互上都支持：

- `Enter` 提交
- `Esc` 取消
- `blur` 自动保存

实现上也很干净：一次只允许一个编辑焦点，编辑名称和编辑标签会相互清空对方状态。

## 5.5 批量标签治理

批量操作区只在 `hasSelection` 时显示，支持：

- `onBatchAddTags()`
- `onBatchRemoveTags()`

这意味着批量条不是永远占空间，而是一个按需出现的操作带。

## 5.6 路径动作

这个面板有两个和路径相关的动作：

- `onCopyPath(path)`：复制路径
- `onOpenPath(path)`：本质上仍然是复制路径，只是 toast 文案变成“Paste into Explorer to open.”

这是一个很值得注意的实现细节：**当前所谓的 “Open” 并不是真的调用系统打开路径，而是复制路径并提示你去资源管理器粘贴。**

也就是说，这里更像“安全快捷辅助”，而不是系统集成型动作。

## 5.7 导出

`exportBookmarks(format)` 支持：

- JSON：`downloadJson('bookmarks', items)`
- CSV：`downloadCsv('bookmarks', ...)`

导出的不是原始 `bookmarks`，而是 **当前 `sorted` 结果**。这意味着：

- 当前搜索结果会影响导出内容
- 当前标签过滤会影响导出内容
- 当前排序不会改数据本身，但会改导出顺序

这是很典型的“导出当前视图”而不是“导出全量原始数据”。

## 6. 视图结构

## 6.1 顶部工具条

第一层工具条包括：

- 搜索框
- 展开 / 收起新增表单按钮
- 视图模式切换（List / Group）

第二层工具条包括：

- 列开关
- 标签过滤
- 当前可见项全选 / 清除
- 导出按钮

从布局上就能看出它在分两类能力：

- 第一层偏“浏览入口”
- 第二层偏“结果集操作”

## 6.2 批量操作条

当存在选中项时，会出现批量操作条：

- 显示已选数量
- 输入批量标签
- 执行批量加 / 删标签
- 执行批量删除

这块是整个组件“管理台”气质最强的区域。

## 6.3 列表模式

列表模式下是标准表格：

- 首列勾选
- 名称列支持行内编辑
- 路径列支持复制 / Open
- 标签列支持行内编辑，并用 `tagCategoryClass()` 套标签视觉分类
- 访问数列展示使用统计
- 最后是动作列

加载初期如果还没拿到数据，会显示 `SkeletonTable`。

## 6.4 分组模式

分组模式不是换成卡片，而是：

- 按目录把书签分组
- 每个目录用 `<details>` 展开
- 每组内部仍然保留和列表模式一致的表格交互

这说明分组模式本质上是“组织方式变化”，不是“交互模型变化”。

## 7. 依赖的小工具

这个面板虽然是业务组件，但也复用了几层公共设施：

- `Button`：统一按钮视觉和 loading/disabled 状态
- `SkeletonTable`：统一加载占位
- `pushToast()`：统一成功 / 警告反馈
- `tagCategoryClass()`：统一标签分类样式
- `downloadCsv()` / `downloadJson()`：统一导出实现

所以它不是自带一堆零散实现，而是建立在 Dashboard 公共层之上。

## 8. 组件特征总结

用一句话概括：`BookmarksPanel.vue` 是一个 **围绕当前结果集进行高频治理** 的组件。

它最重要的设计特征有 4 个：

1. **结果集驱动**：搜索、过滤、排序、导出、全选都围绕当前可见列表展开。
2. **就地编辑**：名称和标签都尽量在表格内完成，而不是开大弹窗。
3. **轻确认机制**：删除用 3 秒确认窗，不打断节奏。
4. **双视图同语义**：List 和 Group 共享同一套行级动作，只是组织方式不同。

## 9. 推荐阅读顺序

如果你要继续往源码里钻，建议这样看：

1. 先看状态定义和 `computed`，建立结果集模型
2. 再看 `load`、`onAdd`、`onDelete`、`onBatchDelete` 四条主流程
3. 然后看 `saveEditName` / `saveEditTags`，理解行内编辑
4. 最后看模板的两种模式：`list` 和 `group`

这样你会更容易理解：这个组件的核心不是“表格长什么样”，而是“当前结果集如何被持续治理”。
