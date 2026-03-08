# Dashboard EnvDoctorPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/EnvDoctorPanel.vue`。

`EnvDoctorPanel` 是 Env 工作台里最“薄”的治理面板之一。它几乎不做本地状态管理，只负责：

- 触发 doctor 检查
- 触发自动 fix
- 展示检查摘要和问题列表

因此它不是诊断引擎，只是诊断流程的前端入口和结果视图。

## 1. 组件定位

这个组件的职责非常单一：

- 给用户一个 `Run` 入口
- 给用户一个 `Fix` 入口
- 把 `report` 和 `fixResult` 可视化

和 `EnvDiffPanel`、`EnvSchemaPanel` 这种内部有更多交互状态的组件不同，它几乎就是一个“动作按钮 + 结果摘要”卡片。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `scope: EnvScope`
- `report: EnvDoctorReport | null`
- `fixResult: EnvDoctorFixResult | null`
- `loading?: boolean`

其中真正被展示的主要是：

- `report.issues`
- `report.errors`
- `report.warnings`
- `report.fixable`
- `fixResult.fixed`

`scope` 会跟随容器层传入，但组件本身并不提供 scope 切换 UI。

### 2.2 Emits

它只向外抛出两个事件：

- `run`
- `fix`

这个边界非常明确：

- 组件不自己跑 doctor
- 组件不自己决定 fix 之后怎么刷新
- 所有执行逻辑都交给 `EnvPanel`

## 3. 本地状态与逻辑复杂度

这个组件没有本地 `ref`、没有 `computed`、没有 `watch`。

这意味着它本质上是一个纯展示组件：

- 输入来自父层
- 输出是两个动作事件
- 自身几乎不保存临时状态

从工程角度看，这是非常干净的一种边界：把复杂逻辑完全推到容器层，组件只做界面组织。

## 4. 模板结构

### 4.1 头部动作区

头部只有：

- 标题 `Doctor`
- `Run`
- `Fix`

没有额外参数设置，也没有确认弹窗逻辑，说明它假设更高层会决定真正的执行策略。

### 4.2 摘要区

如果有 `report`，会展示：

- `Issues`
- `Errors`
- `Warnings`
- `Fixable`

如果有 `fixResult`，还会额外展示：

- `Fixed`

这里值得注意的一点是：`fixResult.details` 并没有在这个组件里展开显示。也就是说，它更偏摘要面板，而不是 fix 详情页。

### 4.3 问题列表区

如果 `report?.issues.length` 有值，就渲染问题列表：

- 左侧是 `severity`
- 右侧是 `message`

它没有把问题按分类折叠，也没有显示更多上下文字段，整体仍然保持“够用即可”的设计。

### 4.4 空态

如果没有报告，就展示：

- `No doctor report loaded.`

## 5. 架构意义

`EnvDoctorPanel` 的价值不在于复杂 UI，而在于它把诊断能力收口成了一张极薄的治理卡片。

这种设计的好处是：

- 不会把诊断逻辑散落到视图层
- fix 后怎么刷新、是否再次 run，都可以由容器统一编排
- 组件本身几乎没有可失控状态

它体现的是一种很明确的职责分离：**DoctorPanel 只负责展示诊断流程，Doctor 逻辑完全不在这里。**

## 6. 组件特征总结

一句话概括 `EnvDoctorPanel.vue`：

- **它是一个纯动作入口 + 摘要结果视图，不是诊断逻辑本体。**

最关键的观察点有三个：

- 无本地状态
- 只有 `run/fix` 两个动作出口
- 展示摘要，不展示完整 fix 细节

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `props + emits`，确认组件几乎没有业务控制权
2. 再看模板里的摘要区与问题列表区
3. 最后回到 `EnvPanel` 看 `onRunDoctor()` 和 `onFixDoctor()` 如何编排执行

这样你会更容易理解它为什么能保持这么薄。
