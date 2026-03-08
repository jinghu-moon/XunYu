# Dashboard Home 面板导读

这篇文档专门拆 `dashboard-ui/src/components/HomePanel.vue`。

`HomePanel` 看起来像首页，但它不是普通导航页，而是一个**跨多个子系统的聚合概览面板**。它把书签、端口、代理和审计四块信息压缩成一个总览视图，帮助你在进入具体工作台之前先建立当前系统状态的直觉。

## 1. 组件定位

这个组件最重要的特点是：**只聚合，不编辑。**

它不承担任何复杂写操作，也不维护各子系统自己的业务流程，而是专注做三件事：

- 并行拉取多域状态
- 生成简明 KPI 和快照
- 把复杂系统压缩成几块可扫读的摘要卡和表格

所以它更接近 Dashboard 的“总览页 / 健康页”，而不是控制台。

## 2. 数据契约与接口边界

## 2.1 依赖的数据类型

`HomePanel.vue` 直接依赖的类型包括：

- `Bookmark`
- `PortsResponse`
- `ProxyItem`
- `ProxyConfig`
- `AuditResponse`

这 5 个类型刚好对应它展示的 4 个域：

- Bookmarks
- Ports
- Proxy
- Audits

## 2.2 依赖的接口

它通过 `api.ts` 并行读取这几组数据：

- `fetchBookmarks()`
- `fetchPorts()`
- `fetchProxyStatus()`
- `fetchProxyConfig()`
- `fetchAudit({ limit: 5 })`

这里最值得注意的是 `fetchAudit({ limit: 5 })`：Home 不想把审计面板搬过来，只取最近 5 条，维持总览页节奏。

## 3. 状态模型

这个组件的状态非常少，和它的“聚合而非编辑”定位一致：

- `bookmarks`
- `ports`
- `proxyItems`
- `proxyCfg`
- `audit`
- `busy`
- `hasLoaded`

这里没有任何表单状态、行内编辑状态、确认状态，说明它不是交互密集型面板。

## 4. 加载策略：一次并行拉齐四个域

`load()` 是整个组件的核心入口。

它通过 `Promise.all(...)` 并行拉取：

1. 书签列表
2. 端口快照
3. 代理状态
4. 代理配置
5. 最近审计

然后一次性写入本地状态。

这种加载策略很符合 Home 面板语义：

- 不追求细粒度逐块懒加载
- 更重视“当前系统横截面”的完整性
- 一次刷新就重新建立一版全局快照

## 5. 计算属性：把原始数据压缩成总览信息

## 5.1 加载态判断

- `isLoading = busy && !hasLoaded`

它的意义是：首次加载展示骨架屏；后续刷新则尽量保留已有内容，只禁用刷新按钮，不再把整个页面清空。

## 5.2 书签聚合

书签相关派生包括：

- `bookmarkCount`
- `bookmarkTagCount`
- `topTags`

其中 `topTags` 会统计所有书签标签出现次数，按频次倒序后只取前 3 个。也就是说，Home 不只是告诉你“有多少书签”，还试图告诉你“当前书签集合最常见的主题是什么”。

## 5.3 端口聚合

端口相关派生包括：

- `portTotal`
- `portPidCount`
- `portSnapshot`

`portSnapshot` 会把 TCP/UDP 合并后按端口排序，再截前 6 条。这说明它的目标不是完整端口表，而是一个“快速扫一眼当前监听分布”的缩略视图。

## 5.4 代理聚合

代理相关派生包括：

- `proxyDefaultUrl`
- `proxyNoProxy`
- `activeProxyItems`
- `proxyConsistency`

这里的 `proxyConsistency` 和单独的 `ProxyPanel` 思路一致：

- 没有 `defaultUrl`：`No defaultUrl`
- 没有激活代理：`No active proxies`
- 所有激活工具都和 `defaultUrl` 一致：`Consistent`
- 否则：`Drift`

也就是说，Home 面板会把代理的“配置—现实状态一致性”直接投影成一个总览信号。

## 5.5 审计聚合

- `recentAudits`
- `auditTotal`

审计在这里不是完整查询，而是“总量 + 最近几条”，用来判断最近系统是否频繁发生操作。

## 6. 辅助函数

只有一个轻量函数：`fmtTs(ts)`。

它把 Unix 秒级时间戳转成 `toLocaleString()`。这很符合 Home 面板的原则：

- 不追求复杂时间格式化
- 只需要把最近审计变成可快速扫读的时间文本

## 7. 视图结构

## 7.1 顶部头部

顶部只有两个元素：

- 标题 `Overview`
- `Refresh` 按钮

这再次说明它没有自己的复杂工具条，它的重点是“刷新整页快照”。

## 7.2 KPI 卡片区

第一层主视图是 4 张 KPI 卡：

- Bookmarks
- Ports
- Proxy
- Audits

每张卡都采用同一种结构：

- label
- 主数值
- 辅助摘要

这让不同域的数据能在视觉上被统一比较。

### Bookmarks 卡

展示：

- 书签总数
- 标签总数
- 前 3 个高频标签

并且高频标签会复用 `tagCategoryClass()`，保持和书签面板一致的标签视觉语义。

### Ports 卡

展示：

- 总端口数
- TCP 数
- UDP 数
- PID 数

### Proxy 卡

展示：

- 激活工具数量 / 工具总数
- 一致性状态

### Audits 卡

展示：

- 审计总数
- 最近记录数

## 7.3 下方三块明细区

KPI 下面是一个 `home-grid`，由三块组成：

- `Ports snapshot`
- `Proxy health`
- `Recent audits`

### `Ports snapshot`

这是一个只读表格，展示：

- port
- pid
- process
- protocol

它只展示 `portSnapshot`，不是完整端口全集。

### `Proxy health`

这块把代理配置和工具状态并排放在一起：

- `defaultUrl`
- `noproxy`
- `consistency`
- 每个工具的状态条目

因此它是一块非常典型的“配置 + 运行态”摘要区。

### `Recent audits`

这块展示最近几条审计记录：

- 时间
- 动作
- 目标
- 结果
- 原因

其中结果列会按 `failed` / 其他状态着不同语义色，帮助快速扫出问题操作。

## 8. 加载占位策略

这个组件对加载态非常克制：

- KPI 区首次加载显示 4 张骨架卡
- `Ports snapshot` 用 `SkeletonTable`
- `Recent audits` 用 `SkeletonTable`

它并没有把 `Proxy health` 也做成复杂骨架，而是尽量保持结构轻量。

## 9. 组件特征总结

`HomePanel.vue` 的核心特征有 4 个：

1. **跨域聚合**：一次汇总多个业务域，而不是只关注单一面板。
2. **只读总览**：不做复杂编辑，避免和具体工作台职责重叠。
3. **信号压缩**：通过 KPI、快照和一致性判断，把复杂状态压缩成少量高价值信息。
4. **首次加载骨架化**：第一次进入体验平滑，后续刷新则尽量保留现有内容。

## 10. 推荐阅读顺序

建议这样读这份组件源码：

1. 先看 `load()`，理解它如何并行拉齐四个域
2. 再看 `bookmark*`、`port*`、`proxy*`、`audit*` 这几组 `computed`
3. 最后看模板，体会 KPI 区和三块明细区如何对应这些派生结果

读完之后你会更容易理解：`HomePanel` 不是“把几个组件拼起来”，而是把多个子系统压缩成一个可快速扫描的观察面板。
