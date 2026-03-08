# Dashboard Audit 面板导读

这篇文档专门拆 `dashboard-ui/src/components/AuditPanel.vue`。

`AuditPanel` 的定位很明确：它不是审计写入器，而是**审计检索与审阅工作台**。它把检索、筛选、分页、导出和单条详情查看组织到了一处。

## 1. 组件定位

这个组件主要解决 5 件事：

- 拉取审计记录
- 按关键字、动作、结果筛选
- 分页浏览当前结果集
- 导出当前结果集
- 查看单条记录详情

也就是说，它的重点不在“怎么产生审计”，而在“怎么消费审计”。

## 2. 数据契约与接口边界

## 2.1 依赖的数据类型

它依赖两类核心类型：

- `AuditEntry`
  - `timestamp`
  - `action`
  - `target`
  - `user`
  - `params`
  - `result`
  - `reason`
- `AuditResponse`
  - `entries`
  - `stats`
  - `next_cursor?`

其中 `stats` 又包含：

- `total`
- `by_action`
- `by_result`

所以这个面板不是只拿一组平铺日志，它还能拿到聚合统计。

## 2.2 API 边界

当前组件只直接调用一个接口：

- `fetchAudit({ limit, search, action, result })`

值得注意的是，`api.ts` 里虽然还有 `exportAuditCsv(...)`，但这个面板并没有直接使用它，而是用前端当前结果集自行导出 CSV / JSON。

这说明当前导出语义是：**导出当前 UI 结果集，而不是把导出责任完全交给后端。**

## 3. 状态模型

状态可以分成 4 组。

## 3.1 筛选状态

- `search`
- `action`
- `result`

这三者共同决定当前查询条件。

## 3.2 数据状态

- `resp`
- `entries`（computed）
- `actionItems`（computed）
- `resultItems`（computed）

`resp` 是原始响应，`entries` 和筛选下拉项则是从响应中派生出来的消费层数据。

## 3.3 分页状态

- `page`
- `pageSize`
- `pageCount`
- `pageStart`
- `pageEnd`
- `pagedEntries`

这里的分页是**前端分页**，不是游标分页。组件会先拉一批匹配结果，再在本地做页切分。

## 3.4 交互状态

- `busy`
- `isLoading`
- `detailEntry`

`detailEntry` 为非空时，详情模态层打开。

## 4. 核心工作流

## 4.1 加载

`load()` 的流程很简单：

1. `busy = true`
2. 调 `fetchAudit(...)`
3. 参数里带上：
   - `limit: 400`
   - `search`
   - `action`
   - `result`
4. 请求完成后把 `page` 重置到 1

这说明组件的默认策略是：

- 一次拉一批足够大的匹配记录
- 切筛选条件后自动回到第一页
- 后续翻页尽量在本地完成

## 4.2 分页

分页逻辑由几个轻量函数与 `computed` 完成：

- `nextPage()`
- `prevPage()`
- `pageCount`
- `pagedEntries`

并且 `watch([entries, pageSize], ...)` 会在结果集变短或每页条数变化时，自动把 `page` 修正到合法范围。

这是一种很稳的本地分页策略。

## 4.3 打开 / 关闭详情

- `openDetail(e)`：把当前条目写入 `detailEntry`
- `closeDetail()`：清空 `detailEntry`

因此详情模态其实是一个非常纯粹的“当前选中记录”投影视图。

## 4.4 Params 格式化

`formatPayload(raw)` 会对 `params` 做轻量美化：

- 空值显示 `-`
- 如果看起来像 JSON（以 `{` 或 `[` 开头），就尝试格式化缩进输出
- 失败则退回原始文本

因此详情区不会只把 `params` 当一段难读的原始字符串直接贴出来。

## 4.5 导出

`exportAudit(format)` 导出的是 `entries.value`，也就是**当前过滤结果集的全集**，而不是当前页。

支持：

- JSON：`downloadJson('audit', items)`
- CSV：`downloadCsv('audit', ...)`

这点很重要：

- 搜索和筛选会影响导出范围
- 分页不会影响导出范围

## 5. 视图结构

## 5.1 顶部工具条

工具条包括：

- 搜索框
- 动作过滤下拉框
- 结果过滤下拉框
- `Refresh`
- 导出按钮组

交互上也很统一：

- 搜索框回车触发 `load`
- 下拉切换立即触发 `load`
- 点击 `Refresh` 重新拉取

## 5.2 统计卡片区

工具条下方有一排统计卡：

- Matched
- Redirect moves
- Redirect copies
- Redirect skips

这说明这个面板当前特别关心 redirect 相关动作，而不是仅仅平铺所有 action。

也可以把它看成：AuditPanel 对 redirect 治理链路做了特化观察。

## 5.3 主表格区

主表格列包括：

- Time
- Action
- Target
- Result
- Reason
- Details 按钮

其中：

- `failed` 记录整行会加失败背景
- 结果列也会按 `success / failed / other` 使用不同语义色

因此“失败项”在列表里会非常醒目。

## 5.4 分页区

底部分页区显示：

- 当前展示范围 `Showing x-y of n`
- 行数选择 `20 / 50 / 100`
- Prev / Next
- 当前页码

这让长结果集在不引入后端分页复杂度的前提下仍可读。

## 5.5 详情模态区

点击 `Details` 后，会打开模态层，展示：

- 时间
- 动作
- 结果
- 目标
- 用户
- 原因
- Params（格式化预览）

所以主表格负责“扫”，详情模态负责“查”。

## 6. 组件特征总结

`AuditPanel.vue` 的设计重点可以概括为：

1. **前端检索台**：一批拉取 + 本地分页。
2. **过滤结果导出**：导出当前筛选全集，不受当前页限制。
3. **失败可视化**：失败记录在表格里被明显强化。
4. **详情解压缩**：列表简洁，详情模态负责展开结构化信息。

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `load()`、分页相关 `computed`
2. 再看 `formatPayload()` 和 `exportAudit()`
3. 最后看模板里的三块：工具条、统计卡、详情模态

读完之后你会更容易把它理解成一个“审计审阅面板”，而不是普通日志表格。
