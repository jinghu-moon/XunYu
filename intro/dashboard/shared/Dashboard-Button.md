# Dashboard Button 组件导读

这篇文档专门拆 `dashboard-ui/src/components/button/Button.vue`。

`Button.vue` 是 Dashboard 的按钮原语。它不是某个面板的业务按钮，而是一层统一的按钮包装，用于把样式预设、尺寸、加载态和可访问性收口到一个组件里。

## 1. 组件定位

这个组件负责：

- 统一按钮尺寸与预设样式
- 处理 `disabled` 与 `loading` 的组合语义
- 提供方形 icon-only 按钮模式
- 在加载态下展示 spinner

它本质上是 Dashboard 设计系统里的基础交互原件。

## 2. 输入输出边界

### 2.1 Props

它使用 `ButtonProps`，核心字段包括：

- `preset?: 'secondary' | 'primary' | 'danger' | 'ghost'`
- `size?: 'sm' | 'md' | 'lg'`
- `square?`
- `disabled?`
- `loading?`
- `type?: 'button' | 'submit' | 'reset'`

### 2.2 Emits

这个组件不声明自定义事件。

也就是说，它保留原生 `<button>` 交互模式，父层直接监听原生点击事件即可。

## 3. 核心逻辑

组件内部只有一个关键派生：

- `isDisabled = props.disabled || props.loading`

这意味着只要进入加载态，按钮就会自动视为不可点击。

这是一种很常见、也很稳妥的交互保护策略。

## 4. 模板结构

模板仍然是原生 `<button>`，只是额外统一了几件事：

- `type`
- `disabled`
- `aria-busy`
- class 组合
- 加载 spinner
- 默认 slot 内容

所以它不是重新发明按钮，而是对原生按钮做了一层设计系统封装。

## 5. 样式层特点

### 5.1 预设样式

支持四种 preset：

- `secondary`
- `primary`
- `danger`
- `ghost`

### 5.2 尺寸系统

支持：

- `sm`
- `md`
- `lg`

### 5.3 方形模式

`square` 模式下会把宽度收成与高度相同，适合纯图标按钮。

### 5.4 加载与确认徽标

- `loading` 时会显示 spinner，并降低内容透明度
- 样式里还支持 `btn--confirm` 和插槽徽标类 `btn__confirm-badge`

这说明它被设计成可以覆盖一些“需要二次确认提示”的按钮场景。

## 6. 使用位置

当前项目里，这个按钮原语已经被大量业务面板复用，例如：

- `BookmarksPanel`
- `ConfigPanel`
- `AuditPanel`
- `HomePanel`
- `PortsPanel`
- `ProxyPanel`
- `RedirectPanel`

所以它是一个已经被广泛落地的基础组件，而不是未来预留。

## 7. 组件特征总结

一句话概括 `Button.vue`：

- **它是 Dashboard 的按钮设计原语，把预设样式、尺寸、加载态与原生语义统一封装起来。**

## 8. 推荐阅读顺序

建议这样读：

1. 先看 `button/types.ts` 里的 `ButtonProps`
2. 再看 `isDisabled`
3. 接着看模板里的 `<button + spinner + slot>`
4. 最后看预设样式、尺寸和 `square/loading/confirm` 这几类样式分支
