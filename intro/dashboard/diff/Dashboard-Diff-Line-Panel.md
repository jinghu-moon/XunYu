# Dashboard Diff LineDiffPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/diff/LineDiffPanel.vue`。

`LineDiffPanel` 是 Diff 子系统里最薄的一层包装。它几乎不做任何额外逻辑，只把行级 diff 数据透传给 `DiffViewer`。

## 1. 组件定位

这个组件的职责只有一个：

- 为行级或 line fallback diff 提供一层语义化命名的外壳

它本身不做统计、不做筛选，也不做转换。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `hunks: DiffHunk[]`
- `viewMode: 'unified' | 'split'`
- `kind: 'line' | 'ast'`

### 2.2 Emits

它没有事件输出。

## 3. 逻辑复杂度

这个组件没有：

- 本地状态
- 派生数据
- watch
- 自定义函数

模板唯一做的事就是：

- `<DiffViewer :hunks="hunks" :view-mode="viewMode" :kind="kind" />`

所以它是一个非常纯的 pass-through 组件。

## 4. 架构意义

之所以保留 `LineDiffPanel`，而不是在 `DiffPanel` 里直接挂 `DiffViewer`，核心价值在于：

- 让行级 diff 有自己的语义边界
- 保持和 `CodeDiffPanel` 的分层对称
- 将来如果要给 line 模式加额外增强，不必直接改容器层

这是一个很典型的“预留扩展点，但当前保持极薄”的组件。

## 5. 组件特征总结

一句话概括 `LineDiffPanel.vue`：

- **它是对 `DiffViewer` 的语义化轻封装，用来承接 line 模式正文。**

## 6. 推荐阅读顺序

建议这样读：

1. 先看 `props`
2. 再看模板中的 `DiffViewer` 透传
3. 最后回到 `DiffPanel` 看它和 `CodeDiffPanel` 的分工对照
