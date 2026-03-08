# Dashboard GlobalFeedback 导读

这篇文档专门拆 `dashboard-ui/src/components/GlobalFeedback.vue`。

`GlobalFeedback` 是 Dashboard 的全局反馈出口。它自己不产生 toast，也不自己统计 loading，而是把 `ui/feedback.ts` 里的全局状态可视化出来。

## 1. 组件定位

这个组件负责两类全局反馈：

- 全局 loading 提示
- 全局 toast 栈

它不是业务面板，而是整个 Dashboard 顶层的反馈终点。

## 2. 输入输出边界

这个组件没有 `props`，也没有 `emits`。

它直接消费：

- `useFeedbackState()`
- `removeToast()`

也就是说，状态来源是共享反馈模块，而不是父组件透传。

## 3. 本地逻辑

组件只有一个派生：

- `isLoading = state.loadingCount > 0`

除此之外没有本地状态。

这让它成为一个非常纯粹的“状态投影视图”。

## 4. 模板结构

### 4.1 Loading 区

如果 `isLoading` 为真，就展示：

- 小型 spinner
- `Loading...`

它放在右下角，语义上更像全局请求中的轻提示，而不是遮罩层。

### 4.2 Toast 栈

toast 栈会遍历 `state.toasts`，展示：

- `title`
- 可选 `detail`
- 手动关闭按钮

每条 toast 还会根据 `level` 带不同样式：

- `error`
- `warning`
- `info`
- `success`

## 5. 和 `ui/feedback.ts` 的关系

这个组件只有放到 `ui/feedback.ts` 背景下才完整：

- `beginLoading()/endLoading()` 控制 `loadingCount`
- `pushToast()` 推入 toast
- `removeToast()` 负责手动或自动移除
- `notifyError()` 统一把异常转成错误 toast

所以 `GlobalFeedback` 是视图层，`ui/feedback.ts` 才是反馈状态源。

## 6. 架构意义

把反馈系统拆成“状态模块 + 渲染组件”有两个明显好处：

- 任意 API 或组件都能复用统一反馈入口
- 顶层壳层只需要挂一个反馈组件即可

## 7. 组件特征总结

一句话概括 `GlobalFeedback.vue`：

- **它是 Dashboard 全局反馈状态的可视化出口，而不是反馈状态的生产者。**

## 8. 推荐阅读顺序

建议这样读：

1. 先看 `ui/feedback.ts`
2. 再看 `GlobalFeedback.vue` 里 `useFeedbackState()` 和 `isLoading`
3. 最后看模板里的 loading 区和 toast 栈
