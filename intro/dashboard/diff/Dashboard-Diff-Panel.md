# Dashboard Diff 面板导读

这篇文档专门拆 `dashboard-ui/src/components/DiffPanel.vue`。

`DiffPanel` 是 Dashboard 里最典型的**容器型工作台**之一。它自身不负责文件浏览、行级渲染或配置树渲染的所有细节，而是把路径输入、选项、文件管理、转换和多种 diff 结果编排成一个完整流程。

## 1. 组件定位

这个组件要解决的问题不是“怎么渲染一份 diff”，而是：

- 如何选择两个文件
- 如何配置 diff 参数
- 何时切到配置语义 diff
- 何时回退到 AST / line diff
- 如何顺手做格式转换

因此它本质上是一个 **Diff 编排容器**。

## 2. 子组件依赖图

从 imports 就能看出它的职责拆分：

- `DiffFileManager.vue`：负责文件选择与路径管理侧栏
- `DiffOptions.vue`：负责 diff 选项表单
- `DiffConvertPanel.vue`：负责格式转换面板
- `DiffStats.vue`：负责统计摘要展示
- `ConfigDiffTree.vue`：负责配置语义树渲染
- `CodeDiffPanel.vue`：负责 AST / 符号 diff 结果渲染
- `LineDiffPanel.vue`：负责行级 diff 结果渲染

也就是说，`DiffPanel` 更像总导演，子组件才是具体演员。

## 3. 数据与状态模型

## 3.1 路径与显示状态

- `oldPath`
- `newPath`
- `viewMode`：`unified` / `split`
- `showOptions`
- `showConvert`
- `convertPath`

这些状态负责“当前在比较什么”和“当前展示哪些附加面板”。

## 3.2 运行结果状态

- `busy`
- `result`
- `errorMsg`

`result` 是普通 diff 主结果。

## 3.3 配置语义 diff 相关状态

- `configSemantic`
- `configSemanticError`

这组状态说明 `DiffPanel` 不只跑后端 diff，还会在前端尝试构建一套配置语义树。

## 3.4 选项状态

`options` 是一个 `reactive` 对象，包含：

- `mode`
- `algorithm`
- `context`
- `ignore_space_change`
- `ignore_all_space`
- `ignore_blank_lines`
- `strip_trailing_cr`
- `force_text`

这组参数最终会完整传给 `fetchDiff(...)`。

## 4. 运行流程：一条 diff 链路如何走

## 4.1 `runDiff()`

`runDiff()` 是整个组件的核心。

它的流程可以概括成：

1. 先 trim `oldPath` / `newPath`
2. 任一路径为空则直接返回
3. 清空：
   - `errorMsg`
   - `result`
   - `configSemantic`
   - `configSemanticError`
4. 如果两个文件都像配置文件，则并行准备 `semanticPromise`
5. 调 `fetchDiff(...)` 获取主 diff 结果
6. 等待 `semanticPromise`，拿配置语义树

这里最关键的设计点是：**普通 diff 和配置语义 diff 是并行准备的，但配置语义 diff 是可选增强，而不是主流程阻塞条件。**

## 4.2 为什么要单独做配置语义 diff

`DiffPanel` 用 `CONFIG_EXTS` 识别配置文件扩展名：

- `toml`
- `yaml`
- `yml`
- `json`
- `json5`

如果 old/new 两个路径都落在这个集合里，就会尝试构建 `configSemantic`。

也就是说，它不是根据后端 diff 结果再决定，而是根据文件类型先决定是否值得做“结构化配置比较”。

## 5. 配置语义树是怎么构建的

这是 `DiffPanel` 里最有工程味的一块。

## 5.1 基础判断函数

- `extensionOf(path)`：取扩展名
- `isConfigPath(path)`：判断是否配置文件
- `shouldBuildConfigSemantic(oldP, newP)`：old/new 是否都适合做配置语义 diff
- `isRecord(value)`：判断普通对象
- `joinConfigPath(parent, key)`：生成节点路径

## 5.2 `buildConfigNode(...)`

这个递归函数会把 old/new JSON 树比较成 `ConfigDiffNode` 树：

- 数组按索引展开成 `[0]`、`[1]`...
- 对象按 key 递归展开
- 叶子节点根据 old/new 是否存在与是否相等，标为：
  - `added`
  - `removed`
  - `modified`
  - `unchanged`

这说明前端在这里不只是“把文本变成 JSON”，而是在构建一棵结构化差异树。

## 5.3 `collectConfigStats(...)`

这个函数只统计叶子节点，汇总成：

- `added`
- `removed`
- `modified`
- `unchanged`

## 5.4 `buildConfigSemanticDiff(...)`

这条链路是：

1. 用 `fetchConvertFile({ preview: true, to_format: 'json' })` 分别把 old/new 文件转换成 JSON 文本
2. `JSON.parse(...)`
3. 调 `buildConfigNode('root', '', oldJson, newJson)`
4. 调 `collectConfigStats(...)`

也就是说，配置语义 diff 的基础不是手写 parser，而是复用现有“转换成 JSON”的能力。

## 6. 其他交互动作

- `updateOldPath(path)` / `updateNewPath(path)`：接受侧栏子组件回传路径
- `swapPaths()`：交换 old/new
- `openConvert(path)`：打开转换面板并设定 `convertPath`

这些动作都很薄，说明 `DiffPanel` 非常专注于容器编排，而不在这些小动作上堆复杂逻辑。

## 7. 视图结构

## 7.1 整体布局

模板分成两栏：

- 左侧 `diff-sidebar`
- 右侧 `diff-main`

左侧只有一个核心子组件：`DiffFileManager`。右侧则是路径输入、选项、转换器和结果展示区。

## 7.2 侧栏：`DiffFileManager`

侧栏通过事件与容器交互：

- `update:old-path`
- `update:new-path`
- `run-diff`
- `open-convert`

这说明文件浏览与路径选择完全被侧栏子组件承接，容器只处理回传结果。

## 7.3 主区顶部：路径输入与动作

主区顶部有：

- Old 路径输入
- New 路径输入
- `Swap`
- `Run Diff`

这是最直接的一层输入控制区。

## 7.4 次级折叠区：Options / Convert

中间一层是两个折叠按钮：

- `Options`
- `Convert`

分别控制：

- `DiffOptions`
- `DiffConvertPanel`

这说明选项和格式转换都被视为“附加工具区”，而不是强制占用主流程空间。

## 7.5 结果区：按 diff 种类分支

结果区分 5 类情况：

### identical

直接显示“Files are identical”，并提示是否是“在空白过滤条件下相同”。

### binary

直接提示二进制文件不同。

### config semantic

如果 `configSemantic` 存在，则优先走：

- `DiffStats`（单位是 nodes）
- `ConfigDiffTree`

这说明配置文件会优先走结构化结果，而不是文本 hunk 视图。

### ast

如果 `result.kind === 'ast'`，则走：

- `DiffStats`（单位是 symbols）
- `CodeDiffPanel`
- `viewMode` 切换 `unified/split`

### line fallback

其余情况走：

- `DiffStats`（单位是 lines）
- `LineDiffPanel`
- `viewMode` 切换 `unified/split`

## 8. 组件特征总结

`DiffPanel.vue` 的核心特征有 4 个：

1. **容器优先**：文件选择、选项、转换、渲染都拆到子组件。
2. **多结果分支**：同一入口可走 identical / binary / config / ast / line 多条路径。
3. **配置增强**：对配置文件额外构建结构化语义树。
4. **附加工具并排存在**：diff 之外的转换能力被顺手纳入同一工作台。

## 9. 推荐阅读顺序

建议这样读：

1. 先看状态定义和 `options`
2. 再看 `runDiff()` 主流程
3. 然后看 `buildConfigSemanticDiff()` 及其递归辅助函数
4. 最后看模板里的结果区分支
5. 如果你要继续读文件工作流子组件，建议按 `Dashboard-Diff-FileManager.md` → `Dashboard-Diff-File-Preview.md` → `Dashboard-Diff-Options.md` → `Dashboard-Diff-Convert-Panel.md` 的顺序读
6. 如果你要继续读结果渲染子组件，建议按 `Dashboard-Diff-Stats.md` → `Dashboard-Diff-Config-Tree.md` → `Dashboard-Diff-Code-Panel.md` → `Dashboard-Diff-Line-Panel.md` → `Dashboard-Diff-Viewer.md` → `Dashboard-Diff-File-Browser.md` 的顺序读

读完之后你会比较清楚：`DiffPanel` 真正的价值不是“渲染某一种 diff”，而是把多种 diff 和文件操作流程装进一个统一容器里。


