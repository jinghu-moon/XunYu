# Dashboard 基础骨架导读

这篇文档不再按“业务面板”拆，而是专门讲 `dashboard-ui` 的壳层、契约层、公共组件层和样式变量层。

如果你已经看过 `Dashboard-Components.md`，那篇更像“组件树地图”；这篇则更像“基础设施地图”。两篇配合看，能把 Dashboard 从“有哪些组件”推进到“这些组件是怎么被组织起来的”。

## 1. 分层视角

从源码结构看，Dashboard 大致分成 5 层：

1. **启动层**：`dashboard-ui/src/main.ts`
2. **应用壳层**：`dashboard-ui/src/App.vue`
3. **契约层**：`dashboard-ui/src/api.ts` + `dashboard-ui/src/types.ts`
4. **公共组件 / UI 工具层**：`dashboard-ui/src/components/*` 中的非业务组件，以及 `dashboard-ui/src/ui/*`
5. **样式变量层**：`dashboard-ui/src/styles/variable.css`、`variables-light.css`、`variables-dark.css`

这 5 层的关系可以理解为：

- `main.ts` 把应用和主题体系装起来。
- `App.vue` 负责“当前在哪个工作台”和“全局交互怎么统一处理”。
- `api.ts` / `types.ts` 把前后端边界统一起来。
- 公共组件层给业务面板提供可复用的壳、反馈和输入能力。
- 样式变量层为所有组件提供同一套视觉 token。

## 2. 启动层：`main.ts` 在做什么

`dashboard-ui/src/main.ts` 不只是简单地 `createApp(App)`。它实际承担了三件基础工作：

### 2.1 样式变量入口

文件最开始就引入了 `./styles/variable.css`。这意味着 Dashboard 的视觉系统不是每个组件各写各的，而是先进入一套全局 token，再由组件消费这些 token。

这套 token 包括：

- 间距、圆角、阴影、z-index、动画时长等基础设计变量
- 明暗主题下的颜色变量
- 表格、按钮、浮层等常用组件所需的复合变量
- 紧凑 / 宽松两种密度模式使用的尺寸变量

### 2.2 PrimeVue 主题预设接入

`main.ts` 引入了 `PrimeVue`、`Aura` 和 `definePreset`，并基于 Aura 做了一层自己的 preset 扩展。

这说明它的设计取向不是“直接吃默认主题”，而是：

- 复用 PrimeVue 的主题机制
- 再把项目自己的黑白灰视觉体系映射进去
- 让 PrimeVue 组件与自定义组件在视觉语言上尽量统一

所以 `main.ts` 的职责更接近“前端运行时装配器”，而不是单纯入口文件。

### 2.3 挂载应用

最后才是常规的：

- `createApp(App)`
- `app.use(PrimeVue, ...)`
- `app.mount('#app')`

也就是说，Dashboard 的真正入口顺序是：**设计变量先就位，主题策略先装配，最后才把 App 壳挂上去。**

## 3. 应用壳层：`App.vue` 负责什么

如果说各个 `*Panel.vue` 是工作台，那 `App.vue` 就是整个 Dashboard 的外壳和总控台。

### 3.1 顶层导航状态

`App.vue` 内部维护了一个顶层 `tab` 状态，目前覆盖 9 个一级工作台：

- `home`
- `bookmarks`
- `ports`
- `proxy`
- `config`
- `env`
- `redirect`
- `audit`
- `diff`

这些值一一映射到页面上的一级面板组件。也就是说，Dashboard 的一级信息架构并不是路由驱动，而是壳层内部的 tab 状态驱动。

### 3.2 全局命令面板

`App.vue` 还维护 `paletteOpen` 和命令列表 `commands`，并把它们传给 `CommandPalette.vue`。

这一层做的事情包括：

- 为每个一级工作台定义命令 id、标题、描述、关键词和执行动作
- 通过命令动作切换当前 tab
- 用 `Ctrl/Cmd + K` 打开命令面板

因此命令面板不是“某个组件自己做个弹窗”，而是壳层统一注册全局导航意图的入口。

### 3.3 全局错误兜底

`App.vue` 还注册了：

- `window.addEventListener('unhandledrejection', ...)`
- `window.addEventListener('error', ...)`
- `window.addEventListener('keydown', ...)`

它会把未处理异常和 Promise rejection 统一转给全局反馈体系。这个设计很关键：

- 各业务组件只需要对本地错误负责
- 壳层再补上“漏网异常”的最终兜底
- 用户看到的错误出口保持一致

### 3.4 顶层组合方式

模板层面，`App.vue` 做的是非常标准的壳层拼装：

- 顶部：`CapsuleTabs` + `DensityToggle` + `ThemeToggle`
- 中间：按当前 `tab` 选择一个业务面板
- 浮层：`CommandPalette`
- 全局反馈：`GlobalFeedback`

所以从架构职责看，`App.vue` 并不深入业务细节，而是专注在：

- 当前工作区切换
- 全局快捷入口
- 全局异常与反馈
- 公共 UI 控件装配

## 4. 契约层：`types.ts` 和 `api.ts`

Dashboard 的业务组件大多不直接接触裸 `fetch` 和随意拼结构，而是通过一套统一契约层工作。

## 4.1 `types.ts`：前后端共享语义模型

`dashboard-ui/src/types.ts` 是整个前端的数据边界文件。它按领域定义了 Dashboard 会消费的 DTO / ViewModel：

- **Bookmarks**：书签项
- **Ports**：端口项、端口响应
- **Proxy**：代理配置、代理状态、代理测试结果
- **Config**：全局配置、保护规则、树配置等
- **Redirect**：重定向规则、profile、dry-run 结果
- **Audit**：审计条目与统计
- **Diff**：文本 diff、配置 diff、文件浏览、文件转换、校验、WebSocket 事件
- **Env**：变量、快照、doctor、diff、profile、schema、annotation、模板、运行结果、状态汇总、WebSocket 事件

这层的价值在于：

- 组件之间不会各自发明一套字段名
- `api.ts` 的返回值和组件状态可以直接对齐
- 新增面板时，先补类型边界，再补请求逻辑和渲染逻辑，改动面更可控

## 4.2 `api.ts`：统一 HTTP / WebSocket 边界

`dashboard-ui/src/api.ts` 把所有请求都收口到同一个文件中，核心基础是：

- `const BASE = '/api'`
- `buildHttpError()`：把 HTTP 异常标准化
- `request()`：统一请求和错误处理入口

在这个基础上，`api.ts` 再按领域导出具体能力：

- Bookmarks：增删改查、批量标签操作
- Ports：端口查询、按端口/按 PID 终止
- Proxy：读取状态、读取配置、保存配置、设置/删除/测试代理
- Redirect：读取 profile、增删 profile、dry-run
- Audit：读取审计、导出 CSV
- Diff：文本 diff、文件树、文件搜索、文件信息、内容预览、格式转换、校验、Diff WebSocket
- Env：变量查询与修改、PATH 编辑、快照、doctor、导入导出、diff、图、审计、profile、schema、annotation、模板展开、命令运行、Env WebSocket

这里可以把 `api.ts` 理解成“前端版网关层”：

- 业务组件只表达“我要什么能力”
- 不重复处理 URL、HTTP 错误、解析逻辑
- WebSocket 也保持与 HTTP 相似的领域划分方式

## 5. 公共组件层

这层是 Dashboard 的真正“复用基础件”。它们不绑定某个具体业务面板，但决定了整个应用的交互一致性。

### 5.1 `CapsuleTabs.vue`

职责：一级导航壳。

它不是普通按钮列表，而是把顶层 tab 的几个共同需求都收进来了：

- 当前项高亮
- 指示器动画
- 键盘导航
- 多种布局形态

`App.vue` 之所以能把一级工作台切得很薄，一个原因就是顶层导航复杂度被 `CapsuleTabs` 吃掉了。

### 5.2 `CommandPalette.vue`

职责：全局命令检索与执行入口。

它负责：

- 根据 `modelValue` 控制显示与隐藏
- 对命令标题、描述、关键词做筛选
- 处理上下键选中与 Enter 执行
- 通过 Teleport 放到 `body` 级浮层

所以它更像“应用导航控制台”，不是某个业务面板里的搜索框。

### 5.3 `GlobalFeedback.vue`

职责：统一展示 loading 和 toast。

这个组件是 `ui/feedback.ts` 的可视化出口。其意义是把：

- 页面级 loading
- 请求错误 toast
- 成功 / 提示消息

统一放到一个全局反馈层里，避免每个组件自己维护一套提示样式。

### 5.4 `ThemeToggle.vue`

职责：主题策略切换。

它管理的是 `system / light / dark` 三态主题，不只是点按钮改个 class。它还包含：

- 主题状态持久化
- DOM 根节点主题状态应用
- 在支持时使用 View Transition API 做切换过渡

也就是说，它负责的是“主题运行时策略”。

### 5.5 `DensityToggle.vue`

职责：布局密度切换。

它专注做一件事：在根节点切换密度 class，并把结果写入 `localStorage`。这正好符合 KISS：

- 不与主题逻辑耦合
- 不与业务组件耦合
- 只负责 UI 密度这一个横切关注点

### 5.6 `SkeletonTable.vue`

职责：统一表格类骨架屏。

很多面板都有表格和列表，`SkeletonTable` 让这些面板在加载阶段不必各自复制一套占位 DOM。

### 5.7 `button/*`

`dashboard-ui/src/components/button/` 是一组很典型的“基础抽象层”：

- `Button.vue`：统一按钮实现
- `types.ts`：定义 `preset`、`size`、`loading`、`square` 等按钮契约
- `index.ts`：统一导出

它的价值不是做复杂按钮逻辑，而是消灭重复按钮样式和交互状态定义。

## 6. UI 工具层：`ui/*`

除组件外，还有 3 个值得单独关注的 UI 工具模块。

### 6.1 `ui/feedback.ts`

这是 Dashboard 的全局反馈状态源：

- `useFeedbackState()` 暴露响应式状态
- `beginLoading()` / `endLoading()` 维护全局 loading 计数
- `pushToast()` / `removeToast()` 管理 toast 队列
- `notifyError()` 统一把异常格式化后推给 toast
- `isToastMarked()` 避免同一错误被重复提示

它本质上是一个轻量全局 store，但没有引入更重的状态库。

### 6.2 `ui/export.ts`

这层负责把前端导出行为统一起来，例如时间戳文件名、CSV / 文本下载等。它的存在说明“导出”被视为横切能力，而不是零散塞到每个面板里。

### 6.3 `ui/tags.ts`

这层给标签相关的展示和处理提供统一小工具，减少书签等面板里重复写同样的 tag 逻辑。

## 7. 样式变量层：`styles/*`

Dashboard 的样式系统不是“组件里写死值”，而是基于一套变量驱动。

### 7.1 `variable.css`

这是总入口，定义大量基础 token，例如：

- spacing
- radius
- blur
- z-index
- duration / easing
- focus ring
- disabled opacity
- touch target

它还定义了很多组件级 token，例如按钮、表格、浮层等会共用的变量。

### 7.2 `variables-light.css` / `variables-dark.css`

这两个文件把“亮色 / 暗色”主题的颜色语义落地，例如：

- 背景层级
- 文字层级
- danger / success / info / warning 语义色
- 边框和阴影

因此主题切换时，本质上不是组件重新写样式，而是 token 源切换。

### 7.3 密度模式

密度切换最终也是样式变量层生效，例如表格单元的 padding 等。也就是说：

- `DensityToggle.vue` 决定模式
- 根节点 class 负责切换上下文
- 样式变量层负责让所有组件自动跟随变化

## 8. 把整个 Dashboard 连起来看

如果把 Dashboard 看成一条完整链路，大致是：

1. `main.ts` 建立运行时和主题基础
2. `App.vue` 决定当前工作台、全局命令和全局反馈
3. 业务面板通过 `api.ts` 调后端，通过 `types.ts` 约束数据
4. 公共组件提供导航、反馈、主题、密度、按钮、骨架等通用能力
5. 样式变量层保证全局视觉一致

所以 Dashboard 的稳定性，并不只来自业务组件本身，而是来自这套“壳层 + 契约层 + 公共层 + 样式层”的分工。

## 9. 推荐阅读路径

如果你已经看完 `Dashboard-Components.md`，建议继续这样读：

1. `dashboard-ui/src/main.ts`：先建立启动与主题装配模型
2. `dashboard-ui/src/App.vue`：再理解壳层如何承接所有工作台
3. `dashboard-ui/src/types.ts`：看前端到底和后端约定了哪些数据结构
4. `dashboard-ui/src/api.ts`：看这些约定如何转成请求能力
5. `dashboard-ui/src/components/CapsuleTabs.vue`、`CommandPalette.vue`、`GlobalFeedback.vue`：看全局交互骨架
6. `dashboard-ui/src/ui/feedback.ts`、`styles/variable.css`：最后补齐反馈机制和样式 token 系统
7. 如果你要按组件文档继续读壳层公共交互，建议按 `shared/Dashboard-Capsule-Tabs.md` → `shared/Dashboard-Command-Palette.md` → `shared/Dashboard-Density-Toggle.md` → `shared/Dashboard-Theme-Toggle.md` → `shared/Dashboard-Global-Feedback.md` → `shared/Dashboard-Feedback-Store.md` → `shared/Dashboard-Export-Utils.md` → `shared/Dashboard-Tag-Utils.md` → `shared/Dashboard-Skeleton-Table.md` → `shared/Dashboard-Button.md` 的顺序读

这样再回头读具体业务面板，就不会只停留在“这个组件会渲染什么”，而能理解它为什么会以现在这种方式被组织起来。


