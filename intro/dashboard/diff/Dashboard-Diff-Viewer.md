# Dashboard Diff DiffViewer 导读

这篇文档专门拆 `dashboard-ui/src/components/diff/DiffViewer.vue`。

`DiffViewer` 是 Diff 子系统里最底层的文本 diff 渲染器。它不负责文件选择、模式判断或统计摘要，只负责把 `DiffHunk[]` 渲染成：

- unified 视图
- split 视图

因此它是一个纯粹的渲染原语。

## 1. 组件定位

这个组件负责：

- 把 hunk 头与 diff 行转成可渲染表格行
- 统一处理行号、`+/-` 标记和正文文本
- 支持 unified / split 两种展现方式

它不是 diff 算法实现，而是结果可视化层。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `hunks: DiffHunk[]`
- `viewMode: 'unified' | 'split'`
- `kind?: 'line' | 'ast'`

其中 `kind` 更像语义标识，真正决定渲染形态的是 `viewMode`。

### 2.2 Emits

它没有自定义事件。

## 3. 核心转换逻辑

### 3.1 `unifiedRows`

在 unified 模式下，组件会把每个 hunk 展开成：

- 一行 hunk header
- 若干行 diff line

并维护：

- `oldNum`
- `newNum`

### 3.2 `pairLines()`

这是 split 模式的核心。它会把：

- `remove` 行
- `add` 行

尽量按组配对，生成左右并排的 `SplitRow`。

也就是说，split 视图不是简单把两边原样平铺，而是先做一次行级配对整理。

### 3.3 `splitRows`

在此基础上，为每个 hunk 先插入 header，再追加 `pairLines()` 结果。

### 3.4 辅助函数

- `hunkHeader()`：拼出 `@@ -a,b +c,d @@` 头，并补符号或 section
- `lineMarker()`：把 tag 转成 `+ / - / 空格`

## 4. 模板结构

### 4.1 Unified 表格

统一视图展示：

- old 行号
- new 行号
- marker
- content

### 4.2 Split 表格

并排视图展示：

- 左侧旧行号与旧内容
- 中间 gutter
- 右侧新行号与新内容

不同 tag 会附带：

- add 背景
- remove 背景
- empty 背景

## 5. 架构意义

`DiffViewer` 的价值在于，它把最底层的 diff 表格渲染稳定封装下来：

- `LineDiffPanel` 直接复用它
- `CodeDiffPanel` 也复用它

所以更上层组件只需决定“喂什么 hunks”和“选哪种视图”，不用重复关心行号和表格渲染细节。

## 6. 组件特征总结

一句话概括 `DiffViewer.vue`：

- **它是 Diff 子系统的底层表格渲染器，负责 unified/split 两种正文视图。**

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `unifiedRows`
2. 再看 `pairLines()` 和 `splitRows`
3. 最后看 unified / split 两段模板分支
