# Dashboard SkeletonTable 导读

这篇文档专门拆 `dashboard-ui/src/components/SkeletonTable.vue`。

`SkeletonTable` 是 Dashboard 各面板共享的骨架屏组件。它不关心具体业务数据，只负责用规则化网格快速占位，让“表格还在加载中”这件事有统一视觉表达。

## 1. 组件定位

这个组件解决的是非常纯粹的一件事：

- 在表格数据加载期间提供统一 skeleton 占位

它不是某个面板的私有实现，而是一个通用占位原语。

## 2. 输入输出边界

### 2.1 Props

它接收两个可选参数：

- `rows?`
- `columns?`

默认值分别是：

- `rows = 6`
- `columns = 5`

### 2.2 Emits

它没有任何事件输出。

这符合它作为纯展示组件的定位。

## 3. 派生逻辑

组件内部有 4 个派生：

- `rowItems`
- `normalizedCols`
- `colItems`
- `gridClass`

其中最关键的是 `normalizedCols`：

- 最少 2 列
- 最多 8 列

这说明它对外部传入列数做了收敛，不让布局无限散开。

## 4. 模板结构

模板是一个很规整的双层循环：

- 外层按行渲染 `skeleton-row`
- 内层按列渲染 `skeleton-cell`

没有表头、没有语义内容，纯粹就是视觉占位。

## 5. 样式特点

`skeleton-cell` 通过 `::after` 做线性渐变 shimmer 动画。

并且按 `cols-2 ~ cols-8` 提前定义好网格模板列数，所以布局计算非常直接，没有运行时复杂样式拼接。

## 6. 架构意义

`SkeletonTable` 的意义是把“加载中表格占位”从各业务面板里抽出来：

- Bookmarks、Audit、Home、Ports 都能复用
- 面板本身不用重复写一套 shimmer 结构

这很符合 DRY。

## 7. 组件特征总结

一句话概括 `SkeletonTable.vue`：

- **它是 Dashboard 表格类视图的统一骨架屏原语。**

## 8. 推荐阅读顺序

建议这样读：

1. 先看 `rows/columns` 默认值
2. 再看 `normalizedCols/gridClass`
3. 最后看模板双循环与 shimmer 样式
