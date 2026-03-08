# Dashboard Diff CodeDiffPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/diff/CodeDiffPanel.vue`。

`CodeDiffPanel` 是 AST diff 分支上的结果面板。它不做 AST 分析本身，而是把已经算好的 hunk 再组织成更适合代码语义阅读的视图。

## 1. 组件定位

这个组件负责两层事情：

- 先把有变化的 symbol 做摘要
- 再把 hunk 交给 `DiffViewer` 做正文渲染

所以它并不是新的 diff 引擎，而是 AST 模式上的结果整形层。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `hunks: DiffHunk[]`
- `viewMode: 'unified' | 'split'`

### 2.2 Emits

它没有自定义事件。

这说明它是一个纯结果展示组件。

## 3. 派生逻辑

组件内部有三个关键派生：

- `changedHunks`
- `displayHunks`
- `symbolSummary`

### 3.1 `changedHunks`

先把 `kind !== 'unchanged'` 的 hunk 过滤出来。

### 3.2 `displayHunks`

如果确实有变化 hunk，就只展示变化部分；否则回退到展示原始全部 hunk。

这让组件在“没有结构变化但仍有上下文”时不会直接空掉。

### 3.3 `symbolSummary`

它会按：

- `symbol_type`
- 或 `symbol`
- 或 `'symbol'`

做计数汇总，然后按出现次数降序排列。

这给 AST diff 提供了一层“改动集中在哪类符号”的概览。

## 4. 模板结构

模板分成三段：

- 顶部 `cd-summary` 芯片摘要
- 中部 `cd-symbols` 表格
- 底部 `DiffViewer`

表格里会展示：

- `Type`
- `Symbol`
- `Change`
- `Range`

因此它先给读者一个结构化摘要，再进入具体 diff 细节。

## 5. 架构意义

`CodeDiffPanel` 的意义在于，它把 AST diff 从普通文本 diff 中区分出来：

- 文本正文仍可复用 `DiffViewer`
- 但语义摘要和符号表由这一层单独补充

这很符合“复用通用 viewer，同时为语义模式加薄增强层”的设计思路。

## 6. 组件特征总结

一句话概括 `CodeDiffPanel.vue`：

- **它是 AST diff 的结果整形层，负责先做符号摘要，再委托 `DiffViewer` 渲染正文。**

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `changedHunks/displayHunks`
2. 再看 `symbolSummary`
3. 最后看模板里的摘要区、表格区和 `DiffViewer`
