# Dashboard EnvSchemaPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/EnvSchemaPanel.vue`。

`EnvSchemaPanel` 是 Env 工作台里最偏“规则治理”的那个组件。它不直接修改环境变量值，而是负责：

- 管理 env schema 规则
- 触发 validate
- 展示校验结果和违规明细

所以它不是底层 schema 引擎，也不是复杂规则编辑器，而是一个把**规则维护 + 校验入口 + 结果查看**组合在一起的治理面板。

## 1. 组件定位

如果说 `EnvVarsTable` 关心“当前有哪些变量”，那么 `EnvSchemaPanel` 关心的是：

- 哪些变量模式必须存在
- 哪些变量必须匹配正则
- 哪些变量只能从枚举值里选
- 当前 scope 下这些规则是否被满足

它面向的不是单条变量编辑，而是整个 Env 域的约束治理。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `schema`
- `validation`
- `scope`
- `loading?`

这意味着组件同时消费两类数据：

- 静态规则集 `schema`
- 某次 validate 之后的动态结果 `validation`

但这两类数据都不是它自己生成的，而是由父容器统一拉取与维护。

### 2.2 Emits

它向外抛出：

- `refresh-schema`
- `add-required`
- `add-regex`
- `add-enum`
- `remove-rule`
- `reset-schema`
- `run-validate`

也就是说：

- 它自己不执行业务规则落库
- 它只负责把“想新增什么规则”“想删除什么规则”“想跑什么校验”表达给父层

这与 Env 其它子组件的设计是一致的：**视图层发意图，容器层执行业务。**

## 3. 本地状态与建模思路

组件内部维护的本地状态有：

- `ruleType`
- `pattern`
- `regex`
- `enumValues`
- `warnOnly`
- `strict`

这里最值得注意的是它的建模方式。

### 3.1 单入口规则创建

它没有给 `required / regex / enum` 各写一套独立表单，而是采用一套共享输入区：

- 先选 `ruleType`
- 再输入 `pattern`
- 然后根据类型补充 `regex` 或 `enumValues`
- 最后决定 `warnOnly`

这使得规则创建保持在一个紧凑面板里，没有被拆成多个平行小模块。

### 3.2 `strict` 属于 validate 上下文，而不是规则本身

`strict` 不参与规则创建，而是参与 `run-validate`。这说明该组件明确区分了两类配置：

- 规则定义层
- 校验执行层

这个区分非常关键，因为它让“规则长什么样”和“这次怎么跑校验”不会混成一件事。

## 4. 核心逻辑

### 4.1 `schemaRules`

组件会把 `props.schema?.rules ?? []` 收口成 `schemaRules`。

这个细节让模板层不必反复判断空值，也说明 schema 为空时，组件仍然可以用“空规则集”方式稳定渲染。

### 4.2 `onAddRule()`

新增规则的核心都在这个分发函数里。

它的行为是：

- 如果 `ruleType === 'required'`，就 emit `add-required`
- 如果 `ruleType === 'regex'`，就 emit `add-regex`
- 如果 `ruleType === 'enum'`，就把 CSV 文本拆成数组，再 emit `add-enum`

也就是说，这个组件采用的是：

- 一个统一入口
- 多分支分发
- 不同规则类型共用同一块 UI

这比拆成多个表单更省空间，也更符合治理面板的工作台气质。

### 4.3 `onValidate()`

校验触发逻辑很简单：

- 读取当前 `scope`
- 读取当前 `strict`
- emit `run-validate({ scope, strict })`

组件不自己解释校验结果，只负责发起校验。

## 5. 模板结构

模板大致可以拆成 4 块。

### 5.1 头部区

包含：

- 标题 `Schema`
- `Refresh` 按钮
- `Reset` 按钮

这块表达的是规则集维度的运维动作。

### 5.2 规则创建区

这里是一条比较紧凑的工具条，包含：

- `ruleType`
- `pattern`
- 条件字段 `regex / enumValues`
- `warnOnly`
- `Add Rule`

重点不在控件多，而在它把三类规则压进了同一个入口。

### 5.3 规则表

规则列表会展示：

- `Pattern`
- `Required`
- `Regex`
- `Enum`
- `Warn`
- `Action`

这张表的作用不是做高级编辑，而是让当前规则集可以被快速浏览和删除。

### 5.4 Validate 区与违规表

Validate 区里有：

- `strict`
- `Run Validate`
- 汇总信息

校验完成后，违规表再展示：

- `Name`
- `Pattern`
- `Kind`
- `Severity`
- `Message`

这意味着 `EnvSchemaPanel` 并不是“配完规则就结束”，而是把规则治理的闭环补完到了结果反馈层。

## 6. 架构意义

`EnvSchemaPanel` 的价值不只是“能加规则”，而是它把规则治理拆成了两个清楚的层次：

- 规则维护
- 规则验证

并且这两层都不自己落地业务，而是通过事件回到 `EnvPanel`。因此整个架构关系是：

- `EnvSchemaPanel` 负责规则视图与交互收集
- `EnvPanel` 负责 API 调用、状态刷新和结果回填
- `api.ts` 与后端负责真正的 schema 存储和校验执行

这使得组件边界比较稳，不会因为规则类型增加而轻易膨胀成难维护的大组件。

## 7. 组件特征总结

一句话概括 `EnvSchemaPanel.vue`：

- **它是 Env 工作台中的规则治理面板，用单入口多分支方式管理规则，并提供 validate 入口与结果回显。**

理解它时，最值得注意的是三点：

- 规则新增并不是多个表单并排，而是统一入口分发
- `strict` 是校验上下文，不是规则字段
- 组件负责治理交互，不负责治理执行

## 8. 推荐阅读顺序

建议按这个顺序读：

1. 先看 `props + emits`，确认规则数据和校验结果都来自父层
2. 再看 `ruleType/pattern/regex/enumValues/warnOnly/strict` 这些本地状态
3. 接着看 `onAddRule()` 和 `onValidate()`，理解它的单入口分发设计
4. 最后看模板，把规则创建区、规则表、Validate 区和违规表串起来

读完后回到 `Dashboard-Env-Panel.md`，你会更容易理解 Env 容器为什么要同时维护 `schema` 和 `validation` 两类状态。


