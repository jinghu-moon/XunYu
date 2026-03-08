# Dashboard DiffOptions 导读

这篇文档专门拆 `dashboard-ui/src/components/diff/DiffOptions.vue`。

`DiffOptions` 是一个非常小的组件，但它在架构上很干净：**它不持有自己的业务状态，只作为 `DiffPanel` 的参数编辑视图存在。**

## 1. 组件定位

它解决的事情非常单一：

- 把 diff 参数可视化成一组表单控件
- 通过双向绑定把参数直接写回父容器状态

它不是“带保存按钮的小配置面板”，而是一个纯参数视图。

## 2. 输入输出边界

这个组件没有 `props + emits` 的传统组合，而是使用：

- `defineModel(...)`

模型字段包括：

- `mode`
- `algorithm`
- `context`
- `ignore_space_change`
- `ignore_all_space`
- `ignore_blank_lines`
- `strip_trailing_cr`
- `force_text`

也就是说，`DiffOptions` 自己不声明局部镜像状态，直接对父级 `options` 做双向读写。

## 3. 为什么这个组件很“薄”

这份源码几乎只有两部分：

- 一行 `defineModel(...)`
- 一段模板

没有：

- 额外 `ref`
- 额外 `computed`
- 额外 `watch`
- 保存 / 重置逻辑
- 参数校验逻辑

这恰恰说明它的边界被控制得很好：

- 参数定义由父层决定
- 参数使用由父层决定
- 它只负责展示和改值

## 4. 模板结构

## 4.1 第一行：核心策略参数

第一行 3 个字段：

- `Mode`
  - `Auto`
  - `Line`
  - `AST`
- `Algorithm`
  - `Histogram`
  - `Myers`
  - `Minimal`
  - `Patience`
- `Context`
  - 数字输入，范围 `0-50`

也就是说，第一行主要是“diff 怎么跑”的核心策略。

## 4.2 第二行：布尔开关参数

第二行是 5 个 checkbox：

- `Ignore space change`
- `Ignore all space`
- `Ignore blank lines`
- `Strip trailing CR`
- `Force text`

这组开关更偏“diff 结果如何归一化 / 容错化”。

## 5. 架构意义

虽然它很小，但这组件的存在很有意义：

1. **把参数面板从 `DiffPanel` 主模板中抽出来**，避免主容器模板过重。
2. **保持纯视图属性**，不让参数编辑器自己再长出业务逻辑。
3. **天然适合折叠显示**，因为它本身就是一块独立参数区。

## 6. 组件特征总结

`DiffOptions.vue` 的特点只有一句话：

- **极薄、纯表单、零业务状态。**

从工程角度看，这是一个很理想的小组件，因为它：

- 可复用
- 易替换
- 易测试
- 不会和容器层状态竞争控制权

## 7. 推荐阅读顺序

这个组件非常简单，直接这样看就够了：

1. 先看 `defineModel(...)` 里的字段列表
2. 再看模板里这些字段是如何映射到输入控件的

读完后你会明白：`DiffOptions` 的价值不在复杂逻辑，而在于它把复杂参数集干净地抽成了一块独立视图。
