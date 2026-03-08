# Dashboard CapsuleTabs 导读

这篇文档专门拆 `dashboard-ui/src/components/CapsuleTabs.vue`。

`CapsuleTabs` 是 Dashboard 壳层里最核心的导航组件之一。它不是业务面板，而是一个可复用的标签切换器，负责把当前选中项、指示器动画、键盘导航和不同布局模式统一封装起来。

## 1. 组件定位

这个组件解决的是“如何把一组 tab 变成带动画和可访问性的胶囊导航”。

它负责：

- 根据 `modelValue` 高亮当前项
- 发出 `update:modelValue`
- 维护一个跟随选中项移动的胶囊指示器
- 支持横向 `flex` 与网格 `grid` 两种布局
- 支持键盘方向键导航

在 `App.vue` 里，它承接的是整个 Dashboard 的顶层工作台切换。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `modelValue`
- `items`
- `equalWidth?`
- `stretch?`
- `layout?: 'flex' | 'grid'`
- `gridCols?`

其中 `items` 的单项结构是：

- `value`
- `label`
- 可选 `icon`

### 2.2 Emits

它只向外抛出：

- `update:modelValue`

所以它保持了非常标准的受控组件形态：

- 当前选中项由父层控制
- 内部只负责表达切换意图

## 3. 本地状态与派生逻辑

组件内部虽然不复杂，但为了把动画做顺，需要维护一组 DOM 相关状态：

- `tabsWrapperRef`
- `tabsRef`
- `tabRefs`
- `indicatorStyle`
- `maxTabWidth`

同时有两个关键派生：

- `activeIndex`：当前选中项在 `items` 中的索引
- `gridStyle`：网格布局下的列数样式

### 3.1 `equalWidth`

如果启用 `equalWidth` 且当前不是 `grid` 布局，组件会扫描所有 tab 宽度，找出最大值，再把每个 tab 固定到这个宽度。

这保证了指示器和整体节奏不会因为文案长短不一而跳动。

### 3.2 指示器计算

`updateIndicator()` 会根据当前激活 tab 的：

- `offsetWidth`
- `offsetHeight`
- `offsetLeft`
- `offsetTop`

重算 `indicatorStyle`，让胶囊指示器平滑移动。

### 3.3 `scrollActiveIntoView()`

当选中项发生变化时，组件还会尝试把当前项滚动到可视区中央：

- `grid` 模式纵向滚动
- `flex` 模式横向滚动

所以它不只是“有指示器的 tabs”，还是一个会主动照顾可视区域的小导航器。

## 4. 交互逻辑

### 4.1 鼠标点击

`handleTabClick(value)` 很直接：

- 只发 `update:modelValue`

### 4.2 键盘导航

`handleKeyDown()` 支持：

- 横向模式：`ArrowLeft/ArrowRight`
- 网格模式：额外支持 `ArrowUp/ArrowDown`

这说明组件有明确的可访问性交互考虑，而不是纯鼠标组件。

### 4.3 监听与尺寸响应

组件会 watch：

- `modelValue`
- `equalWidth/layout/gridCols/items`

并在 `onMounted` 时通过 `ResizeObserver` 跟踪尺寸变化，自动重算宽度和指示器位置。

## 5. 模板结构

模板很清晰：

- 外层滚动容器 `capsule-tabs-wrapper`
- 内层 `tablist`
- 一层绝对定位的 `capsule-indicator`
- 多个 tab 项
- 一个 `extra` slot

这里有两个很有意思的细节：

- 每个 tab 都带 `role="tab"` 和 `aria-selected`
- 每个 tab 内部还放了一个 `tab-ghost`，用于保持视觉稳定

## 6. 架构意义

`CapsuleTabs` 的价值在于，它把 Dashboard 顶层导航从 `App.vue` 中抽成了一个足够成熟的 UI 原语。

这样做的好处是：

- 壳层不用自己维护动画细节
- 业务工作台切换不和 DOM 测量逻辑耦合
- 将来在别处也可以复用这套 tabs 能力

## 7. 组件特征总结

一句话概括 `CapsuleTabs.vue`：

- **它是一个受控的胶囊式 tab 导航原语，重点在指示器、布局适配和键盘可访问性。**

## 8. 推荐阅读顺序

建议这样读：

1. 先看 `props + emits`，确认它是标准受控组件
2. 再看 `activeIndex/gridStyle/maxTabWidth`
3. 接着看 `updateIndicator()`、`scrollActiveIntoView()`、`handleKeyDown()`
4. 最后看模板里的 `tablist + indicator + tab-ghost + extra slot`
