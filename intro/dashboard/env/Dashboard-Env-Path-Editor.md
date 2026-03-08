# Dashboard EnvPathEditor 导读

这篇文档专门拆 `dashboard-ui/src/components/EnvPathEditor.vue`。

`EnvPathEditor` 是 Env 工作台里最专用的一个编辑器，它只关心 `PATH` 变量拆分后的条目列表，不关心普通环境变量，也不承担复杂的排序与校验能力。

所以它的定位不是“通用变量编辑器”，而是一个围绕 `PATH` 片段增删的轻量操作面板。

## 1. 组件定位

这个组件解决的是一件非常具体的事：

- 展示当前作用域下的 PATH 条目
- 增加一个新条目
- 删除一个已有条目
- 允许选择插到头部还是尾部

它本质上是 `EnvVarsTable` 之外的一个专项编辑器，因为 `PATH` 这种值天然适合按“列表项”而不是整串文本来操作。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `entries: string[]`
- `scope: EnvScope`
- `loading?: boolean`

这里的 `entries` 不是组件自己算出来的，而是 `EnvPanel` 从变量主数据里提取出来的 `pathEntries`。

### 2.2 Emits

它向外抛出：

- `scope-change`
- `refresh`
- `add`
- `remove`

也就是说：

- 它自己不做 PATH 写入
- 它只表达“要加哪个条目”“要删哪个条目”“当前作用域切到哪里”
- 真正的 PATH 修改仍由容器层调用 API 落地

## 3. 本地状态与核心逻辑

组件内部状态很少：

- `entry`
- `head`

### 3.1 `entry`

用于输入新的 PATH 项。

### 3.2 `head`

用于决定新条目插入位置：

- `true` 表示插到头部
- `false` 表示走默认位置

### 3.3 `addEntry()`

新增逻辑很简单：

1. 先 trim 输入值
2. 空字符串直接返回
3. emit `add({ entry, head })`
4. 成功发出后清空输入框

这说明它只负责把“PATH 编辑意图”收集成最小 payload，不管理后续刷新。

## 4. 模板结构

### 4.1 头部区

头部包含：

- 标题 `PATH Editor`
- `scope` 选择器
- `Refresh` 按钮

这里的作用域选择只有：

- `user`
- `system`

没有 `all`。这和它的编辑型职责一致，因为 PATH 写操作本来就需要落到明确作用域。

### 4.2 新增工具条

第二块是 PATH 条目新增区，包含：

- 条目输入框
- `insert at head`
- `Add`

这块没有更多参数，也没有复杂校验，保持了很强的 KISS 风格。

### 4.3 PATH 列表区

最后是 `ul.path-list`：

- 每一项用 `code` 展示路径文本
- 每一项都带 `Remove`

也就是说，这个组件支持的编辑动作只有两种：

- 插入
- 删除

并不提供拖拽重排、批量调整或去重策略。

## 5. 架构意义

`EnvPathEditor` 的价值在于，它把 PATH 这种特殊变量从通用变量表中拆了出来。

这样做的好处是：

- PATH 可以按片段操作，而不是整体字符串重写
- 变量总表不必为了 PATH 特判变复杂
- 容器层仍然保留统一写入入口

它是一个很典型的“专项子编辑器”：只解决 PATH 这一类问题，不向外扩展。

## 6. 组件特征总结

一句话概括 `EnvPathEditor.vue`：

- **它是 Env 工作台里的 PATH 专项编辑器，只负责条目级增删与位置提示。**

最值得注意的点有三个：

- 它消费的是容器层拆好的 PATH 条目列表
- 它只支持增删，不支持复杂重排
- 它的写操作始终通过事件回到 `EnvPanel`

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `props + emits`，确认它是 PATH 专项视图，不是通用变量层
2. 再看 `entry/head/addEntry()`，理解它如何表达插入语义
3. 最后看模板里的 scope 选择器、输入条和列表区

读完后回到 `Dashboard-Env-Panel.md`，会更容易理解为什么 `EnvPanel` 要把 PATH 单独拆成一块能力。


