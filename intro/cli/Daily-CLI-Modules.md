# 日常 CLI 模块导读

本文档聚焦 `xun` 里最适合日常高频使用、也最容易拿来建立项目直觉的四组命令：`bookmark`、`proxy`、`ports`、`tree`。

和 `env`、`acl`、`redirect`、`diff` 这种“系统级子模块”相比，这四组命令更像是项目的日常工作台：

- `bookmark` 负责目录导航、书签维护和标签组织。
- `proxy` 负责代理状态查看、配置写入和带代理执行。
- `ports` 负责端口占用、进程查询与快速结束进程。
- `tree` 负责目录树构建、过滤、统计和输出。

如果你想“一边读代码，一边马上知道这个项目平时怎么用”，这篇文档通常比先读重量级子系统更友好。

## 1. 这四组命令为什么适合放在一起看

它们有三个共同点：

1. **都走同一条常规 CLI 分发链路**：`main.rs -> cli.rs -> commands/dispatch/mod.rs -> commands/dispatch/misc.rs -> 各自模块`。
2. **都不是 feature gate 下的重型实验能力**：大多数情况下，它们属于默认可用能力，是这个工具箱的日常入口层。
3. **都已经完成了明确的模块拆分**：不是把逻辑堆在一个超长文件里，而是按“定义层 / 执行层 / 辅助层”拆开，适合逐个组件理解。

所以，把这四组命令放一起看，可以快速理解 `xun` 的常规命令组织方式。

## 2. 共同执行链路

这四组命令都遵循同一种结构：

```text
src/main.rs
  -> 参数兼容修正 / argh 解析
  -> src/cli.rs 暴露顶层 Xun / SubCommand
  -> src/commands/dispatch/mod.rs
     -> src/commands/dispatch/misc.rs
        -> bookmark / proxy / ports / tree
```

这个结构的价值在于：

- `src/cli/*` 只负责“命令长什么样”。
- `src/commands/dispatch/*` 只负责“命令应该交给谁”。
- `src/commands/<module>/*` 才负责“命令到底怎么执行”。

读这四组命令时，建议都按这个顺序走一遍，不要一上来就钻进执行细节。

## 3. 书签模块：`bookmark`

### 3.1 命令定义层：`src/bookmark/cli_namespace.rs` + `src/bookmark/cli_commands.rs`

这一层回答的是“书签系统对外暴露了哪些动作”。从定义上看，`bookmark` 不是单命令，而是一整组围绕目录导航的数据管理能力：

- 列表与查询：`list`、`recent`、`stats`、`all`、`keys`
- 导航动作：`z`、`zi`、`o`、`oi`、`open`
- 数据写入：`save`、`set`、`touch`、`rename`、`pin`、`learn`
- 标签管理：`tag add/remove/list/rename`
- 维护动作：`check`、`gc`、`dedup`
- 数据交换：`export`、`import`

这里最值得注意的一点是：**书签既有面向人类交互的命令，也有面向脚本 / 补全 / 机器消费的命令**。例如 `all`、`keys`、`z --list` 就明显偏机器接口。

### 3.2 执行层拆分：`src/bookmark/commands/*`

书签模块是一个职责边界很清晰的命令族：

- `list.rs`：负责展示、最近访问、统计、机器输出等“读路径”。
- `navigation.rs`：负责 `z / zi / o / oi / open` 等“导航路径”。
- `mutate.rs`：负责 `save`、`set`、`touch`、`rename`、`delete_bookmark` 等“写路径”。
- `tags.rs`：负责标签增删改查。
- `io.rs`：负责导入导出。
- `maintenance/*`：负责 `check`、`gc`、`dedup` 等“数据健康治理”。
- `integration.rs`：负责 `learn / init / import --from ...` 等外部集成。

这种拆法很符合单一职责原则：

- “展示”不夹带“修改”；
- “导航”不夹带“数据清洗”；
- “标签管理”不和“导入导出”混在一起。

从读代码体验上，这个模块已经比较成熟，职责分层明显。

### 3.3 一个容易看漏的旁路：`delete -bm`

虽然“删除书签”能力最终会落到 `bookmarks::delete_bookmark()`，但它的 CLI 入口不是单独的 `rm-bookmark` 之类，而是复用了更通用的 `delete` 命令：

- `src/bookmark/cli_commands.rs` 定义了 `delete`，并提供 `--bookmark / -bm`
- `src/commands/dispatch/misc.rs` 把它分发到 `src/commands/delete/*`
- `src/commands/delete/cmd.rs` 检查 `args.bookmark`
- 如果是 `--bookmark`，再转到 `src/commands/delete/cmd/bookmark.rs`
- 最终调用 `bookmarks::delete_bookmark()`；其实现源码位于 `src/bookmark/commands/mutate.rs`

这说明项目并没有把“删除”只理解为书签动作，而是抽象成了一个更通用的删除框架；书签删除只是其中一个分支。

### 3.4 Dashboard 对应层

书签模块在 Dashboard 里有非常完整的可视化入口：

- 后端 API：`/api/bookmarks`、`/api/bookmarks/export`、`/api/bookmarks/import`、`/api/bookmarks/{name}`、`/api/bookmarks/{name}/rename`、`/api/bookmarks/batch`
- 后端处理：`src/commands/dashboard/handlers/bookmarks.rs`
- 前端组件：`dashboard-ui/src/components/BookmarksPanel.vue`

`BookmarksPanel.vue` 不是只做一个简单列表，而是把很多 CLI 能力搬成了交互式工作台：

- 搜索与标签过滤
- 列表 / 分组视图切换
- 行内编辑名称与标签
- 单条删除与批量删除
- 批量加标签 / 去标签
- CSV / JSON 导出
- 复制路径、直接打开路径

所以，如果你想看“同一套书签能力如何同时服务 CLI 和 Web UI”，书签模块是非常好的样本。

### 3.5 推荐阅读顺序

1. `src/bookmark/cli_namespace.rs`
2. `src/bookmark/cli_commands.rs`
3. `src/commands/dispatch/misc.rs`
4. `src/bookmark/commands/mod.rs`
5. `src/bookmark/commands/list.rs`
6. `src/bookmark/commands/navigation.rs`
7. `src/bookmark/commands/mutate.rs`
8. `src/bookmark/commands/tags.rs`
9. `src/bookmark/commands/io.rs`
10. `src/bookmark/commands/integration.rs`
11. `src/bookmark/commands/maintenance/*`
12. `src/commands/dashboard/handlers/bookmarks.rs`
13. `dashboard-ui/src/components/BookmarksPanel.vue`

## 4. 代理模块：`proxy`

### 4.1 命令定义层：`src/cli/proxy.rs`

代理模块对外暴露的动作非常完整，既能“修改状态”，也能“读取状态”，还能“带着代理执行命令”：

- 主命令：`proxy`
- 快捷动作：`pon`、`poff`、`pst`、`px`
- 配置动作：`set`、`del`、`get`
- 诊断动作：`detect`、`test`

仅从命令定义就能看出，这不是一个只会写环境变量的小工具，而是一个包含**配置写入、系统探测、状态观测、网络测试**的代理管理子系统。

### 4.2 执行层拆分：`src/commands/proxy/*`

代理模块内部主要分成四层：

- `ops/apply.rs`：负责 `cmd_proxy_on`、`cmd_proxy_off`、`cmd_proxy_exec`、`cmd_proxy`，也就是“真正执行开关与带代理运行”。
- `ops/detect.rs`：负责 `cmd_proxy_detect`、`cmd_proxy_status`，也就是“探测现在是什么状态”。
- `config.rs`：负责配置落盘、按目标工具写入 / 删除代理、解析 `only=` 过滤范围，以及代理状态持久化。
- `env.rs`：负责环境变量输出与系统代理解析，包含对系统代理值的读取逻辑。
- `test.rs`：负责代理连通性与延迟测试。

这个拆法背后的设计很清晰：

- **apply** 解决“要做什么改动”；
- **detect** 解决“当前状态是什么”；
- **config** 解决“改动如何落地到 Git / npm / Cargo / MSYS2 等对象”；
- **test** 解决“改了以后是否真的通”。

相比把所有逻辑塞进一个 `cmd_proxy()`，现在这种拆法更易扩展，也更容易替换单一目标工具的写入策略。

### 4.3 这个模块的核心价值

代理模块最重要的不是“把 `HTTP_PROXY` 设上”，而是统一多种代理落点：

- 进程级环境变量
- Git 配置
- npm 配置
- Cargo 配置
- MSYS2 环境
- 代理状态缓存 / 持久化

因此它实际上是一个“多后端代理编排器”，不是单点配置脚本。

### 4.4 Dashboard 对应层

代理模块在 Dashboard 里也有完整入口：

- 后端 API：`/api/proxy/status`、`/api/proxy/config`、`/api/proxy/test`、`/api/proxy/set`、`/api/proxy/del`
- 后端处理：`src/commands/dashboard/handlers/proxy.rs`
- 前端组件：`dashboard-ui/src/components/ProxyPanel.vue`

`ProxyPanel.vue` 的状态组织方式和 CLI 视角相互呼应：

- `items`：当前代理状态视图
- `cfg`：持久化配置视图
- `url / noproxy / only / includeMsys2`：一次设置动作所需输入
- `testTargets / testTimeoutMs / testJobs / testResult`：测试动作的参数与结果

也就是说，Web UI 不是单纯展示状态，而是把“状态读取 + 配置保存 + 立即生效 + 测试验证”串成一个完整闭环。

### 4.5 推荐阅读顺序

1. `src/cli/proxy.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/proxy/mod.rs`
4. `src/commands/proxy/ops/apply.rs`
5. `src/commands/proxy/ops/detect.rs`
6. `src/commands/proxy/config.rs`
7. `src/commands/proxy/env.rs`
8. `src/commands/proxy/test.rs`
9. `src/commands/dashboard/handlers/proxy.rs`
10. `dashboard-ui/src/components/ProxyPanel.vue`

## 5. 端口模块：`ports`

### 5.1 命令定义层：`src/cli/ports.rs`

端口模块由四个常用动作组成：

- `ports`：看端口占用
- `kill`：按端口杀进程
- `ps`：按进程名 / PID / 窗口标题查进程
- `pkill`：按进程名 / PID / 窗口标题结束进程

这四个动作放在一起，构成了一个标准的“诊断 + 定位 + 处置”闭环。

### 5.2 执行层拆分：`src/commands/ports/*`

端口模块内部拆分得很干净：

- `query.rs`：负责 `cmd_ports()`，做端口枚举、过滤、格式化输出。
- `process.rs`：负责 `cmd_ps()`、`cmd_pkill()`，做进程搜索与结束。
- `kill.rs`：负责 `cmd_kill()`，按端口回收占用进程。
- `render.rs`：负责表格渲染。
- `common.rs`：负责开发端口判断、范围解析、字符串截断等公共逻辑。

同时它又明显依赖两个底层基础能力：

- `crate::ports`：偏端口视角，负责监听端口与协议数据。
- `crate::proc`：偏进程视角，负责查找和结束进程。

因此这个模块本质上是在做“端口模型”和“进程模型”的组合编排。

### 5.3 读这个模块时应该抓住什么

`ports` 这组命令虽然没有 `env` 那么庞大，但实现很实用，也很工程化：

- `cmd_ports()` 会先枚举，再按协议 / 范围 / PID / 名称过滤，最后按输出格式决定是表格、TSV 还是 JSON。
- `cmd_ps()` / `cmd_pkill()` 明确把“查找”和“结束”区分开，避免一个命令承担过多职责。
- `render.rs` 单独存在，说明输出样式在这里是独立关注点，而不是查询逻辑的附属品。

这是一种非常典型的“命令薄、组合多、输出独立”的工具型模块结构。

### 5.4 Dashboard 对应层

端口模块在 Dashboard 中也有很强的可视化表达：

- 后端 API：`/api/ports`、`/api/ports/icon/{pid}`、`/api/ports/kill/{port}`、`/api/ports/kill-pid/{pid}`
- 后端处理：`src/commands/dashboard/handlers/ports.rs`
- 前端组件：`dashboard-ui/src/components/PortsPanel.vue`

`PortsPanel.vue` 做的事情比 CLI 更多：

- TCP / UDP 合并展示
- 开发端口过滤
- 按 PID 分组
- 自动刷新
- 图标加载与降级显示
- CSV / JSON 导出
- 带确认窗口的 Kill 动作

后端 `ports.rs` 还实现了图标缓存，这说明 Dashboard 端口页并不只是“把 CLI 输出换个皮”，而是围绕观察效率做了额外增强。

### 5.5 推荐阅读顺序

1. `src/cli/ports.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/ports/mod.rs`
4. `src/commands/ports/query.rs`
5. `src/commands/ports/process.rs`
6. `src/commands/ports/kill.rs`
7. `src/commands/ports/render.rs`
8. `src/commands/ports/common.rs`
9. `src/commands/dashboard/handlers/ports.rs`
10. `dashboard-ui/src/components/PortsPanel.vue`

## 6. 树形模块：`tree`

### 6.1 命令定义层：`src/cli/tree.rs`

`tree` 是一个单命令工具，但参数非常丰富：

- 深度控制：`--depth`
- 输出控制：`--output`、`--plain`、`--stats-only`
- 过滤控制：`--include`、`--exclude`、`--hidden`
- 性能控制：`--fast`、`--max-items`
- 展示控制：`--sort`、`--size`
- 体验控制：`--no-clip`

这说明 `tree` 虽然只是一个命令，但内部已经是一个小型流水线，而不是简单递归打印目录。

### 6.2 执行层拆分：`src/commands/tree/*`

树形模块的执行链是这四组命令里最规整的一种：

- `cmd.rs`：总控入口 `cmd_tree()`，负责读取配置、组装过滤器、决定走统计模式还是输出模式。
- `collect.rs`：负责采集目录项与排序前准备。
- `build.rs`：负责递归构建树输出，以及统计计数。
- `filters.rs`：负责排序参数解析和排除规则判断。
- `format.rs`：负责字节大小格式化。
- `stats.rs`：负责统计结果输出。
- `clipboard.rs`：负责复制到剪贴板（Windows 下启用）。
- `types.rs`：集中定义 `SortKey`、`TreeFilters`、`TreeItem`、`TreeOutput` 等核心类型。

这个结构很像一个标准的“扫描 -> 过滤 -> 排序 -> 构建 -> 输出”管线。

### 6.3 这个模块最值得看的设计点

`cmd_tree()` 里有两个分支特别值得注意：

1. **`stats_only` 分支**：只计数、不构建输出树，说明统计和渲染被有意识地拆开了。
2. **buffer / stream 分支**：如果需要保存文件或复制剪贴板，就先缓冲；否则可以直接输出。

这两个分支都体现出一个思路：`tree` 不是只生成“屏幕文本”，而是把树结果当成一种中间产物，可以被复用到更多输出通道。

### 6.4 配置与过滤来源

树形模块并不只依赖命令行参数，它还会整合：

- 内置默认排除项
- 全局配置里的 `tree` 配置
- 根目录下的 `.xunignore`
- CLI 的 `--include / --exclude`

所以读这个模块时，不要只盯着递归逻辑；真正决定输出内容的是过滤策略组合。

### 6.5 Dashboard 映射

目前这部分能力没有像 `bookmark / proxy / ports` 那样对应到独立 Dashboard 面板。

换句话说，`tree` 现在更像一个纯 CLI 工具模块：

- 关注快速扫描
- 关注输出质量
- 关注复制 / 落盘 / 统计
- 暂时不追求可视化交互工作台

这也让它成为理解 `xun`“纯命令行工具模块”写法的一个很干净的样本。

### 6.6 推荐阅读顺序

1. `src/cli/tree.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/tree.rs`
4. `src/commands/tree/cmd.rs`
5. `src/commands/tree/types.rs`
6. `src/commands/tree/filters.rs`
7. `src/commands/tree/collect.rs`
8. `src/commands/tree/build.rs`
9. `src/commands/tree/format.rs`
10. `src/commands/tree/stats.rs`
11. `src/commands/tree/clipboard.rs`

## 7. 把四组命令放在一起看，会更容易看懂什么

### 7.1 `xun` 的常规命令组织方式

这四组命令一起看，你会发现项目对“普通命令模块”有一套稳定范式：

- `src/cli/<module>.rs` 定义参数结构
- `dispatch/misc.rs` 负责分发
- `src/commands/<module>/mod.rs` 做导出汇总
- 再按职责拆成多个子文件

这说明项目不是随机拆文件，而是有明确的命令族组织习惯。

### 7.2 CLI 与 Dashboard 的复用边界

在这四组里：

- `bookmark`、`proxy`、`ports` 同时服务 CLI 与 Dashboard
- `tree` 目前主要服务 CLI

这能帮助你快速判断一个子模块处于哪种成熟阶段：

- 只有 CLI：通常是“工具能力已成型，但还没可视化封装”
- CLI + Dashboard：通常说明能力边界已经稳定，值得做双入口复用

### 7.3 项目当前的工程风格

这四组命令共同体现出几种明显风格：

- 输出格式通常是独立关注点，而不是顺手 `println!`
- 交互确认被单独设计，而不是到处散落
- 状态读取和状态修改倾向于分层拆开
- Dashboard 倾向于在 CLI 能力上再叠加筛选、导出、批量操作与可视化反馈

## 8. 推荐整体阅读顺序

如果你准备按“从最容易建立直觉，到最复杂系统”的顺序继续理解项目，建议：

1. 先读 `bookmark`
2. 再读 `proxy`
3. 再读 `ports`
4. 再读 `tree`
5. 然后回到 `env`
6. 再到 `acl`
7. 再看 `redirect`
8. 最后读 `diff` 和 Dashboard 深水区

这个顺序的好处是：先建立“普通命令模块”的手感，再进入规则引擎和重系统模块，理解成本会低很多。

## 9. 当前实现上的几个观察

- `bookmark` 是默认能力里最像“核心产品功能”的模块，不只是工具函数集合。
- `proxy` 明显在往“多后端状态编排器”方向发展，而不是单纯环境变量脚本。
- `ports` 的 CLI 和 Dashboard 复用关系很自然，是观察型能力复用得最顺的一组。
- `tree` 虽然没有 Dashboard 面板，但内部管线已经很完整，是纯 CLI 模块的优秀样本。
- 仓库里仍有不少 `*_monolith_backup_*` 文件，说明这些模块最近经历过持续拆分；当前读到的结构已经是较新的模块化结果。
