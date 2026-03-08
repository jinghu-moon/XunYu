# Dashboard feedback.ts 导读

这篇文档专门拆 `dashboard-ui/src/ui/feedback.ts`。

它不是可视组件，而是 Dashboard 的**全局反馈状态源**：统一维护 loading 计数、toast 队列，以及错误通知去重逻辑。

## 1. 它解决什么问题

如果每个面板都各自维护一套“请求中”“成功提示”“失败提示”，会立刻出现三类问题：

- loading 展示位置不统一
- toast 样式和生命周期不统一
- 同一错误可能在组件层、API 层、全局异常层被重复提示

`feedback.ts` 的作用就是把这些问题统一收口，让所有组件共享一套反馈机制。

## 2. 它维护了哪些状态

文件里只有一个响应式全局状态：

- `toasts`：当前 toast 队列
- `loadingCount`：全局进行中的异步请求数量

这意味着它不是“面板级 store”，而是“应用级反馈 store”。

## 3. 核心导出函数

### 3.1 `useFeedbackState()`

把全局响应式状态暴露给视图层消费。

当前直接消费它的核心组件是：

- `dashboard-ui/src/components/GlobalFeedback.vue`

也就是说，`GlobalFeedback` 只是渲染器，真正的状态源在这里。

### 3.2 `beginLoading()` / `endLoading()`

这两个函数通过引用计数方式维护全局 loading：

- 开始请求时 `+1`
- 结束请求时 `-1`
- `endLoading()` 会把下限钉在 `0`

这个设计比单一布尔值稳，因为 Dashboard 同时可能有多个并发请求。

### 3.3 `pushToast()` / `removeToast()`

这套 API 负责 toast 生命周期：

- 分配递增 id
- 新 toast 插入队首
- 最多只保留 6 条
- 按 level 自动决定默认 TTL
- TTL 到期后自动移除
- 手动关闭时同步清理 timer

这里的设计重点是：**状态队列和计时器表分离**。视图只关心 toast 列表，定时器由工具层自己管理。

### 3.4 `notifyError()` / `isToastMarked()`

这是这个文件最关键的地方。

`notifyError()` 不只是“把错误弹出来”，它还会：

- 统一把未知错误格式化成可展示文本
- 为错误对象打上 `TOAST_MARK`
- 避免同一个错误被重复 toast

这让它非常适合放在多层错误链路的公共末端。

## 4. 谁在用它

### 4.1 `api.ts`

`dashboard-ui/src/api.ts` 是 loading / 错误通知的主要入口：

- 请求开始前调 `beginLoading()`
- 请求结束后调 `endLoading()`
- 请求失败时调 `notifyError()`

所以大多数业务面板并不直接处理“全局 loading”，而是通过 `api.ts -> feedback.ts` 间接完成。

### 4.2 `App.vue`

`dashboard-ui/src/App.vue` 用它处理全局漏网异常：

- `unhandledrejection`
- `window.error`

这说明 `feedback.ts` 同时承接两条错误链：

- 正常请求链路
- 全局兜底异常链路

### 4.3 业务面板

有些业务组件也会直接使用它发出成功/警告提示，例如：

- `BookmarksPanel.vue`
- `AuditPanel.vue`
- `PortsPanel.vue`
- `ConfigPanel.vue`

所以它既服务于底层 API，也服务于业务交互确认。

## 5. 为什么它不做成组件

因为反馈状态和反馈渲染是两件事：

- `feedback.ts` 负责状态与规则
- `GlobalFeedback.vue` 负责视觉出口

这样拆的好处是：

- 任意模块都能在不依赖组件树的情况下发 toast
- 反馈规则可以被 API 层与壳层共同复用
- 视图层保持很薄，便于替换样式

## 6. 一句话概括

`feedback.ts` 是 Dashboard 的全局反馈总线：它统一管理并发 loading、toast 队列和错误去重，而 `GlobalFeedback.vue` 只是这条总线的可视化出口。

## 7. 建议连读

1. `./Dashboard-Global-Feedback.md`
2. `../Dashboard-Foundation.md`
3. `dashboard-ui/src/api.ts`
4. `dashboard-ui/src/App.vue`
