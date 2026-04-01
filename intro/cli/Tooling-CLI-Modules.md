# 工程辅助 CLI 模块导读

本文档聚焦另一组很适合从“工程工作流”角度理解的命令：`config`、`ctx`、`find`、`backup`、`delete`。

如果说 `bookmark / proxy / ports / tree` 更像高频日常工作台，那么这五组命令更像工程支撑链路：

- `config`：维护全局配置
- `ctx`：切换项目上下文与会话状态
- `find`：规则驱动的文件搜索
- `backup`：增量备份
- `delete`：受控删除、扫描、预检与交互确认

把这五组放在一起看，会更容易看懂 `xun` 如何覆盖“配置 -> 切换 -> 搜索 -> 备份 -> 清理”这条完整的本地工程流程。

## 1. 为什么把这五组命令放在一起看

它们的共同点不是“都很常用”，而是**都在支撑工程生命周期**：

1. `config` 决定工具默认行为。
2. `ctx` 决定当前会话和项目上下文。
3. `find` 决定如何把目标文件筛出来。
4. `backup` 决定如何在修改前后保留可恢复快照。
5. `delete` 决定如何安全地执行破坏性清理。

所以，这组命令比上一轮的“日常工作台型命令”更偏工程控制面。

## 2. 这五组命令并不走完全相同的分发链

它们虽然都从 `src/cli.rs` 进入，但分发层有两条路：

```text
config / find / backup / delete
  -> src/commands/dispatch/misc.rs

ctx
  -> src/commands/dispatch/env.rs
```

这个细节很重要，因为它说明项目作者把 `ctx` 和 `env` 放进了同一个“环境 / 会话系统”分组，而不是把它当普通杂项命令处理。

## 3. 配置模块：`config`

### 3.1 命令定义层：`src/cli/config.rs`

`config` 的 CLI 定义非常精简，只有三件事：

- `get`
- `set`
- `edit`

但这不代表能力弱，恰恰相反，它把复杂度压缩到了“点路径 + JSON 值”这套抽象上：

- `get`：按点路径读取，如 `proxy.defaultUrl`
- `set`：按点路径写入，值尽量按 JSON 解析
- `edit`：直接打开配置文件人工编辑

这种设计很 KISS：命令面很薄，但表达力足够高。

### 3.2 执行层：`src/commands/app_config.rs`

`config` 的执行层目前集中在一个文件里，但职责仍然比较清楚：

- `config_path()`：统一定位配置文件路径
- `load_json()` / `save_json()`：读写 JSON，写入时先写临时文件再 rename
- `get_by_dot()` / `set_by_dot()`：实现点路径访问
- `parse_value()`：尽量把输入解析为 JSON 值，否则退回字符串
- `cmd_get()` / `cmd_set()` / `cmd_edit()`：对应三个子命令

这个模块的关键点在于：**它不是直接绑定某个固定结构体，而是先用 `serde_json::Value` 做通用路径编辑**。这让 CLI `config` 更像一个通用配置操纵器，而不是只服务某一个子系统。

### 3.3 Dashboard 对应层

`config` 是这五组命令里少数同时拥有 Dashboard 入口的模块：

- 后端路由：`/api/config`
- 后端处理：`src/commands/dashboard/handlers/config.rs`
- 前端组件：`dashboard-ui/src/components/ConfigPanel.vue`

这里有一个很值得注意的分层差异：

- 后端 `config.rs` 支持 `GET /api/config`、`POST /api/config`（局部 patch）和 `PUT /api/config`（整体替换）
- 当前前端 `ConfigPanel.vue` 只暴露了 `tree.defaultDepth` 和 `tree.excludeNames` 两个配置项

也就是说，**后端配置面比前端面板更宽**；目前 Dashboard 只是先把最常改的树配置可视化了。

### 3.4 推荐阅读顺序

1. `src/cli/config.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/app_config.rs`
4. `src/commands/dashboard/mod.rs`
5. `src/commands/dashboard/handlers/config.rs`
6. `dashboard-ui/src/components/ConfigPanel.vue`

## 4. 上下文模块：`ctx`

### 4.1 命令定义层：`src/cli/ctx.rs`

`ctx` 对外暴露的是一套“会话上下文 profile 管理”接口：

- `set`
- `use`
- `off`
- `list`
- `show`
- `del`
- `rename`

从参数定义上就能看出，`ctx` 不是简单的“目录别名”：

- 可以记录 `path`
- 可以携带 `proxy` / `noproxy`
- 可以带默认 `tag`
- 可以注入 `env`
- 可以从 `env-file` 导入环境变量

因此它本质上是“项目工作上下文模板”，不是路径快捷方式。

### 4.2 分发层位置：`src/commands/dispatch/env.rs`

`ctx` 没有走常规 `misc.rs`，而是和 `env` 一起走 `dispatch/env.rs`。这说明在项目结构里，`ctx` 被视作环境系统的一部分。

### 4.3 执行层拆分：`src/commands/ctx*`

`ctx` 模块的拆分很标准：

- `src/commands/ctx.rs`：入口与保留名定义
- `src/commands/ctx/cmd/*`：各个子命令实现
- `src/commands/ctx/env.rs`：环境变量解析与 dotenv 导入
- `src/commands/ctx/proxy.rs`：代理配置归一化与输出
- `src/commands/ctx/session.rs`：当前 active profile 读取
- `src/commands/ctx/validate.rs`：名称与环境变量键校验
- `src/ctx_store.rs`：真正的持久化模型与 session/store 读写

尤其 `src/ctx_store.rs` 很关键，它定义了：

- `CtxStore`
- `CtxProfile`
- `CtxProxy`
- `CtxProxyMode`
- `CtxProxyState`
- `CtxSession`

所以 `ctx` 的“状态模型”并不藏在命令处理函数里，而是被单独抽到 store 层了。

### 4.4 这个模块最重要的设计点

`ctx` 最值得注意的一点是：**它不是静态配置管理，而是动态会话切换系统**。

`use` / `off` 这两个动作不只是改 JSON 文件，它们还会：

- 读取 / 写入 session 文件
- 记录切换前的目录、环境变量和代理状态
- 输出类似 `__ENV_SET__`、`__ENV_UNSET__`、`__CD__` 这样的控制指令
- 依赖 `XUN_CTX_STATE`、`XUN_CTX_FILE` 等环境变量
- 在需要时联动 `proxy::config::{set_proxy, del_proxy}`

这说明 `ctx` 的真正消费端不是 Rust 进程本身，而是 **shell integration**。如果没有 `xun init <shell>` 注入的那层壳，`ctx use`/`ctx off` 只能完成一半。

### 4.5 Dashboard 映射

目前没有独立的 Dashboard `CtxPanel`。

这意味着 `ctx` 目前仍是一个**纯 CLI / shell 集成子系统**，更偏向开发者终端工作流，而不是 Web 可视化工作台。

### 4.6 推荐阅读顺序

1. `src/cli/ctx.rs`
2. `src/commands/dispatch/env.rs`
3. `src/commands/ctx.rs`
4. `src/commands/ctx/cmd/mod.rs`
5. `src/commands/ctx/cmd/set.rs`
6. `src/commands/ctx/cmd/use_ctx.rs`
7. `src/commands/ctx/cmd/off.rs`
8. `src/commands/ctx/cmd/list.rs`
9. `src/commands/ctx/cmd/show.rs`
10. `src/commands/ctx/cmd/delete.rs`
11. `src/commands/ctx/cmd/rename.rs`
12. `src/commands/ctx/proxy.rs`
13. `src/commands/ctx/env.rs`
14. `src/ctx_store.rs`

## 5. 搜索模块：`find`

### 5.1 命令定义层：`src/cli/find.rs`

`find` 的命令面很宽，参数明显已经超出“按名字搜文件”这个层级：

- 规则输入：`include / exclude / regex_include / regex_exclude / filter_file`
- 属性过滤：`extension / not_extension / name / attribute`
- 元数据过滤：`size / fuzzy_size / mtime / ctime / atime / depth`
- 结果控制：`count / dry_run / test_path / format`
- 空文件 / 空目录过滤：`empty_files / empty_dirs / not_empty_files / not_empty_dirs`

这说明 `find` 是一个规则驱动搜索器，而不是单纯 glob 包装层。

### 5.2 命令入口与真实引擎的分离

`src/commands/find.rs` 本身非常薄，只做一件事：

- 把 `FindCmd` 转发给 `crate::find::cmd_find(args)`

真正的核心实现不在 `src/commands/find.rs`，而在 `src/find/*`。这也是理解这个模块时最容易看漏的一点。

### 5.3 底层引擎：`src/find/*`

`src/find/mod.rs` 把 `find` 明确拆成三层：

- `rules`：把 CLI 输入编译成匹配规则
- `filters`：把时间、大小、深度、属性等条件编译成过滤器
- `walker`：负责真正扫描文件系统

这三层的配合关系是：

```text
FindCmd
  -> compile_rules(args)
  -> compile_filters(args)
  -> walker::scan / scan_count
```

其中有几个关键点：

- `dry_run + test_path`：不是扫描磁盘，而是只测试某个路径会被规则判成 include 还是 exclude
- `count`：只计数，不生成完整结果集
- Windows 下优先尝试 `mft::try_scan_mft()`
- 如果不能走 MFT，则退回 `walker` 的并行或单线程扫描

这说明 `find` 已经同时兼顾了**规则可解释性**和**扫描性能**。

### 5.4 读这个模块时最值得抓住什么

`find` 的难点不在命令入口，而在“规则系统 + 扫描器”的耦合方式：

- `rules/*` 解决“路径在逻辑上该不该被纳入”
- `filters/*` 解决“即使纳入规则，也是否满足元数据约束”
- `walker/*` 解决“如何高效遍历并产出结果”

所以读这个模块时，不要只盯着输出渲染；真正的复杂度在规则编译与扫描后端。

### 5.5 Dashboard 映射

目前没有独立的 Dashboard `FindPanel`。

因此 `find` 当前仍然是一个纯 CLI 搜索引擎模块。

### 5.6 推荐阅读顺序

1. `src/cli/find.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/find.rs`
4. `src/find/mod.rs`
5. `src/find/rules/mod.rs`
6. `src/find/filters/compile.rs`
7. `src/find/matcher.rs`
8. `src/find/walker.rs`
9. `src/find/walker/parallel.rs`
10. `src/find/walker/single.rs`
11. `src/find/mft/*`

## 6. 备份模块：`backup / bak`

### 6.1 命令定义层：`src/cli/backup.rs`

正式命令是 `backup`，别名是 `bak`。它现在聚焦四种工作模式：

- 默认：创建一次增量备份
- `backup list`：列出现有备份
- `backup verify <name>`：校验备份完整性
- `backup find [tag]`：按标签或描述查找备份

恢复已经拆到独立的 `restore` 顶层命令，别名为 `rst`。

同时它还支持：

- `--msg`：备份描述
- `--dir`：指定工作目录
- `--dry-run`
- `--no-compress`
- `--retain`
- `--include / --exclude`
- `--incremental`

所以 `backup`（别名 `bak`）并不是简单的“打个 zip”，而是带版本、扫描、差异、压缩与保留策略的备份工具。

### 6.2 执行层拆分：`src/commands/backup*`

`src/commands/backup.rs` 是总控入口，但核心逻辑被拆到多个文件：

- `config.rs`：加载 `.xun-bak.json`，并兼容迁移旧 `.svconfig.json`
- `version.rs`：扫描现有版本并决定下一版本号
- `scan.rs`：扫描当前工作区文件
- `baseline.rs`：读取上一份备份基线
- `diff.rs`：比较当前状态与上一版本，并按差异复制
- `zip.rs`：可选压缩为 zip
- `retention.rs`：执行保留策略
- `list.rs`：列出备份
- `verify.rs`：校验备份完整性
- `find.rs`：按标签或描述筛选备份
- `report.rs` / `util.rs` / `time_fmt.rs`：做报告、大小格式化与时间格式化

这个模块很像一个标准的“快照系统”实现，而不是单命令脚本。

### 6.3 这个模块的控制流

默认创建备份时，大致流程是：

```text
加载 .xun-bak.json（必要时先迁移旧 .svconfig.json）
  -> 扫描历史版本
  -> 生成下一版本名
  -> 扫描当前文件
  -> 读取上一份 baseline
  -> diff + copy
  -> 可选 zip 压缩
  -> 执行 retention
  -> 输出报告
```

其中还有两个很有意思的实现点：

- 它支持配置里的 `include / exclude`，也支持命令行追加 `--include / --exclude`
- 如果配置启用 `useGitignore`，会把 `.gitignore` 规则并入扫描过滤

所以 `backup`（别名 `bak`）不只是“把整个目录复制一份”，而是在做**受规则控制的增量快照**。

### 6.4 恢复语义

恢复职责已经从 `bak` 子模块拆出，当前入口与执行层是：

- `src/cli/restore.rs`
- `src/commands/restore.rs`
- `src/commands/restore_core.rs`

这条链路同时支持：

- 从目录型备份恢复
- 从 zip 型备份恢复
- 恢复整个备份
- 仅恢复一个相对路径文件

而且还显式检查恢复路径安全性，避免不安全的相对路径穿透。这说明它对恢复操作的边界是有防守意识的。

### 6.5 Dashboard 映射

目前没有独立的 Dashboard `BackupPanel` 或 `RestorePanel`。

因此 `backup` / `restore` 目前仍然是纯 CLI 工具模块，更偏面向开发者自己的备份 / 回滚流程。

### 6.6 推荐阅读顺序

1. `src/cli/backup.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/backup.rs`
4. `src/commands/backup/config.rs`
5. `src/commands/backup/version.rs`
6. `src/commands/backup/scan.rs`
7. `src/commands/backup/baseline.rs`
8. `src/commands/backup/diff.rs`
9. `src/commands/backup/verify.rs`
10. `src/commands/backup/find.rs`
11. `src/commands/backup/retention.rs`
12. `src/commands/backup/zip.rs`
13. `src/commands/backup/list.rs`

如果你要追恢复链路，改读：

1. `src/cli/restore.rs`
2. `src/commands/restore.rs`
3. `src/commands/restore_core.rs`

## 7. 删除模块：`delete`

### 7.1 一个容易让人误判的入口位置

`delete` 这个命令的 CLI 定义目前放在 `src/bookmark/cli_commands.rs` 里，而不是单独的 `src/cli/delete.rs`。

这很容易让第一次读代码的人误以为它只是书签附属功能。实际上并不是：

- 书签删除只是 `delete --bookmark / -bm` 的一个分支
- 真正的文件删除系统在 `src/commands/delete/*`
- 它是一个独立而且明显偏重型的子模块

### 7.2 命令入口：`src/commands/delete/cmd.rs`

`cmd_delete()` 做了很多关键的总控工作：

- 先判断是否是 `--bookmark` 分支
- 校验 `--any` 与 `--reserved` 的冲突
- 校验 `level`
- 必要时尝试提权重启
- 解析目标路径，分成 `direct_files` 和 `scan_dirs`
- 计算删除选项 `DeleteOptions`
- 决定是否进入 TUI
- 汇总结果并输出 / 写 CSV 日志

也就是说，`cmd_delete()` 自己并不负责“删除算法细节”，但它是整个删除系统的编排中心。

### 7.3 三条主要分支

#### 7.3.1 书签删除分支

如果传了 `--bookmark`，命令会转到：

- `src/commands/delete/cmd/bookmark.rs`
- 最终调用 `bookmarks::delete_bookmark()`；其实现源码位于 `src/bookmark/commands/mutate.rs`

所以书签删除只是 `delete` 的一条旁路。

#### 7.3.2 非交互 CLI 流水线分支

对目录扫描型删除，核心会落到：

- `src/commands/delete/pipeline/mod.rs`
- `src/commands/delete/pipeline/scan.rs`
- `src/commands/delete/pipeline/execute.rs`

这条流水线大致是：

```text
run_cli_pipeline
  -> smart_scan
  -> delete_paths
  -> render_results / print_summary / write_csv
```

其中：

- `smart_scan()` 负责扫描匹配目标
- `delete_paths()` 负责逐项执行删除
- 如果 `collect_info` 打开，会补文件信息与哈希
- 如果删除失败且 `on_reboot` 打开，会尝试安排重启后删除

#### 7.3.3 TUI 分支

如果满足 `should_use_tui()`，并且启用了 `delete_tui` feature，就会进入：

- `src/commands/delete/tui/app.rs`
- `src/commands/delete/tree/*`

这里的结构其实很完整：

- `FileTree` 负责目录树、可见节点、勾选状态和过滤状态
- TUI 支持浏览、展开 / 折叠、全选 / 取消全选、过滤、确认删除
- 真正确认后，仍然会回到 `pipeline::delete_paths()` 执行底层删除

这说明 TUI 只是交互壳，底层执行器和 CLI 流水线是复用的。

### 7.4 底层支撑层

`delete` 之所以重，不只是因为它会删文件，而是因为它带了大量 Windows 和大规模扫描相关支撑：

- `scanner.rs`：扫描树与 pattern 编译
- `filters.rs`：目标名 / 排除目录 / 文件过滤
- `file_info.rs`：文件信息、类型识别、SHA-256
- `render.rs`：表格 / TSV / JSON / CSV 日志输出
- `winapi/*`：权限、句柄、所有权、删除相关 WinAPI 支撑
- `usn_scan/*`：更底层的扫描支撑
- `reboot_delete.rs`：重启后删除
- `progress.rs`：进度上报

因此它本质上是一个“受控删除平台”，而不是一个 `std::fs::remove_file` 的命令包装。

### 7.5 Dashboard 映射

目前没有独立的 Dashboard 文件删除面板。

需要区分两件事：

- **有**书签删除的 Dashboard 入口，因为书签面板会调用删除书签 API
- **没有**文件删除系统自己的 Dashboard 面板

所以 `delete` 主体仍然是 CLI / TUI 驱动的重型能力。

### 7.6 推荐阅读顺序

1. `src/bookmark/cli_commands.rs`（先看 `delete` 定义）
2. `src/commands/dispatch/misc.rs`
3. `src/commands/delete/mod.rs`
4. `src/commands/delete/cmd.rs`
5. `src/commands/delete/cmd/preflight.rs`
6. `src/commands/delete/pipeline/mod.rs`
7. `src/commands/delete/pipeline/scan.rs`
8. `src/commands/delete/pipeline/execute.rs`
9. `src/commands/delete/render.rs`
10. `src/commands/delete/filters.rs`
11. `src/commands/delete/scanner.rs`
12. `src/commands/delete/file_info.rs`
13. `src/commands/delete/tree/*`
14. `src/commands/delete/tui/*`
15. `src/commands/delete/winapi/*`
16. `src/commands/delete/usn_scan/*`

## 8. 把这五组命令放在一起看，会更容易看懂什么

### 8.1 项目的工程控制面是怎么搭起来的

这五组命令串起来以后，可以看到一个很清晰的本地工程链路：

- `config` 负责“工具默认规则”
- `ctx` 负责“当前工作上下文”
- `find` 负责“把目标筛出来”
- `backup` 负责“动手前先留快照”
- `delete` 负责“最后安全执行清理”

这比单独看某一个命令，更能体现 `xun` 的整体产品思路。

### 8.2 纯 CLI 子系统与双入口子系统的边界

在这五组里：

- `config` 已经有 Dashboard 双入口
- `ctx / find / backup / delete` 仍然主要是 CLI 子系统

这说明项目不是凡事都优先做 Web UI，而是先把 CLI / shell 工作流打磨成熟，再按价值决定是否可视化。

### 8.3 “薄入口 + 厚引擎”是这里的常见模式

这五组里至少有三种典型形态：

- `config`：入口薄，执行也相对集中
- `find`：命令入口极薄，真正复杂度在 `src/find/*`
- `delete`：命令入口负责编排，复杂度分散在 pipeline / tree / winapi / usn_scan

这能帮助你快速判断后续读其他模块时，应该先追“命令面”，还是先追“底层引擎面”。

## 9. 推荐整体阅读顺序

如果你想按工程工作流来继续理解项目，我建议：

1. 先读 `config`
2. 再读 `ctx`
3. 再读 `find`
4. 再读 `backup`
5. 最后读 `delete`

这样做的原因是：

- 前两者是“状态与上下文”
- 中间两者是“查找与保护”
- 最后一个才是“高风险执行”

这个顺序更符合真实使用时的心智模型。

## 10. 当前实现上的几个观察

- `config` 的 CLI 很通用，但 Dashboard `ConfigPanel` 目前只覆盖了树配置的一小部分。
- `ctx` 本质上是 shell 集成系统，不理解 `XUN_CTX_STATE` 和控制指令输出，就很难真正读懂它。
- `find` 的 `src/commands/find.rs` 几乎只是门面，真正复杂度在 `src/find/*`。
- `backup` 已经不是“备份脚本”，而是一个有版本、基线、差异、压缩和 retention 的快照系统。
- `delete` 比它的命令面看起来重得多，尤其带有明显的 Windows 平台支撑特征。
