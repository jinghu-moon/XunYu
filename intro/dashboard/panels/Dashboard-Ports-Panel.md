# Dashboard Ports 面板导读

这篇文档专门拆 `dashboard-ui/src/components/PortsPanel.vue`。

`PortsPanel` 是一个典型的“观察 + 筛选 + 批量视图切换 + 危险动作”的工作台。它不只是列端口，还把进程维度、协议维度、开发端口视角和导出能力都组织到一处。

## 1. 组件定位

这个组件的职责可以概括成：**观察当前端口占用，并在必要时终止对应进程。**

它承担的能力包括：

- 读取 TCP/UDP 端口列表
- 按进程 / PID / 路径 / 命令行过滤
- 按协议过滤
- 只看开发端口
- 自动刷新
- 导出当前结果集
- 在“单表模式”和“按 PID 分组模式”之间切换
- 对指定 PID 执行 kill

所以它比单纯的 `netstat` 表格多了一层“工作台组织能力”。

## 2. 数据契约与接口边界

## 2.1 数据模型

这个面板主要消费：

- `PortInfo`
  - `port`
  - `pid`
  - `name`
  - `exe_path`
  - `cmdline`
  - `cwd`
  - `protocol`

后端返回的是：

- `PortsResponse`
  - `tcp: PortInfo[]`
  - `udp: PortInfo[]`

也就是说，后端语义里 TCP 和 UDP 是天然分开的，而前端再把它们合并成统一结果集处理。

## 2.2 API 边界

该面板只使用两个接口：

- `fetchPorts()`：拉当前端口快照
- `killPid(pid)`：终止某个进程

这里没有逐端口 kill 的 UI 动作，说明当前 Dashboard 更偏向“按进程治理”，而不是细到端口级别的操作面板。

## 3. 状态模型

状态可以分成 4 组。

## 3.1 原始数据状态

- `tcp`
- `udp`

它们分别保存原始协议分组结果。

## 3.2 视图控制状态

- `devOnly`
- `groupByPid`
- `protocolFilter`
- `processFilter`
- `autoRefreshMs`

这组状态决定“当前结果集怎么被切片和展示”。

## 3.3 动作状态

- `killBusyPid`
- `killConfirmKey`
- `killConfirmRemaining`
- `killConfirmTimer`

这组状态只服务于 kill 动作。

## 3.4 横切状态

- `busy`
- `autoTimer`

## 4. 过滤与结果集组织

这个组件的核心不在于原始数据，而在 `filtered` 和 `grouped` 两层结果集转换。

## 4.1 `filtered`

`filtered` 的构建顺序是：

1. 把 `tcp + udp` 合并成统一列表
2. 如果 `devOnly` 为真，只保留 `3000-9999` 区间端口
3. 如果指定协议，只保留 `tcp` 或 `udp`
4. 如果有 `processFilter`，则匹配：
   - 进程名
   - PID
   - `exe_path`
   - `cmdline`
   - `cwd`
5. 最后按端口号升序排序

这意味着它的过滤逻辑不是单纯搜进程名，而是偏“进程上下文搜索”。

## 4.2 `grouped`

如果开启 `groupByPid`，组件会把 `filtered` 结果按 PID 聚合成分组结构：

- `pid`
- `name`
- `exe_path`
- `cmdline`
- `cwd`
- `items: PortInfo[]`

然后：

- 组内按端口排序
- 组间按 PID 排序

所以“按 PID 分组模式”不是另一套接口，而是当前过滤结果的再组织。

## 4.3 其他派生

- `isLoading`：只在首次且没有任何数据时才显示骨架屏
- `skeletonColumns`：根据当前模式切换骨架列表列数

这说明组件对两种视图模式都有对应的加载占位策略。

## 5. 进程图标与标题辅助

这个面板还做了一些很实用的增强：

- `iconUrl(pid)`：从 `/api/ports/icon/{pid}?size=...` 拉进程图标
- `iconFallback(name)`：图标失败时，用进程名首字母兜底
- `procTitle(...)`：把 `cmdline` 和 `cwd` 组合成 tooltip 文本
- `onIconLoad()` / `onIconError()`：在 DOM 上标记图标成功或失败状态

因此它并不只是文本表格，而是有一点“进程浏览器”的味道。

## 6. 核心工作流

## 6.1 加载

`load()` 逻辑很简单：

1. 如果当前已经 `busy`，直接返回，避免重入
2. 调 `fetchPorts()`
3. 更新 `tcp` / `udp`

这说明端口面板认为“端口快照”是一类应该整体替换的数据，而不是局部 patch 的数据。

## 6.2 Kill 进程

`onKillPid(pid, name)` 采用 3 秒确认窗模式：

- 第一次点击：进入 armed 状态
- 第二次点击：调用 `killPid(pid)`
- 成功后重新 `load()`

这和书签删除、代理移除的确认风格一致，说明 Dashboard 对危险动作采用统一的轻确认策略。

## 6.3 自动刷新

自动刷新由：

- `autoRefreshMs`
- `startAutoRefresh(ms)`
- `stopAutoRefresh()`
- `watch(autoRefreshMs, ...)`

共同实现。

它的好处是：

- 切换刷新间隔时会自动重建 timer
- 组件卸载时会正确清理 timer
- 不需要引入额外状态库或 polling 抽象层

## 6.4 导出

`exportPorts(format)` 导出的不是原始 `tcp/udp`，而是 **当前 `filtered` 结果集**。

支持：

- JSON：`downloadJson('ports', items)`
- CSV：`downloadCsv('ports', ...)`

所以和书签面板一样，它也是“导出当前视图”，不是“导出全部原始数据”。

## 7. 视图结构

## 7.1 第一层工具条

第一层工具条包括：

- `Refresh`
- 自动刷新间隔选择
- 导出 CSV / JSON
- `Group by PID`

这层主要是“面板级控制”。

## 7.2 第二层工具条

第二层工具条包括：

- 进程过滤输入框
- 协议过滤下拉框
- `Dev ports only`

这层主要是“结果集切片控制”。

## 7.3 单表模式

关闭 `groupByPid` 时，组件渲染标准端口表：

- Port
- PID
- Process
- Protocol
- Kill 按钮

其中 Process 列会显示：

- 进程图标 / 首字母兜底
- 进程名
- tooltip 中的命令行和工作目录信息

这是一种偏“逐条扫描”的视图。

## 7.4 按 PID 分组模式

开启 `groupByPid` 后，组件按进程分组展示：

- 每个 PID 一块 `<details>`
- header 展示进程信息和该 PID 下端口数量
- 组内表格只展示 Port / Protocol / Kill

这时视角就从“端口表”切到“进程表”。

也就是说：

- 单表模式适合找“某个端口是谁占的”
- 分组模式适合看“某个进程占了哪些端口”

## 8. 组件特征总结

`PortsPanel.vue` 的设计重点可以概括为：

1. **统一结果集**：先把 TCP/UDP 合并，再做过滤和展示。
2. **双视图切换**：同一份数据同时支持端口视角和进程视角。
3. **轻量危险动作**：Kill 使用 3 秒确认窗。
4. **观察强化**：图标、tooltip、自动刷新和导出都在增强“观察效率”。

## 9. 推荐阅读顺序

建议这样读这份组件源码：

1. 先看 `filtered` 和 `grouped`
2. 再看 `load()`、`onKillPid()`、自动刷新相关函数
3. 然后看导出逻辑
4. 最后看模板里两种模式的对应关系

这样你会更容易把它理解成一个“结果集工作台”，而不是一个单纯的端口列表。
