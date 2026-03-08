# Dashboard DensityToggle 导读

这篇文档专门拆 `dashboard-ui/src/components/DensityToggle.vue`。

`DensityToggle` 是一个非常薄的全局样式上下文切换器。它不直接改表格组件本身，而是通过修改根节点 class，让整套样式变量系统进入不同密度模式。

## 1. 组件定位

这个组件只负责：

- 在 `compact` / `spacious` 之间切换
- 把用户偏好写进 `localStorage`
- 把密度 class 挂到 `document.documentElement`

它不是布局管理器，而是密度偏好的前端开关。

## 2. 输入输出边界

这个组件没有 `props`，也没有 `emits`。

它本质上是一个纯本地偏好组件：

- 状态完全由自身维护
- 生效方式是操作 DOM 根节点 class

## 3. 本地状态与逻辑

### 3.1 本地状态

- `density`

支持两个值：

- `compact`
- `spacious`

### 3.2 `applyDensity()`

切换时会在根节点上切换：

- `density-compact`
- `density-spacious`

所以组件本身不改任何具体间距值，只负责切换样式上下文。

### 3.3 初始化与持久化

- `onMounted()` 时从 `localStorage` 读取 `densityPreference`
- watch `density` 时把偏好写回 `localStorage`

这保证了用户刷新页面后密度仍然保持一致。

## 4. 模板结构

模板非常简单：

- 一段 `Density` 标签
- 一个按钮组 `Compact / Spacious`

按钮组带有：

- `role="group"`
- `aria-label="Table density"`

说明它也考虑了基础可访问性。

## 5. 架构意义

`DensityToggle` 的价值不在于界面复杂，而在于它把密度偏好和具体组件实现解耦了：

- 开关只切模式
- 样式变量系统负责真正落地间距差异

这是一种很符合 DRY 的做法。

## 6. 组件特征总结

一句话概括 `DensityToggle.vue`：

- **它是一个通过根节点 class 切换全局密度模式的偏好开关。**

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `density` 状态和 `options`
2. 再看 `applyDensity()`
3. 最后看 `onMounted + watch` 与模板按钮组
