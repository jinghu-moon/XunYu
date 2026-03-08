# Dashboard Diff DiffStats 导读

这篇文档专门拆 `dashboard-ui/src/components/diff/DiffStats.vue`。

`DiffStats` 是 Diff 工作台最薄的摘要组件。它不处理 diff 内容本身，只负责把增删改未变这四类数量压缩成一排统计芯片。

## 1. 组件定位

这个组件只解决一件事：

- 用统一视觉展示 diff 统计数字

它是一个典型的纯展示摘要条。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `added`
- `removed`
- `modified`
- `unchanged`
- `unitLabel?`

默认 `unitLabel` 是：

- `items`

### 2.2 Emits

它没有任何事件输出。

## 3. 派生逻辑

唯一的派生是：

- `total = added + removed + modified + unchanged`

所以它的职责极其单纯。

## 4. 模板结构

模板会固定渲染 5 个片段：

- `+ added`
- `- removed`
- `~ modified`
- `= unchanged`
- `total + unitLabel`

并且四类芯片分别使用成功、危险、警告、中性语义色。

## 5. 架构意义

`DiffStats` 的存在价值很直接：

- 让 config / ast / line 三种 diff 模式都可以复用同一套统计摘要 UI

这让容器层只需要换数据和单位，不需要重复写摘要布局。

## 6. 组件特征总结

一句话概括 `DiffStats.vue`：

- **它是 Diff 结果的统一统计摘要条。**

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `props`
2. 再看 `total`
3. 最后看模板里的 4 类芯片和总数文本
