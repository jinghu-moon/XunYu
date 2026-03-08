# Dashboard Diff DiffFilePreview 导读

这篇文档专门拆 `dashboard-ui/src/components/diff/DiffFilePreview.vue`。

`DiffFilePreview` 是 Diff 工作台里的右侧文件预览器。它不负责选文件，而是在拿到路径后并行拉取：

- 文件元信息
- 文件内容片段
- 可选的格式校验结果

因此它不是单纯的文本预览框，而是一块“预览 + 校验 + 懒加载”的综合结果面板。

## 1. 组件定位

这个组件负责：

- 展示当前选中文件路径
- 拉取并显示文件元信息
- 按分页窗口拉取文件内容
- 对可校验文件执行 validate
- 在校验错误和内容行号之间建立跳转

它是 `DiffFileManager` 的结果查看器，而不是文件选择器。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `path: string`
- `refreshKey?: number`

`refreshKey` 默认值是 `0`，用于强制触发重新加载。

### 2.2 Emits

它没有自定义事件。

原因很明确：这个组件已经直接调用了 `api.ts` 中的文件预览相关接口。

## 3. 本地状态与派生逻辑

它内部维护了三组主结果：

- `info`
- `content`
- `validation`

以及对应 loading / error 状态：

- `infoLoading`
- `contentLoading`
- `validationLoading`
- `validationError`
- `error`

还有几个关键辅助状态：

- `showValidationPanel`
- `requestSeq`
- `highlightedLine`

### 3.1 派生

关键派生包括：

- `activePath`
- `canValidate`
- `invalidValidation`
- `canLoadMore`

其中：

- `canValidate` 只对白名单扩展名开放校验
- `canLoadMore` 依赖 `content.truncated`

### 3.2 `requestSeq`

这是这个组件最关键的并发保护手段。

每次 `refresh()` 都会递增序号，后续异步请求返回时会校验序号一致性，避免旧请求回写新状态。

### 3.3 内容窗口与高亮跳转

组件只按 `PREVIEW_LIMIT` 拉一段内容。遇到校验错误跳转时，如果目标行不在当前窗口里，会重新加载覆盖该行的窗口，然后：

- 高亮该行
- 滚动到视口中央

这使得“校验错误 -> 跳到具体内容行”这条链路非常完整。

## 4. 核心逻辑

### 4.1 `refresh()`

并行执行：

- `loadInfo()`
- `loadContent()`
- `loadValidation()`

### 4.2 `loadMore()`

在内容被截断时按当前 offset 继续追加读取。

### 4.3 `jumpToValidationLine()`

如果目标行不在当前内容窗口，就先重载窗口，再高亮并滚动定位。

## 5. 模板结构

模板大体分为：

- 头部：标题、路径、校验状态、Refresh
- 元信息区：Size / Lines / Language / Class / Modified
- 可选校验详情面板
- 内容区：支持空态、二进制态、空文件态、文本表格态
- 底部：当前已加载行数与 `Load More`

所以它不是简单 `pre` 预览，而是一块完整的文件检查台。

## 6. 架构意义

`DiffFilePreview` 的意义在于，它把文件“看什么”与“怎么选文件”拆开了：

- `DiffFileManager` 负责选路径
- `DiffFilePreview` 负责解释这个路径对应的文件内容与状态

这让 Diff 工作台的文件侧边体验明显更完整。

## 7. 组件特征总结

一句话概括 `DiffFilePreview.vue`：

- **它是带并发保护、校验跳转和懒加载能力的文件预览器。**

## 8. 推荐阅读顺序

建议这样读：

1. 先看 `props` 与三组主状态 `info/content/validation`
2. 再看 `canValidate/canLoadMore/requestSeq`
3. 接着看 `refresh()`、`loadMore()`、`jumpToValidationLine()`
4. 最后看模板里的元信息区、校验区和内容区
