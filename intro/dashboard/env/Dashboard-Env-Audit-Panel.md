# Dashboard EnvAuditPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/EnvAuditPanel.vue`。

`EnvAuditPanel` 是 Env 工作台里的审计摘要面板。它的职责非常明确：把一批审计记录列表化展示出来。

所以它不是审计分析器，也不是可搜索的日志平台，而是一张非常薄的只读结果卡片。

## 1. 组件定位

这个组件只做三件事：

- 刷新审计列表
- 展示加载/空态
- 以表格方式显示近期审计记录

它和 `EnvVarHistoryDrawer` 的区别在于：

- `EnvAuditPanel` 看的是全局审计摘要
- `EnvVarHistoryDrawer` 看的是单变量历史切片

## 2. 输入输出边界

### 2.1 Props

它接收：

- `entries: EnvAuditEntry[]`
- `loading?: boolean`

`EnvAuditEntry` 在这个面板里会展示到的字段包括：

- `at`
- `action`
- `scope`
- `result`
- `name`
- `message`

### 2.2 Emits

它只向外抛出一个动作：

- `refresh`

这说明它完全不接管数据来源，也不提供二次筛选、排序或搜索逻辑。

## 3. 逻辑复杂度

这个组件没有本地状态，也没有派生逻辑。

它唯一值得注意的处理是：

- 表格只渲染 `entries.slice(0, 80)`

也就是说，它刻意把自己限制成一个“最近若干条记录的摘要表”，而不是无限滚动日志视图。

## 4. 模板结构

### 4.1 头部区

头部包含：

- 标题 `Audit`
- `Refresh`

### 4.2 状态区

它会根据状态渲染三种分支：

- `loading` -> `Loading...`
- 空列表 -> `No audit entries.`
- 有数据 -> 表格

### 4.3 表格区

表格列固定为：

- `At`
- `Action`
- `Scope`
- `Result`
- `Name`
- `Message`

其中：

- `result === 'ok'` 用 `ok` 样式
- 否则用 `error` 样式

说明它只对最核心的成功/失败状态做了轻量强调。

## 5. 架构意义

`EnvAuditPanel` 的作用不是“深度分析日志”，而是给整个 Env 工作台一个全局可见的近期审计窗口。

这个边界很合理，因为：

- 复杂审计分析会让组件迅速膨胀
- 变量级细节已经由 `EnvVarHistoryDrawer` 承担
- 容器层可以独立决定一次拉多少条记录

它体现的是一种摘要视图思路，而不是全功能日志中心。

## 6. 组件特征总结

一句话概括 `EnvAuditPanel.vue`：

- **它是近期审计记录的只读摘要表，强调快速查看而不是深入分析。**

最值得注意的点有三个：

- 无本地状态
- 只有 `refresh` 一个动作出口
- 显式限制为前 80 条记录

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `props + emits`，确认它是纯只读卡片
2. 再看模板里的三种状态分支
3. 最后注意 `entries.slice(0, 80)` 这个摘要化处理

读完后再和 `Dashboard-Env-Var-History-Drawer.md` 对照看，会更容易分清“全局审计”和“单变量历史”。


