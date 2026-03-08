# Dashboard Config 面板导读

这篇文档专门拆 `dashboard-ui/src/components/ConfigPanel.vue`。

这个组件看起来很小，但它在 Dashboard 架构里很有代表性：**它不是通用配置编辑器，而是一个刻意收敛范围的窄面板。**

## 1. 组件定位

从 `types.ts` 看，`GlobalConfig` 至少包含：

- `tree`
- `proxy`
- `protect?`
- `redirect?`

但 `ConfigPanel.vue` 当前只编辑其中的 `tree` 配置，具体只覆盖：

- `tree.defaultDepth`
- `tree.excludeNames`

这意味着它的职责非常克制：

- 不试图覆盖全部全局配置
- 不做 JSON 原始编辑器
- 不做多模块混编配置台

它更像一个 **Tree 行为偏好编辑器**。

## 2. 数据契约与接口边界

## 2.1 数据模型

这个面板依赖的核心类型是：

- `TreeConfig`
  - `defaultDepth?: number | null`
  - `excludeNames?: string[]`
- `GlobalConfig`
  - `tree: TreeConfig`
  - 以及其他暂未在本面板暴露的配置域

## 2.2 API 边界

这个组件只用了两个接口：

- `fetchConfig()`：加载当前配置
- `patchConfig(patch)`：局部更新配置

值得注意的是，`api.ts` 里其实还提供了 `replaceConfig(cfg)`，但这个面板没有使用它。

这说明当前设计是：

- 面板只做局部 patch
- 不承担整份配置替换责任
- 尽量减少误改别的配置域的风险

## 3. 状态模型

状态非常少，这也是它“窄面板”定位的体现：

- `cfg`：后端返回的当前配置快照
- `depthInput`：深度输入框的字符串值
- `excludeInput`：排除名输入框的字符串值
- `busy`：加载 / 保存忙碌状态

另外还有两个只读派生：

- `currentDepth`
- `currentExclude`

它们用于顶部摘要卡，而不是作为提交源。

## 4. 输入归一化与校验

这个组件的主要逻辑，不在复杂副作用里，而在输入归一化上。

## 4.1 `normalizeExclude(raw)`

它把逗号分隔的字符串处理成字符串数组：

- 按 `,` 切分
- `trim()`
- 过滤空项

也就是说，UI 里是一个简单 CSV 输入框，提交时再变成结构化数组。

## 4.2 `parseDepth(raw)`

这个函数把深度输入解析为：

- `null`：空字符串，表示清空 / 不限制
- `number`：合法的非负整数
- `'invalid'`：非法输入

这层校验很关键，因为它明确拒绝：

- 负数
- 非整数
- 非数值

## 4.3 `syncForm(data)`

这个函数负责把后端配置反向同步回表单：

- `defaultDepth` 转成字符串
- `excludeNames` 转成逗号分隔文本

这让表单和配置快照保持同构，而不会出现保存后 UI 不更新的问题。

## 5. 核心工作流

## 5.1 加载

`load()` 的流程非常简单：

1. `busy = true`
2. `fetchConfig()` 拉当前配置
3. 更新 `cfg`
4. `syncForm(data)` 回填表单
5. `busy = false`

这个组件没有额外缓存、没有局部分支加载，说明它默认配置体量很小，允许整体拉取。

## 5.2 保存

`onSave()` 的流程也很清晰：

1. 先 `parseDepth(depthInput)`
2. 如果非法，直接 `notifyError(..., 'Config')`
3. 再 `normalizeExclude(excludeInput)`
4. 调 `patchConfig({ tree: { defaultDepth, excludeNames } })`
5. 用服务端返回值更新 `cfg`
6. 再次 `syncForm(data)`

这里有两个值得注意的点：

- 它提交的是 **结构化 patch**，不是拼字符串配置
- 它总是以服务端返回结果为准，避免前端保留脏状态

## 6. 视图结构

这个组件的模板结构非常克制，基本只有三块。

## 6.1 顶部工具条

顶部只有两个动作：

- `Refresh`
- `Save config`

没有重置、导入导出、原始 JSON 编辑等复杂操作，进一步证明它是一个窄面板。

## 6.2 摘要区

`config-summary` 里有两张摘要卡：

- 当前 `defaultDepth`
- 当前 `excludeNames`

它们的作用是把“已生效配置”与“正在编辑的输入框”分开显示，减少误解。

其中：

- 空的 `defaultDepth` 会被解释为 unlimited depth
- `excludeNames` 会展示当前条目数

## 6.3 编辑表单

表单只有两个字段：

- `Default depth`
- `Exclude names (csv)`

输入形态也很朴素：

- 数字输入框
- 文本输入框

这符合 KISS：它没有为了少量配置引入复杂表单系统。

## 7. 组件特征总结

`ConfigPanel.vue` 的最大特点不是“功能多”，而是 **边界明确**。

它做了三件正确的事：

1. 只暴露当前真正高价值、低歧义的配置项。
2. 用 `patchConfig()` 而不是 `replaceConfig()`，避免误伤其他配置域。
3. 把表单输入和配置快照分开，用摘要区明确当前生效值。

从工程角度看，这种小面板比“大而全配置中心”更稳，因为：

- 输入校验更容易做对
- 风险面更小
- 改动范围更可控
- 后续扩展也更容易按块追加

## 8. 当前实现上的观察

这块代码有一个很明确的信号：**Dashboard 并不想把所有 CLI / 后端配置都一股脑搬进 Web UI。**

`ConfigPanel` 当前只覆盖 tree 相关偏好，说明项目对 Dashboard 的定位是：

- 先覆盖高频、低风险、可视化收益明显的配置
- 而不是立刻把它做成“所有配置的一站式入口”

这和 `../Dashboard-Foundation.md` 里提到的壳层思路是吻合的：前端工作台按能力块渐进生长，而不是一次吞下全部系统复杂度。

## 9. 推荐阅读顺序

这个面板很小，建议直接这样读：

1. 先看 `GlobalConfig` 和 `TreeConfig` 的类型定义
2. 再看 `normalizeExclude`、`parseDepth`、`syncForm`
3. 最后看 `load` 和 `onSave`

读完之后你会很清楚：这不是一个“配置很多所以简单写了两个输入框”的组件，而是一个 **有意保持窄边界** 的组件。
