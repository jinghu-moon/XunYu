# Dashboard EnvAnnotationsPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/EnvAnnotationsPanel.vue`。

`EnvAnnotationsPanel` 负责的是 Env 工作台里的“变量备注”能力。它不修改变量值，只给变量名附一条说明文本。

所以它不是变量编辑器，而是一个变量元数据维护面板。

## 1. 组件定位

这个组件主要做三件事：

- 查看已有 annotation 列表
- 为某个变量保存 note
- 编辑已有 note 并重新保存

它解决的问题很具体：给变量补一层人类可读的说明，而不是动变量本体。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `entries: EnvAnnotationEntry[]`
- `loading?: boolean`

这里的数据结构非常轻：

- `name`
- `note`

它没有 `scope`，说明在这个前端视图里，annotation 更像基于变量名的说明表，而不是按 scope 切换的主编辑区。

### 2.2 Emits

它向外抛出：

- `refresh`
- `set`
- `delete`

这意味着：

- 保存和删除仍由父层执行
- 组件自己不请求 annotation API
- 它只负责组织表单和列表交互

## 3. 本地状态与核心逻辑

### 3.1 本地状态

组件内部维护两个输入：

- `name`
- `note`

这是一个非常典型的“小表单 + 列表”结构。

### 3.2 `onSet()`

保存逻辑会：

1. trim `name`
2. trim `note`
3. 如果任一为空则直接返回
4. emit `set({ name, note })`

这里没有额外校验，也没有复杂模式，保持了很强的 KISS 风格。

### 3.3 `onFill(item)`

点击某行的 `Edit` 后，组件会把：

- `item.name`
- `item.note`

回填到输入框里。

这说明它并没有单独做“编辑弹窗”，而是复用顶部输入区完成编辑。

## 4. 模板结构

### 4.1 头部区

头部包含：

- 标题 `Annotations`
- `Refresh`

### 4.2 输入工具条

工具条里有：

- 变量名输入框
- 备注输入框
- `Save`

它本质上就是一块极轻量的双字段表单。

### 4.3 列表区

没有数据时显示：

- `No annotations.`

有数据时展示表格，列为：

- `Name`
- `Note`
- `Actions`

### 4.4 行内动作

每一行有两个动作：

- `Edit`
- `Delete`

`Edit` 只是填充表单，不是直接进入独立编辑模式。

## 5. 架构意义

`EnvAnnotationsPanel` 的意义在于，它把“变量说明”从环境值本体中分离出来。

这种拆分的好处是：

- 不污染变量主编辑流
- 可以单独刷新与维护说明信息
- 在视图层保持很轻的输入模型

它是一块非常纯粹的元数据治理面板。

## 6. 组件特征总结

一句话概括 `EnvAnnotationsPanel.vue`：

- **它是变量备注的轻量维护面板，用一个复用表单完成新增和编辑。**

最值得注意的点有三个：

- 没有 scope 切换语义
- 编辑复用顶部输入条，而不是另开弹窗
- 保存前只做最基本的非空校验

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `props + emits`，确认它只维护 `name/note` 对
2. 再看 `onSet()` 和 `onFill()`
3. 最后看模板里的输入条和行内 `Edit/Delete`

读完后再回 `EnvPanel` 看 `refreshAnnotations()`、`onSetAnnotation()`、`onDeleteAnnotation()` 的编排即可。
