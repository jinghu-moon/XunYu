# Dashboard EnvVarHistoryDrawer 导读

这篇文档专门拆 `dashboard-ui/src/components/EnvVarHistoryDrawer.vue`。

`EnvVarHistoryDrawer` 是 Env 工作台里的上下文浮层组件。它不自己获取历史数据，也不自己决定何时打开，而是由容器层在用户点击变量历史时驱动显示。

所以它不是独立页面，而是一个单变量历史的侧边抽屉。

## 1. 组件定位

这个组件负责：

- 在页面右侧打开一个抽屉
- 展示某个变量的历史记录时间线
- 提供关闭入口

它解决的是“从变量主表快速钻到历史细节”的问题。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `varName: string | null`
- `entries: EnvAuditEntry[]`
- `loading?: boolean`

这里最关键的控制字段是：

- `varName`

只要它是 `null`，整个抽屉就不会渲染出来。

### 2.2 Emits

它只向外抛出一个事件：

- `close`

这意味着：

- 打开由父层决定
- 关闭也只是告诉父层“可以收起了”
- 历史数据加载与清空都不在这里

## 3. 结构与交互逻辑

### 3.1 `Teleport` 到 `body`

组件使用了：

- `Teleport to="body"`

这说明它不是普通文档流中的一个区域，而是标准的浮层实现。这样做可以避免被父布局裁切，并保持抽屉层级独立。

### 3.2 显示条件

只有 `varName` 存在时才会同时渲染：

- 背景遮罩 `backdrop`
- 抽屉主体 `drawer`

这是一种非常干净的“单一显隐开关”设计。

### 3.3 关闭方式

组件提供两种关闭手段：

- 点击遮罩关闭
- 点击 `Close` 按钮关闭

两者最终都统一发出 `close`。

## 4. 模板结构

### 4.1 头部区

头部会展示：

- 当前变量名 `varName`
- 副标题 `Variable history`
- `Close`

因此用户进入抽屉后，第一眼就知道自己正在看哪一个变量。

### 4.2 内容区

内容区有三种状态：

- `loading` -> `Loading...`
- 空列表 -> `No history entries.`
- 有数据 -> 时间线列表

### 4.3 时间线区

每条记录会展示：

- `at`
- `action`
- `result`
- `message`

它并没有展示完整 before/after 值，也没有分组与分页，仍然保持在“够看历史脉络”的粒度上。

## 5. 架构意义

`EnvVarHistoryDrawer` 的价值，在于它把变量历史查看从主列表中抽离出来，避免 `EnvVarsTable` 变成一个过重的复合组件。

这带来两个明显好处：

- 主表仍然保持浏览和轻编辑职责
- 历史查看可以作为按需打开的二级视图存在

它是一个很标准的“容器控制型浮层组件”。

## 6. 组件特征总结

一句话概括 `EnvVarHistoryDrawer.vue`：

- **它是由容器完全驱动的单变量历史抽屉，只负责浮层展示与关闭反馈。**

最值得注意的点有四个：

- 使用 `Teleport` 挂到 `body`
- `varName` 同时控制遮罩和抽屉显隐
- 没有任何本地业务状态
- 历史数据类型复用了 `EnvAuditEntry`

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `props + emits`，确认它是纯受控浮层
2. 再看 `Teleport` 与 `varName` 条件渲染
3. 最后看时间线的 loading/empty/list 三态

读完后回到 `EnvVarsTable` 看 `show-history`，再回到 `EnvPanel` 看 `onShowHistory()`，整条交互链就能串起来了。
