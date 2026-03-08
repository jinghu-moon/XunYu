# Dashboard Redirect 面板导读

这篇文档专门拆 `dashboard-ui/src/components/RedirectPanel.vue`。

`RedirectPanel` 是当前 Dashboard 里最像“规则编辑器”的组件之一。它同时处理 profile 管理、规则草稿编辑、规则排序、保存前校验、未保存离开保护，以及 dry-run 预演。

如果说 `ConfigPanel` 是一个窄面板，那 `RedirectPanel` 更像一个**规则工作台**。

## 1. 数据契约与接口边界

## 1.1 数据模型

它依赖的核心类型有：

- `RedirectConfig`
  - `profiles: Record<string, RedirectProfile>`
- `RedirectProfile`
  - `rules`
  - `unmatched`
  - `on_conflict`
  - `recursive?`
  - `max_depth?`
- `RedirectRule`
  - `name`
  - `match`
  - `dest`
- `MatchCondition`
  - `ext`
  - `glob?`
  - `regex?`
  - `size?`
  - `age?`
- `RedirectDryRunResponse`
  - `results`
  - `stats`

从这些类型可以看出，这个面板同时覆盖两类对象：

- **配置对象**：profile / rule / match 条件
- **执行结果对象**：dry-run 统计与明细

## 1.2 API 边界

该面板对接 4 个接口：

- `fetchRedirectProfiles()`：读取全部 profile
- `upsertRedirectProfile(name, profile)`：保存或覆盖 profile
- `deleteRedirectProfile(name)`：删除 profile
- `redirectDryRun({ source, profile, copy })`：以当前草稿做预演

这里最重要的一个设计点是：**dry-run 可以直接吃当前草稿生成的 profile，而不要求先保存。**

这让规则编辑和预演形成了闭环。

## 2. 本地草稿模型

这个组件内部额外定义了一个前端专用类型 `RuleDraft`：

- `name`
- `extCsv`
- `glob`
- `regex`
- `size`
- `age`
- `dest`

它和后端的 `RedirectRule` 并不完全相同。

这样做的原因很明确：

- 后端模型里 `ext` 是数组
- 前端编辑时更适合用 CSV 字符串
- 某些字段需要保留空字符串，而不是立刻转成 `null`

因此 `RuleDraft` 才是这个面板真正的编辑模型，`RedirectProfile` 是保存模型。

## 3. 状态模型

这块状态比较多，但分层很清楚。

## 3.1 配置与草稿状态

- `cfg`
- `profileName`
- `ruleDrafts`
- `unmatchedMode`
- `archiveAge`
- `archiveDest`
- `onConflict`
- `recursive`
- `maxDepth`
- `newProfileName`

这些状态共同组成“当前正在编辑的 profile 草稿”。

## 3.2 运行与反馈状态

- `lastError`
- `dryError`
- `drySource`
- `dryCopy`
- `dryBusy`
- `dryResp`

这组状态服务于 dry-run 预演和错误反馈。

## 3.3 保存快照与 dirty 检测

- `savedSnapshot`
- `savedProfile`
- `lastProfileName`

这三个状态是整个组件最关键的基础设施之一。它们用来判断：

- 当前草稿是否已经偏离已保存状态
- 切换 profile 时是否要阻止误丢失
- 某一行规则是否已修改

## 3.4 危险动作与拖拽状态

- `deleteProfileBusy`
- `confirmKey`
- `confirmRemaining`
- `confirmTimer`
- `dragIndex`

它们分别服务于：

- 删除 profile / 删除 rule 的轻确认
- 规则拖拽排序

## 4. 草稿与保存模型的双向转换

这块是 `RedirectPanel` 最核心的实现。

## 4.1 `normalizeCsv(csv)`

把 CSV 文本拆成数组，主要服务于 `extCsv -> ext[]` 转换。

## 4.2 `profileToDraft(p)`

这个函数负责把后端 profile 转成前端草稿状态：

- 解析 `unmatched`
  - `skip`
  - 或 `archive:age:dest`
- 回填 `on_conflict`
- 回填 `recursive` / `max_depth`
- 把每条 rule 转成 `RuleDraft`
  - `ext[]` 变成 `extCsv`
  - `glob/regex/size/age` 空值转为空字符串

因此它是真正的“读模型 -> 编辑模型”转换器。

## 4.3 `draftToProfile()`

这个函数负责把当前草稿转回后端可保存模型：

- `unmatchedMode === 'skip'` 时输出 `skip`
- `archive` 模式时输出 `archive:${age}:${dest}`
- 每条 rule 的 `extCsv` 转成数组
- 空字符串字段转成 `null`
- `maxDepth` 被规范成最小为 1

因此它是真正的“编辑模型 -> 保存模型”转换器。

## 5. dirty 检测与保存快照

## 5.1 `markSavedSnapshot()`

保存成功或加载成功后，会把当前 profile 序列化成字符串快照，同时也保存一份结构化 `savedProfile`。

## 5.2 `isDirty`

当前草稿与 `savedSnapshot` 的 JSON 串比较不一致时，就认为页面有未保存修改。

## 5.3 局部变化判断

组件还进一步拆出了：

- `isUnmatchedChanged`
- `isOnConflictChanged`
- `isRecursiveChanged`
- `isMaxDepthChanged`
- `isRuleChanged(idx)`

这让模板能把具体变更字段和具体变更规则高亮出来，而不是只给一个笼统的“已修改”。

## 6. 核心工作流

## 6.1 加载

`load(force = false)` 做的事情很多：

1. 若当前 dirty 且未强制加载，先询问是否丢弃修改
2. `fetchRedirectProfiles()` 拉全部 profile
3. 如果当前 `profileName` 不存在，就回退到第一个可用 profile 或 `default`
4. 若目标 profile 存在，则 `profileToDraft(p)` 回填草稿
5. 否则重置为默认空草稿
6. 调 `markSavedSnapshot()`
7. 更新 `lastProfileName`

因此它不是简单请求函数，而是整个编辑器的“重新对齐入口”。

## 6.2 保存

`onSave()` 的流程很直接：

- `upsertRedirectProfile(profileName, draftToProfile())`
- `load(true)` 重新同步

这说明最终真相仍以后端保存结果为准，前端不会只相信自己的本地草稿。

## 6.3 删除 profile

`onDeleteProfile()` 的保护层次很多：

1. 如果当前有未保存修改，先确认是否丢弃
2. 删除按钮采用 3 秒确认窗
3. 真删后把 `profileName` 重置到 `default`
4. 再 `load(true)`

## 6.4 新建 profile

`onCreateProfile()` 会：

- 先检查 dirty 丢弃确认
- 校验 `newProfileName`
- 如果同名已存在，直接 `alert`
- 否则创建一份默认 profile

这里的新 profile 不是空白完全体，而是带一条默认 `Images` 规则的最小可用模板。这降低了首次编辑门槛。

## 6.5 切换 profile

`onProfileChange()` 会在切换前检查 dirty，必要时回滚到 `lastProfileName`，防止误切换丢数据。

## 6.6 规则增删与排序

- `onAddRule()`：追加一条空草稿规则
- `onDeleteRule(idx)`：3 秒确认后删除该规则
- `onDragStart()` / `onDrop()`：通过 `dragIndex` 实现前端重排

所以这个表格不是静态表单，而是具备列表编辑器特征。

## 7. 保存前校验

`validationErrors` 会统一收集以下问题：

- profile 名为空
- archive 模式下 `archiveDest` / `archiveAge` 为空
- recursive 模式下 `maxDepth < 1`
- 某条 rule 的 match 条件全空
- 某条 rule 的 `dest` 为空

另外 `isRuleInvalid(r)` 会专门判断单条规则是否完全没有 match 条件，用于给输入框打 invalid 样式。

这说明校验策略分两层：

- 顶层错误列表：阻止保存
- 行级 invalid 高亮：帮助定位问题

## 8. 未保存离开保护

组件会 `watch(isDirty, ...)`：

- dirty 时注册 `beforeunload`
- 干净时移除 `beforeunload`

这意味着：

- 切换页面 / 刷新浏览器前会被提醒
- 不是所有时候都长期挂着监听器

这是一个很标准、也很克制的编辑器保护策略。

## 9. Dry run：这个面板的第二条主线

## 9.1 输入与执行

`runDryRun()` 的流程是：

1. 清空 `dryError`
2. 校验 `drySource` 非空
3. 调 `redirectDryRun({ source, profile: currentProfile, copy: dryCopy })`
4. 把结果写入 `dryResp`

关键点在于：它用的是 `currentProfile.value`，也就是**当前草稿态 profile**。

## 9.2 结果展示

Dry run 区会展示：

- `stats.total`
- `stats.dry_run`
- `stats.skipped`
- `stats.failed`
- 每条结果的：
  - `result`
  - `action`
  - `rule`
  - `src`
  - `dst`
  - `reason`

同时 `dryRowClass(result)` 会给不同结果上不同背景色：

- `failed`
- `skipped`
- `dry_run`

所以这块并不是“调一个 API 然后 dump JSON”，而是已经形成了可审阅的预演工作台。

## 10. 视图结构

## 10.1 顶部 profile 工具条

这一层包括：

- 当前 profile 选择器
- Reload
- Save
- Delete profile
- New profile name
- Create

这是 profile 级管理区。

## 10.2 状态提示区

紧接着是几个提示区：

- `Unsaved changes`
- validation 列表
- `lastError`
- `dryError`

也就是说，这个组件会把“草稿状态”“校验状态”“执行错误状态”明确拆开，而不是混成一个统一错误框。

## 10.3 Profile 参数区

这部分负责：

- `unmatchedMode`
- archive 的 `age` / `dest`
- `onConflict`
- `recursive`
- `maxDepth`
- `Add rule`

这是规则表上方的 profile 级参数控制区。

## 10.4 Dry run 区

这部分负责：

- 输入 source path
- 选择 copy 模式
- 执行 Run
- 看 dry-run 统计和明细表

## 10.5 规则表

规则表是整个组件的主体，包括：

- 拖拽排序把手
- 规则名
- `Ext (csv)`
- `Glob`
- `Regex`
- `Size`
- `Age`
- `Dest`
- 删除按钮

每行还能根据：

- `isRuleChanged(idx)`
- `isRuleInvalid(r)`

显示“已改动”和“无效”状态。

## 11. 组件特征总结

`RedirectPanel.vue` 的核心特点有 5 个：

1. **双模型编辑**：前端用 `RuleDraft` 编辑，保存时再转成 `RedirectProfile`。
2. **草稿感知**：内建 dirty 检测、字段级变化高亮、beforeunload 保护。
3. **规则治理**：支持 profile、新建、删除、规则增删和重排。
4. **保存前校验**：统一错误列表 + 行级 invalid 提示。
5. **预演闭环**：当前草稿可直接 dry-run，不必先保存。

## 12. 推荐阅读顺序

建议这样读这份组件源码：

1. 先看 `RuleDraft`、`profileToDraft()`、`draftToProfile()`
2. 再看 `savedSnapshot` / `savedProfile` / `isDirty`
3. 然后看 `load()`、`onSave()`、`onCreateProfile()`、`onProfileChange()`
4. 最后看 `validationErrors` 和 `runDryRun()`

这样你会更容易理解：`RedirectPanel` 的本质不是一张表，而是一个围绕 profile 草稿进行编辑、校验、预演和保存的规则工作台。
