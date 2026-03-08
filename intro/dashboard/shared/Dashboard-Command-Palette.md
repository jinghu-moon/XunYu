# Dashboard CommandPalette 导读

这篇文档专门拆 `dashboard-ui/src/components/CommandPalette.vue`。

`CommandPalette` 是 Dashboard 的全局命令面板。它不自己定义命令，而是消费父层传入的命令列表，并负责搜索、键盘导航、执行和关闭。

因此它是一个标准的“命令执行壳”，不是命令注册中心。

## 1. 组件定位

这个组件负责：

- 以浮层方式打开命令面板
- 对命令列表做全文检索
- 维护当前高亮命令
- 支持 `Esc / ↑ / ↓ / Enter`
- 执行选中的命令后自动关闭

在 `App.vue` 中，它和 `Ctrl/Cmd + K` 这条全局快捷键链路配合使用。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `modelValue: boolean`
- `commands: CommandItem[]`

命令项结构包括：

- `id`
- `label`
- 可选 `description`
- 可选 `keywords`
- 可选 `section`
- `run()`

### 2.2 Emits

它只向外抛出：

- `update:modelValue`

也就是说：

- 面板开关由父层控制
- 命令执行逻辑也不在这里生成
- 它只负责“选中谁、执行谁、何时关闭”

## 3. 本地状态与派生逻辑

组件内部有两个核心状态：

- `query`
- `activeIndex`

以及一个输入框引用：

- `inputRef`

### 3.1 `filtered`

搜索逻辑会把查询词按空白分词，然后在以下字段中做包含匹配：

- `label`
- `description`
- `section`
- `keywords`

并且要求每个 term 都能命中，这让它更像轻量全文检索，而不是简单前缀匹配。

### 3.2 打开时重置

当 `modelValue` 变成 `true` 时，组件会：

- 清空查询词
- 把高亮索引重置为 0
- 在 `nextTick` 后自动聚焦输入框

这保证了每次打开都是一个干净状态。

### 3.3 高亮索引收敛

watch `filtered` 后，如果结果列表变短，组件会把 `activeIndex` 重新收回到有效范围。

## 4. 核心交互

### 4.1 关闭

`close()` 只做一件事：

- emit `update:modelValue(false)`

### 4.2 移动与执行

- `move(delta)` 用循环取模方式上下切换高亮项
- `execute(cmd)` 调用 `cmd.run()`，然后关闭面板

### 4.3 键盘逻辑

`onKeydown()` 支持：

- `Escape` 关闭
- `ArrowDown` 下移
- `ArrowUp` 上移
- `Enter` 执行当前高亮命令

## 5. 模板结构

模板通过 `Teleport` 挂到 `body`，结构包括：

- 背景遮罩 `cmdk-backdrop`
- 面板主体 `cmdk-panel`
- 标题与提示行
- 搜索输入框
- 命令列表
- 空态提示

命令列表中的每一项都会显示：

- 主标题 `label`
- 可选副标题 `description`
- 可选分组 `section`

## 6. 架构意义

`CommandPalette` 的边界很干净：

- 命令来源由 `App.vue` 提供
- 浮层交互由组件内部解决
- 执行结果由命令自身逻辑承担

所以它把全局快捷操作抽成了一个可以独立理解的基础设施组件。

## 7. 组件特征总结

一句话概括 `CommandPalette.vue`：

- **它是全局命令浮层的执行壳，负责搜索、导航和执行，不负责命令注册。**

## 8. 推荐阅读顺序

建议这样读：

1. 先看 `CommandItem` 结构和 `props + emits`
2. 再看 `filtered`、打开重置逻辑和高亮索引收敛
3. 接着看 `move()`、`execute()`、`onKeydown()`
4. 最后看 `Teleport` 下的浮层模板
