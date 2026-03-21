# CLI 模块导读

本文档从“一个命令模块一个命令模块”的角度，梳理 `xun` 的 CLI 入口、定义层、执行层和各命令族职责，适合你在阅读 Rust 源码时快速建立模块地图。

## 1. 执行链路总览

`xun` 的 CLI 主链路可以概括为：

```text
main.rs
  -> 参数兼容修正（如 del / -bm）
  -> argh 解析为 cli::Xun
  -> runtime::init()
  -> commands::dispatch()
     -> dispatch/core.rs
     -> dispatch/env.rs
     -> dispatch/dashboard.rs
     -> dispatch/misc.rs
```

这条链路有两个明显特征：

1. **定义层和执行层分离**：`src/cli/*` 只负责命令定义与参数结构；真正的业务逻辑放在 `src/commands/*`。
2. **分发层按职责分组**：不是一个超长 `match`，而是拆成 `core / env / dashboard / misc` 四层，降低了主 dispatch 的复杂度。

## 2. 三层结构：定义、分发、执行

### 2.1 定义层：`src/cli/*`

这一层是 `argh::FromArgs` 定义，回答的是“用户可以输入什么命令、带什么参数”。

- `src/cli.rs`：顶层 `Xun` 与 `SubCommand` 枚举。
- `src/cli/bookmarks.rs`：书签与导航命令族。
- `src/cli/config.rs`：配置命令族。
- `src/cli/ctx.rs`：上下文切换命令族。
- `src/cli/proxy.rs`：代理命令族。
- `src/cli/ports.rs`：端口 / 进程命令族。
- `src/cli/env/`：环境变量系统的子命令树。
- `src/cli/shell.rs`：`init` / `completion` / `__complete`。
- 以及一批 feature-gated 模块：`alias`、`lock`、`protect`、`crypt`、`dashboard`、`redirect`、`diff`、`batch_rename`、`cstat`、`img`。

### 2.2 分发层：`src/commands/dispatch/*`

- `core.rs`：只处理 `init`，因为它本质上是 shell 集成脚本生成，不属于一般业务命令。
- `env.rs`：只处理 `ctx` 和 `env`，说明这两块被视为“会话 / 环境系统”。
- `dashboard.rs`：处理 `serve`、`cstat`、`img` 这类相对独立、重量较高的功能块。
- `misc.rs`：剩余大多数命令都在这里转给具体模块执行。

### 2.3 执行层：`src/commands/*`

这一层是真正的实现。命令族普遍继续拆分为多个子文件，例如：

- 书签：`list / navigation / mutate / tags / io / maintenance`
- 端口：`query / process / kill / render / common`
- 代理：`ops / detect / config / env / test`
- Env：`cmd/*` 下再按 `vars / snapshot / schema / profile / run ...` 拆分

## 3. 顶层命令族总览

下面这张表优先回答“这个命令族大概是干什么的、实现在哪”。

| 命令族 | 主要命令 | 定义层 | 执行层 | 说明 |
| --- | --- | --- | --- | --- |
| Shell 集成 | `init` `completion` `__complete` | `src/cli/shell.rs` | `dispatch/core.rs`、`commands/completion.rs` | 输出 shell wrapper、补全脚本、动态补全结果 |
| 书签 / 导航 | `list` `z` `o` `ws` `sv` `set` `delete` `tag` ... | `src/cli/bookmarks.rs` | `src/commands/bookmarks/*` + `src/commands/delete/*` | `xun` 的传统核心能力 |
| 配置 | `config get/set/edit` | `src/cli/config.rs` | `src/commands/app_config.rs` | 面向全局 JSON 配置文件 |
| 上下文 | `ctx set/use/off/list/show/del/rename` | `src/cli/ctx.rs` | `src/commands/ctx.rs` | 项目 / 会话上下文切换 |
| 代理 | `proxy` `pon` `poff` `pst` `px` | `src/cli/proxy.rs` | `src/commands/proxy/*` | 系统 / 工具链代理状态管理 |
| 端口 / 进程 | `ports` `kill` `ps` `pkill` | `src/cli/ports.rs` | `src/commands/ports/*` | 端口查看、按端口杀进程、进程搜索 |
| 备份 | `backup` `bak` | `src/cli/bak.rs` | `src/commands/bak.rs`（模块名 `backup`） | 备份与归档 |
| 恢复 | `restore` `rst` | `src/cli/restore.rs` | `src/commands/restore.rs` + `src/commands/restore_core.rs` | 从目录/zip 备份恢复文件 |
| 树视图 | `tree` | `src/cli/tree.rs` | `src/commands/tree/*` | 文件树收集、过滤、格式化、复制 |
| 搜索 | `find` | `src/cli/find.rs` | `src/commands/find.rs` | 搜索相关能力 |
| Env | `env ...` | `src/cli/env/*` | `src/commands/env/*` | 独立子系统，功能最完整 |
| ACL | `acl ...` | `src/cli/acl.rs` | `src/commands/acl_cmd/*` | Windows ACL 读写、diff、审计、修复 |
| Video | `video probe/compress/remux` | `src/cli/video.rs` | `src/commands/video/*` | 视频探测、压缩、封装转换 |
| Alias | `alias ...` | `src/cli/alias.rs` | `src/commands/alias.rs` | feature: `alias` |
| Lock | `lock` `mv` `ren` | `src/cli/lock.rs` | `src/commands/lock/*` | feature: `lock` |
| FS 删除 | `rm` | `src/cli/fs.rs` | `src/commands/fs.rs` | feature: `fs` |
| Protect | `protect ...` | `src/cli/protect.rs` | `src/commands/protect.rs` | feature: `protect` |
| Crypt | `encrypt` `decrypt` | `src/cli/crypt.rs` | `src/commands/crypt.rs` | feature: `crypt` |
| Redirect | `redirect` | `src/cli/redirect.rs` | `src/commands/redirect/*` | feature: `redirect` |
| Dashboard | `serve` | `src/cli/dashboard.rs` | `src/commands/dashboard/*` | feature: `dashboard` |
| Diff | `diff` | `src/cli.rs` + diff 定义 | `src/commands/diff.rs` | feature: `diff` |
| Batch Rename | `brn` | `src/cli/batch_rename.rs` | `src/commands/batch_rename/*` | feature: `batch_rename` |
| CStat | `cstat` | `src/cli/cstat.rs` | `src/commands/cstat/*` | feature: `cstat` |
| Img | `img` | `src/cli/img.rs` | `src/commands/img.rs` | feature: `img` |

## 4. 一个命令族一个命令族地看

### 4.1 `init` / `completion` / `__complete`

- `init`：打印 shell 集成脚本。它会注入 wrapper 函数、别名、动态补全以及一些“魔法输出”解释逻辑，例如 `__CD__:`、`__ENV_SET__:`。
- `completion`：输出 shell 补全脚本，支持 `powershell / bash / zsh / fish`。
- `__complete`：内部补全入口，接收预分词参数，返回候选项；通常由 shell 补全脚本间接调用，不建议用户直接手敲。

这组命令的设计目标是：**让 CLI 不只是“被调用”，还要能“嵌进 shell 工作流”**。

### 4.2 书签 / 导航命令族

定义层在 `src/cli/bookmarks.rs`，执行层拆成：

- `list.rs`：`list / recent / stats / all / fuzzy / keys`
- `navigation.rs`：`z / open(o) / workspace(ws)`
- `mutate.rs`：`save(sv) / set / touch / rename`
- `tags.rs`：`tag add/remove/list/rename`
- `io.rs`：`export / import`
- `maintenance/`：`check / gc / dedup`

这一族是 `xun` 最像“日常效率工具”的部分，既有导航，也有数据维护，还有导入导出和标签体系。

一个值得注意的点：`delete` 在 CLI 里长得像书签命令，但实际执行在 `src/commands/delete/*`，因为它同时覆盖“删除书签”和“删除文件”两类语义。

### 4.3 `config`

- 子命令：`get / set / edit`
- 实现在 `src/commands/app_config.rs`

这组命令面向 `%USERPROFILE%\.xun.config.json` 这一类全局配置文件，属于“直接改配置”的窄工具，而不是带 schema 的复杂配置系统。

### 4.4 `ctx`

- 子命令：`set / use / off / list / show / del / rename`
- 执行在 `src/commands/ctx.rs`

`ctx` 更像一个“会话态上下文切换器”，用于给当前 shell / 当前任务附加项目上下文，而不是单纯的数据 CRUD。

### 4.5 `proxy` / `pon` / `poff` / `pst` / `px`

代理系统分两层：

- 直接命令：`pon`、`poff`、`pst`、`px`
- 子命令命名空间：`proxy set/del/get/detect/test`

执行层又拆成：

- `ops/`：应用 / 关闭代理、执行命令
- `detect.rs`：探测与状态汇总
- `config.rs`：配置持久化
- `env.rs`：环境变量相关逻辑
- `test.rs`：连通性测试

这一族的核心不是“保存一个 URL”，而是**同时对齐系统代理、工具链环境变量和配置文件**。

### 4.6 `ports` / `kill` / `ps` / `pkill`

执行层清晰分成三件事：

- `query.rs`：`ports`
- `kill.rs`：`kill`
- `process.rs`：`ps / pkill`

配套还有：

- `common.rs`：端口范围和公共判定
- `render.rs`：输出渲染

这说明端口系统并不只盯“端口”，而是把“端口”和“进程”作为一个联合视角来设计。

### 4.7 `backup` / `bak`

- 正式命令：`backup`
- 别名：`bak`
- 执行在 `src/commands/bak.rs`（模块名 `backup`）

这是相对独立的命令族，适合单独阅读，不依赖复杂分发。

### 4.8 `restore` / `rst`

- 正式命令：`restore`
- 别名：`rst`
- 执行在 `src/commands/restore.rs` + `src/commands/restore_core.rs`

它负责从目录型或 zip 型备份恢复文件，并带有 snapshot、glob 和安全校验逻辑。

### 4.9 `tree`

`tree` 的执行层虽然对外只有一个命令，但内部拆得很细：

- `collect.rs`：收集目录项
- `filters.rs`：过滤逻辑
- `format.rs`：输出格式
- `stats.rs`：统计
- `clipboard.rs`：复制结果
- `build.rs` / `types.rs` / `constants.rs`：构建与类型支撑

这说明 `tree` 已经不是“打印一个树”这么简单，而是一个带过滤、统计和输出能力的子模块。

### 4.9 `find`

- 单入口命令：`find`
- 执行在 `src/commands/find.rs`

从代码组织上看，这是一类相对聚焦、实现尚未继续重拆的命令。

### 4.10 `env`

这是当前 CLI 里最完整的子系统。定义层在 `src/cli/env/`，执行层在 `src/commands/env/cmd/`，二者几乎是一一对应的。

定义层分组：

- `status.rs`：`status`
- `vars.rs`：`list / search / get / set / del`
- `path.rs`：`path / path-dedup / add / rm`
- `snapshot.rs`：`snapshot create/list/restore/prune`
- `doctor.rs`：`check / doctor / audit / watch`
- `profile.rs`：`profile list/capture/apply/diff/delete`
- `batch.rs`：`batch set/delete/rename`
- `import_export.rs`：`apply / export / export-all / export-live / env / import`
- `diff_graph.rs`：`diff-live / graph / validate`
- `schema.rs`：`schema show/add-required/add-regex/add-enum/remove/reset`
- `annotations.rs`：`annotate set/list`
- `config.rs`：`config show/path/reset/get/set`
- `run.rs`：`template / run / tui`

执行层也按同样的文件名继续拆开，这种“定义层和执行层镜像对齐”的组织方式非常适合维护。

如果你要深入 `env`，建议把它当成一个单独产品来理解，而不是一个普通子命令。

### 4.11 `acl`

`acl` 是另一个重量级子系统，子命令包括：

- `view / add / remove / purge / diff / batch / effective / copy`
- `backup / restore / inherit / owner / orphans / repair / audit / config`

执行层按职责拆成：

- `view.rs`：查看 / diff / effective
- `edit.rs`：增删改、copy、inherit、owner
- `batch.rs`：批量、备份、恢复
- `repair.rs`：孤儿项与修复
- `audit.rs`：ACL 审计
- `config.rs`：ACL 配置

这组命令明显是 Windows ACL 运维工具，不建议把它和普通文件权限语义简单等同。

### 4.12 `video`

- `video probe`
- `video compress`
- `video remux`

执行层对应：`probe.rs`、`compress.rs`、`remux.rs`，由 `mod.rs` 统一调度。组织上很清爽，适合直接顺着入口看。

### 4.13 feature 命令族

这些命令族默认不一定可用，但结构上同样清晰：

- `alias`：alias / app alias 体系
- `lock`：`lock who`、`mv`、`ren`
- `rm`：独立文件删除命令
- `protect`：`set / clear / status`
- `crypt`：`encrypt / decrypt`
- `redirect`：目录重定向 / 分类规则
- `serve`：本地 Dashboard 服务入口
- `diff`：CLI diff
- `brn`：批量重命名
- `cstat`：代码统计 / 扫描
- `img`：图片处理

它们的共同点是：**都被 feature gate 明确包住，说明编译体积和依赖成本被认真考虑过。**

## 5. `SubCommand` 到实现模块的直观映射

可以把它理解成一张“路由表”：

- `Acl` -> `commands/acl_cmd`
- `Config` -> `commands/app_config.rs`
- `Ctx` -> `commands/ctx.rs`
- `Env` -> `commands/env/*`
- 书签相关命令 -> `commands/bookmarks/*`
- `Delete` -> `commands/delete/*`
- `Proxy` 相关 -> `commands/proxy/*`
- `Ports/Kill/Ps/Pkill` -> `commands/ports/*`
- `Tree` -> `commands/tree/*`
- `Find` -> `commands/find.rs`
- `Backup` -> `commands/bak.rs`（模块名 `backup`）
- `Restore` -> `commands/restore.rs` + `commands/restore_core.rs`
- `Video` -> `commands/video/*`
- feature 命令 -> 各自命名的模块

如果你要追一条命令链，通常按下面顺序最快：

1. `src/cli.rs`
2. 对应 `src/cli/<module>.rs`
3. `src/commands/dispatch/*.rs`
4. 对应 `src/commands/<module>`

## 6. 与状态 / 配置相关的基础模块

理解 CLI 时，不要只看 `commands/*`，还要知道它们依赖哪些基础设施：

- `src/runtime.rs`：全局运行时选项，如 `quiet / verbose / non_interactive / no_color`
- `src/store.rs`：书签数据库与文件锁
- `src/config/*`：全局配置读写
- `src/env_core/*`：Env 子系统的核心领域逻辑
- `src/output.rs`：错误与输出样式

也就是说，`commands/*` 更多像“应用服务层”，真正的状态模型和底层逻辑并不都在命令文件里。

## 7. 推荐阅读顺序

### 7.1 想快速建立全局认识

1. `src/main.rs`
2. `src/cli.rs`
3. `src/commands/dispatch/mod.rs`
4. `src/commands/dispatch/misc.rs`

### 7.2 想先看高频日常命令

1. 书签命令族
2. `proxy`
3. `ports`
4. `tree`
5. `find`

### 7.3 想看重量级系统

1. `env`
2. `acl`
3. `redirect`
4. `diff`
5. `dashboard`

## 8. 当前实现上的几个观察

- `env` 和 `acl` 是最明显的“系统级子模块”，规模远超一般子命令。
- 书签系统已经完成了较细的职责拆分，是一个相对成熟的命令族。
- `tree`、`ports`、`proxy` 这些高频工具都已经从“一个文件写完”走向模块化拆分。
- 仓库里有不少 `*_monolith_backup_*` 和 `*_split_tmp` 文件，说明项目正在经历一轮显式的拆分重构。
- CLI 与 Dashboard 不是两个完全独立的产品，而是共享底层能力的两个入口：前者偏脚本化，后者偏可视化。
