# Dashboard EnvTemplateRunPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/EnvTemplateRunPanel.vue`。

`EnvTemplateRunPanel` 是 Env 工作台里最像“工具箱”的一块。它把三类运行态能力压在了一张卡片里：

- 模板展开 / 校验
- 导出生效中的环境
- 带环境上下文执行命令

所以它不是单一职责的小组件，而是围绕“运行时 Env 工具”组织起来的一块综合面板。

## 1. 组件定位

这个组件主要解决三类场景：

- 我想验证一个模板里引用了哪些变量
- 我想把当前生效环境导出成某种格式
- 我想在这个环境上下文里跑一条命令并看结果

因此它更像一个运行态运维工具箱，而不是普通配置面板。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `scope: EnvScope`
- `templateResult: EnvTemplateResult | null`
- `runResult: EnvRunResult | null`
- `loading?: boolean`

这里对应两类回显：

- 模板展开/校验结果
- 命令执行结果

### 2.2 Emits

它向外抛出：

- `template-expand`
- `export-live`
- `run`

其中 payload 分别携带：

- 模板文本与 `validate_only`
- 导出格式
- 命令 token、schema 检查、通知和输出上限

也就是说，这个组件虽然能力多，但真正执行逻辑仍然都在父层。

## 3. 本地状态与核心逻辑

### 3.1 模板相关状态

- `templateInput`
- `validateOnly`
- `exportFormat`

这三项分别对应：

- 模板源文本
- 是否只做校验
- 导出格式选择

### 3.2 命令运行相关状态

- `commandTokens`
- `schemaCheck`
- `notify`
- `maxOutput`

这里可以看出它不是把命令当单行字符串传递，而是要求：

- 一行一个 token

这让前端更容易稳定地构造 `cmd: string[]`。

### 3.3 `normalizedScope()`

组件内部专门定义了 `normalizedScope()`：

- 如果 `scope === 'all'`，就收敛成 `user`

这是因为模板展开、导出生效环境和命令执行都更接近“明确上下文下的操作”，不适合停留在模糊的 `all` 语义里。

### 3.4 `onExpandTemplate()`

它会：

1. trim 模板文本
2. 空值直接返回
3. emit `template-expand`

### 3.5 `onRunCommand()`

命令执行前会：

1. 把 textarea 按行拆成 token 数组
2. 去掉空白 token
3. 空数组直接返回
4. 把 `maxOutput` 收敛到 `1024 ~ 1024 * 1024`
5. emit `run`

这个上限收敛是很关键的保护措施，避免前端把不合理的输出额度直接交给后端。

## 4. 模板结构

### 4.1 模板展开区

第一块包含：

- 模板输入框
- `validate only`
- `Expand`

如果 `templateResult` 存在，还会展示：

- `valid`
- `refs`
- `missing`
- `cycles`
- 可选 `output`

### 4.2 Export Live 区

第二块是导出工具条：

- `dotenv`
- `sh`
- `json`
- `reg`
- `Export Live`

这块只提供动作入口，不在组件内显示导出结果，说明结果处理更偏下载流或父层反馈。

### 4.3 命令输入区

命令区由两部分组成：

- token textarea
- 运行参数工具条

参数包括：

- `schema check`
- `notify`
- `max output`
- `Run`

### 4.4 执行结果区

如果有 `runResult`，会展示：

- `exit`
- `success`
- `truncated`
- `stdout`
- `stderr`

值得注意的是，类型里有 `command_line`，但组件并没有把它渲染出来。也就是说，它更关注执行结果，而不是完整命令回显。

## 5. 架构意义

`EnvTemplateRunPanel` 的最大特点，不是“功能纯”，而是“把一组运行态工具集中在一起”。

这种设计有两个明显效果：

- 用户在同一块区域里完成模板验证、环境导出和命令执行
- 容器层可以统一接管这些较重的操作型 API

从职责上看，它仍然守住了边界：**视图收集参数，业务执行交给父层。**

## 6. 组件特征总结

一句话概括 `EnvTemplateRunPanel.vue`：

- **它是 Env 工作台里的运行态工具箱，把模板、导出和命令执行三类能力集中到了一张卡片里。**

最关键的观察点有四个：

- `all` 会被收敛成 `user`
- 命令输入采用“一行一个 token”模型
- `maxOutput` 有前端收敛保护
- 结果回显只覆盖模板与运行结果，不覆盖导出结果

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `props + emits`，确认这是三合一工具面板
2. 再看 `normalizedScope()`、`onExpandTemplate()`、`onRunCommand()`
3. 最后看模板里的三段结构：Expand、Export Live、Run

读完后再回 `EnvPanel`，去看这些动作分别接到了哪些 API。
