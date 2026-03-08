# Dashboard ThemeToggle 导读

这篇文档专门拆 `dashboard-ui/src/components/ThemeToggle.vue`。

`ThemeToggle` 是 Dashboard 的全局主题切换器。它不只是简单切亮暗色，还同时处理：

- 用户主题偏好持久化
- 跟随系统主题
- 点击外部关闭菜单
- 在支持环境下用 View Transition API 做切换动画

所以它是一个比 `DensityToggle` 更完整的全局偏好组件。

## 1. 组件定位

这个组件负责：

- 在 `system / light / dark` 三种模式间切换
- 显示当前模式对应图标
- 在下拉菜单中选择主题
- 把主题状态同步到根节点 class

它是 Dashboard 壳层级的视觉上下文控制器。

## 2. 输入输出边界

这个组件没有 `props`，也没有 `emits`。

它的状态来源完全是：

- 本地 `ref`
- `localStorage`
- `window.matchMedia('(prefers-color-scheme: light)')`
- `document.documentElement`

## 3. 本地状态与核心逻辑

### 3.1 本地状态

- `currentThemeStatus`
- `theme`
- `showThemeMenu`
- `mediaQuery`

其中：

- `currentThemeStatus` 表示用户偏好值
- `theme` 表示当前实际生效的亮/暗状态

### 3.2 初始化

`onMounted()` 时会：

- 从 `localStorage.themePreference` 读取偏好
- 立即应用当前主题
- 注册点击外部关闭菜单逻辑
- 监听系统主题变化

### 3.3 `applyTheme()`

这是最关键的函数：

- 如果是 `system`，先根据系统浅色偏好推导实际亮暗
- 再比较当前主题是否真的发生变化
- 如果浏览器支持 `document.startViewTransition` 且用户未开启减少动画，就用圆形扩散动画做主题切换
- 否则直接调用 `executeThemeDOMUpdate()`

### 3.4 `executeThemeDOMUpdate()`

实际 DOM 更新非常直接：

- 更新 `theme`
- 更新 `currentThemeStatus`
- 在根节点上添加或移除 `light` class

### 3.5 `selectTheme()`

选择菜单项时：

- 先关闭菜单
- 只有真的切换时才写 `localStorage` 并调用 `applyTheme()`

## 4. 模板结构

模板包括：

- 一个触发按钮，显示当前主题图标
- 一个下拉菜单，列出 `System / Light / Dark`
- 当前选中项带 `active` 和勾选图标

这说明它是一个标准的“触发器 + 菜单”模式，而不是切换开关模式。

## 5. 架构意义

`ThemeToggle` 的价值不只是 UI，而是把主题策略统一收口：

- 用户偏好
- 系统偏好
- 动画能力探测
- DOM 根 class 切换

这些都被控制在一个小组件里，外部壳层无需知道细节。

## 6. 组件特征总结

一句话概括 `ThemeToggle.vue`：

- **它是一个带系统跟随和动画增强的全局主题偏好控制器。**

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `themeItems/currentThemeStatus/theme`
2. 再看 `onMounted()` 与系统主题监听
3. 接着看 `applyTheme()` 和 `executeThemeDOMUpdate()`
4. 最后看模板里的触发器与菜单
