# Dashboard EnvProfilesPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/EnvProfilesPanel.vue`。

`EnvProfilesPanel` 负责的是 Env 工作台里的“配置快照模板”能力：把某个作用域下的变量集合保存成 profile，并在之后做应用、删除或差异比较。

它不是完整的 profile 详情页，而是 profile 列表和动作入口面板。

## 1. 组件定位

这个组件主要承担：

- 创建 profile（capture）
- 展示已有 profile 列表
- 应用某个 profile
- 删除某个 profile
- 请求当前环境与某个 profile 的 diff

因此它更偏“profile 操作台”，不是 profile 内容浏览器。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `profiles: EnvProfileMeta[]`
- `scope: EnvScope`
- `loading?: boolean`

表格里展示到的字段包括：

- `name`
- `scope`
- `var_count`
- `created_at`

### 2.2 Emits

它向外抛出：

- `refresh`
- `capture`
- `apply`
- `delete`
- `diff`

这说明它自己不保存 profile，也不自己应用 profile，只把动作意图交给父层。

## 3. 本地状态与核心逻辑

### 3.1 `profileName`

组件内部只有一个本地输入：

- `profileName`

它用于创建新 profile。

### 3.2 `onCapture()`

创建逻辑会：

1. trim profile 名称
2. 空值直接返回
3. 如果当前 `scope === 'all'`，则收敛到 `user`
4. emit `capture({ name, scope })`

这个“all -> user”的收敛很关键，因为 capture 属于写操作语义，需要明确目标作用域。

### 3.3 `onApply(name)`

应用 profile 时也会做同样的 scope 收敛：

- `all` 会被转成 `user`

### 3.4 `onDiff(name)`

但 diff 不一样：

- 它直接把 `props.scope` 原样发出去

这意味着 profile diff 更接近分析动作，而不是写入动作。

## 4. 模板结构

### 4.1 头部区

头部包含：

- 标题 `Profiles`
- `Refresh`

### 4.2 Capture 工具条

工具条里只有：

- profile 名称输入框
- `Capture`

没有更多字段，说明 profile 创建是非常轻的操作。

### 4.3 列表区

如果没有 profile，就展示：

- `No profiles.`

否则渲染表格，列包括：

- `Name`
- `Scope`
- `Vars`
- `Created`
- `Actions`

### 4.4 行内动作

每一行有三个动作：

- `Apply`
- `Diff`
- `Delete`

这正好覆盖了 profile 生命周期里最核心的操作。

## 5. 架构意义

`EnvProfilesPanel` 的边界控制得比较好：

- profile 只是以 metadata 列表形式展示
- diff 结果并不在这个组件内部渲染
- 更详细的状态仍由 `EnvPanel` 统一维护

这点可以从容器模板里看出来：`profileDiff` 的摘要提示是在 `EnvPanel` 里显示的，而不是在 `EnvProfilesPanel` 内部。也就是说，这个组件专注于动作发起，不扩展成结果容器。

## 6. 组件特征总结

一句话概括 `EnvProfilesPanel.vue`：

- **它是 profile 的列表化操作台，负责 capture/apply/delete/diff 入口，不负责 profile 内容展示。**

最值得注意的点有三个：

- 写操作会把 `all` 收敛到 `user`
- diff 作为分析动作保留原始 scope
- profile diff 结果回显不在这个组件里

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `props + emits`，确认它只是 profile 元数据列表
2. 再看 `onCapture/onApply/onDiff` 的 scope 处理差异
3. 最后看模板里的 Capture 区和 Actions 列

读完后回到 `EnvPanel` 看 `profileDiff` 的显示位置，会更容易理解这块能力是如何分层的。
