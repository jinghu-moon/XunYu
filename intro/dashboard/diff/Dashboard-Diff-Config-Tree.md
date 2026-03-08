# Dashboard Diff ConfigDiffTree 导读

这篇文档专门拆 `dashboard-ui/src/components/diff/ConfigDiffTree.vue`。

`ConfigDiffTree` 是配置语义 diff 的递归树组件。它不做配置解析，也不决定 diff 策略，只负责把已经构建好的 `ConfigDiffNode` 树按可展开的形式渲染出来。

## 1. 组件定位

这个组件负责：

- 渲染一个配置 diff 节点
- 在节点有子项时支持展开/收起
- 递归渲染整棵配置 diff 树
- 把叶子节点值变化压缩成一行摘要文本

它是配置语义 diff 的核心呈现组件。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `node: ConfigDiffNode`
- `level?: number`

其中 `level` 默认是 `0`。

### 2.2 Emits

它没有事件输出。

展开/收起完全由组件自己的局部状态处理。

## 3. 本地状态与派生逻辑

### 3.1 本地状态

- `expanded`

默认规则是：

- `level < 2` 时默认展开

这让树在初次打开时有一定可读性，但不会整棵全爆开。

### 3.2 派生

核心派生包括：

- `hasChildren`
- `rowStyle`
- `statusText`
- `leafText`

其中 `rowStyle` 用左侧缩进体现层级，`leafText` 则把叶子节点的 old/new 值按状态格式化成简短文本。

### 3.3 `formatValue()`

它会对值做截断和字符串化处理：

- 字符串过长时截断
- 对对象尝试 `JSON.stringify`
- 无法序列化时回退成 `String(value)`

这让树行不会被超长值撑爆。

## 4. 模板结构

每个节点分两层：

- 一行 `cfg-row`
- 一个可选的 `cfg-children`

行内会显示：

- 展开按钮
- `key`
- `kind`
- `status`
- 子节点数量或叶子值摘要

有子节点时，会递归继续渲染 `ConfigDiffTree`。

## 5. 架构意义

`ConfigDiffTree` 的意义在于，它把配置 diff 的层级结构完整保留下来了，而不是退化成平面表格。

这对于 JSON/TOML/YAML 一类配置文件尤其重要，因为结构变化本身就是信息。

## 6. 组件特征总结

一句话概括 `ConfigDiffTree.vue`：

- **它是配置语义 diff 的递归树视图，重点在层级、状态和叶子值摘要。**

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `expanded/hasChildren`
2. 再看 `statusText/leafText/formatValue()`
3. 最后看模板里的递归渲染结构
