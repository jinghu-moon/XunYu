# 扩展能力 CLI 模块导读

本文档聚焦 `xun` 里更偏“扩展能力 / 专项能力”的命令模块：`alias`、`lock`、`protect`、`crypt`、`brn`、`video`、`img`、`cstat`、`serve`。

和前两篇文档里的日常命令、工程辅助命令相比，这一组模块更有两个共同特征：

- 很多能力由 Cargo feature 控制，不一定默认出现在每个构建里。
- 模块更偏专项工具、重型工作流或外部系统集成，而不是基础日常操作。

如果你已经理解了 `bookmark / proxy / ports / tree / config / ctx / find / backup / restore / delete`，这一篇就是下一层：看项目怎样把“专项能力”接到同一套 CLI 骨架上。

## 1. 这一组模块为什么适合放在一起看

这组命令虽然主题不同，但结构上很像：

1. **很多都是 feature-gated**：例如 `alias`、`lock`、`protect`、`crypt`、`cstat`、`img`、`dashboard`、`batch_rename`。
2. **很多都属于“薄入口 + 厚引擎”**：命令入口文件很短，真正复杂度藏在 `src/alias/*`、`src/img/*`、`src/batch_rename/*`、`src/cstat/*`、`src/commands/video/*` 等子模块里。
3. **很多都带明显平台 / 工具链依赖**：例如 `lock` / `protect` / `crypt` 明显偏 Windows，`video` 依赖 `ffmpeg/ffprobe`，`img` 依赖多种编码与矢量化后端。

所以，把它们放一起看，能更容易看出 `xun` 如何承载“插件式专项能力”。

## 2. 它们的分发路径并不完全一致

这组模块主要分成两条分发链：

```text
alias / lock / protect / crypt / brn / video
  -> src/commands/dispatch/misc.rs

serve / cstat / img
  -> src/commands/dispatch/dashboard.rs
```

这里有个很值得注意的点：

- 进入 `dispatch/dashboard.rs` 的命令，不一定都意味着“Web 前端面板命令”。
- 它更多表示“相对独立、运行形态更重”的能力，例如 `serve`、`cstat`、`img`。

也就是说，`dashboard` 这个 dispatch bucket 更像“重型运行时能力分组”，不只是 Dashboard UI 本身。

## 3. 别名模块：`alias`

### 3.1 命令定义层：`src/cli/alias.rs`

`alias` 是这组命令里最像“子产品”的一个模块。它对外暴露了两棵命令树：

- Shell alias：`setup`、`add`、`rm`、`ls`、`find`、`which`、`sync`、`export`、`import`
- App alias：`app add/rm/ls/scan/which/sync`

仅从命令面就能看出，这不是简单的命令重命名，而是覆盖了：

- alias 配置存储
- shell 后端安装
- shim 生成与同步
- 应用扫描与别名注册
- 导入导出与查询

### 3.2 命令入口与真实引擎的分离

`src/commands/alias.rs` 很薄，只负责两件事：

- 调用 `crate::alias::cmd_alias(args)`
- 把领域错误映射为统一 CLI 错误

真正的复杂度在 `src/alias/*`。

### 3.3 底层引擎：`src/alias/*`

`src/alias/mod.rs` 把 alias 拆成多块非常清晰的子系统：

- `config.rs`：别名配置模型，定义 `Config`、`ShellAlias`、`AppAlias`
- `context.rs`：把 CLI 参数收敛成 `AliasCtx`
- `shell_alias_cmd.rs`：Shell alias 的 setup / add / rm / export / import
- `app_alias_cmd.rs`：App alias 的 add / rm / ls / scan
- `query.rs`：`ls`、`find`、`which`
- `sync.rs`：同步 shim、app paths、shell 集成
- `scanner/*`：扫描注册表、开始菜单、PATH 等来源的应用
- `shim_gen/*`：生成和同步 shim 文件
- `shell/*`：按 shell 后端输出运行时支持

所以 `alias` 本质上不是“命令别名列表”，而是一个 **shell 别名 + 应用别名 + shim 生成 + 应用发现** 的综合系统。

### 3.4 读这个模块时最值得抓住什么

`alias` 最值得看的不是增删改查，而是三层配合：

1. **配置层**：别名数据怎么存。
2. **发现层**：应用怎么扫描出来。
3. **落地层**：shim / shell 后端怎么把别名真正变成可执行入口。

这让它比一般 alias 功能更接近一个“命令入口编排器”。

### 3.5 Dashboard 映射

目前没有独立的 Dashboard `AliasPanel`。

因此 `alias` 目前仍然是纯 CLI / shell 工作流能力。

### 3.6 推荐阅读顺序

1. `src/cli/alias.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/alias.rs`
4. `src/alias/mod.rs`
5. `src/alias/config.rs`
6. `src/alias/context.rs`
7. `src/alias/query.rs`
8. `src/alias/shell_alias_cmd.rs`
9. `src/alias/app_alias_cmd.rs`
10. `src/alias/scanner/*`
11. `src/alias/shim_gen/*`
12. `src/alias/sync.rs`

## 4. 锁与移动模块：`lock / mv / ren`

### 4.1 命令定义层：`src/cli/lock.rs`

这组命令表面上看只是三个动作：

- `lock who`
- `mv`
- `ren`

但参数面已经暴露出它不是普通 `move` 包装：

- 支持 `--unlock`
- 支持 `--force-kill`
- 支持 `--dry-run`
- 支持 `--yes`
- 在启用 `protect` 时还支持 `--force` / `--reason`

所以它本质上是“带锁处理与保护绕过策略的移动 / 重命名执行器”。

### 4.2 执行层：`src/commands/lock/mod.rs`

`lock` 模块目前集中在一个文件，但逻辑已经很系统化：

- `cmd_lock_who()`：查看是谁占用了文件
- `cmd_mv()` / `cmd_ren_file()`：移动与重命名入口
- `unlock_and_retry()`：遇锁重试的总控逻辑
- `list_lockers_or_default()` / `kill_lockers()`：定位并处理阻塞进程
- `ensure_force_kill_authorized()`：强杀前的安全检查
- `try_open_exclusive()`：快速探测独占打开状态

### 4.3 这个模块的关键价值

真正重要的不是“移动文件”，而是它把下面几件事组合到了一起：

- 锁持有者探测
- 可选强杀阻塞进程
- 重试执行
- 与 `protect` 模块联动的保护检查
- 安全审计日志

也就是说，它是一个 **带风控和锁恢复能力的文件移动层**。

### 4.4 Dashboard 映射

目前没有独立的 Dashboard 面板。

### 4.5 推荐阅读顺序

1. `src/cli/lock.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/lock/mod.rs`

## 5. 保护模块：`protect`

### 5.1 命令定义层：`src/cli/protect.rs`

`protect` 暴露了三类动作：

- `set`
- `clear`
- `status`

其输入语义也很明确：

- `deny`：拒绝哪些动作
- `require`：绕过保护时必须满足什么条件
- `system_acl`：是否同时下探到 Windows NTFS ACL

### 5.2 执行层：`src/commands/protect.rs`

`protect` 的执行层目前集中在一个文件里，但逻辑边界很清楚：

- 规则持久化到全局配置中的 `protect.rules`
- 可选调用 `crate::windows::acl::deny_delete_access()` / `clear_deny_delete()` 做深层保护
- 每次 set / clear 都写安全审计日志
- `status` 提供表格 / JSON 状态输出

### 5.3 这个模块真正的角色

`protect` 不是一个权限系统，而是一个 **本地操作防误伤保护层**：

- 规则层面阻断高风险操作
- 必要时要求 `force + reason`
- 深层时再叠一层 NTFS ACL 防护

它和 `lock` 放一起读会很顺，因为 `lock` 的 move / rename 路径会主动调用保护检查。

### 5.4 Dashboard 映射

目前没有独立的 `ProtectPanel`。

不过 `/api/config` 后端已经支持整个全局配置对象，因此保护配置在后端配置模型里是有位置的；只是当前 Dashboard 前端没有给它单独做面板。

### 5.5 推荐阅读顺序

1. `src/cli/protect.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/protect.rs`
4. `src/config/*`
5. `src/windows/acl/*`
6. `src/security/audit/*`

## 6. 加解密模块：`crypt`

### 6.1 命令定义层：`src/cli/crypt.rs`

`crypt` 只暴露两个顶层命令：

- `encrypt`
- `decrypt`

但每个命令都支持两套后端路径：

- Windows EFS：`--efs`
- age：`--passphrase` 或 `--to/--identity`

所以它本质上不是单一加密实现，而是双后端加解密入口。

### 6.2 执行层：`src/commands/crypt.rs`

`crypt` 的分流逻辑很直接：

- 如果传 `--efs`，就走 `crate::windows::efs::*`
- 否则走 `crate::age_wrapper::*`

这个模块还做了几件很实用的工程细节：

- EFS 前先检查卷是否支持 EFS
- 文件被占用时，错误提示会引导去用 `xun lock who`
- age 加密时自动推导输出路径，如追加 `.age`
- age 解密时会尝试自动去掉 `.age` 或追加 `.decrypted`
- 加解密动作都会写安全审计日志

### 6.3 读这个模块时最值得抓住什么

`crypt` 最值得注意的是：**它并不追求统一加密抽象，而是保留了 Windows 原生 EFS 和跨平台 age 两条能力线**。

这意味着它更像“按使用场景选择后端”的工具模块，而不是加密框架。

### 6.4 Dashboard 映射

目前没有独立的 Dashboard 面板。

### 6.5 推荐阅读顺序

1. `src/cli/crypt.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/crypt.rs`
4. `src/windows/efs/*`
5. `src/age_wrapper.rs`
6. `src/security/audit/*`

## 7. 批量重命名模块：`brn`

### 7.1 命令定义层：`src/cli/batch_rename.rs`

`brn` 的设计非常典型：**默认 preview，`--apply` 才执行**。

它支持的改名模式包括：

- `--regex` + `--replace`
- `--case`
- `--prefix`
- `--suffix`
- `--strip-prefix`
- `--seq`

同时支持扩展名过滤、递归、确认跳过等执行控制。

### 7.2 执行层：`src/commands/batch_rename/mod.rs`

命令入口做了很标准的流水线控制：

- `undo` 特判：`xun brn undo`
- `resolve_mode()`：确保只激活一个改名模式
- `collect_files()`：收集目标文件
- `compute_ops()`：生成 rename 操作列表
- `detect_conflicts()`：预先发现冲突
- preview / apply / TUI 三路分发
- `write_undo()` / `run_undo()`：提供撤销能力

### 7.3 底层引擎：`src/batch_rename/*`

真正的领域逻辑在：

- `collect.rs`：收集文件
- `compute.rs`：计算每种改名模式对应的 rename ops
- `conflict.rs`：冲突检测
- `types.rs`：`CaseStyle`、`RenameOp` 等类型
- `undo.rs`：撤销记录与回滚执行

这是一种很典型的“先算计划，再执行计划”的批处理工具结构。

### 7.4 Dashboard 映射

目前没有独立面板。

### 7.5 推荐阅读顺序

1. `src/cli/batch_rename.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/batch_rename/mod.rs`
4. `src/batch_rename/collect.rs`
5. `src/batch_rename/compute.rs`
6. `src/batch_rename/conflict.rs`
7. `src/batch_rename/undo.rs`
8. `src/commands/batch_rename/tui.rs`

## 8. 视频模块：`video`

### 8.1 命令定义层：`src/cli/video.rs`

`video` 对外暴露三类动作：

- `probe`
- `compress`
- `remux`

这三个动作分别代表：

- 看媒体信息
- 有损转码压缩
- 无损换封装

从命令面就能看出它是围绕 `ffmpeg/ffprobe` 设计的媒体工具壳。

### 8.2 执行层：`src/commands/video/*`

`src/commands/video/mod.rs` 只是命令分发，真正逻辑拆在多个文件：

- `probe.rs`：媒体探测
- `compress.rs`：压缩转码
- `remux.rs`：封装转换
- `ffmpeg.rs`：外部二进制定位、probe 和 ffmpeg 调用
- `plan.rs`：压缩策略与尝试计划生成
- `common.rs`：输入输出检查、容器兼容检查
- `types.rs`：`ProbeSummary` 等领域类型
- `error.rs`：视频领域错误

### 8.3 这个模块最值得看的设计点

`video` 最值得看的不是调用 ffmpeg 本身，而是 **在真正执行前先做规划与兼容性判断**：

- `compress` 会根据模式、引擎、容器生成尝试计划
- `remux` 在 strict 模式下会先用 probe 结果校验容器 / 编码兼容性
- `ffmpeg.rs` 把二进制定位和 stderr 摘要提取单独收口

这让 `video` 比“直接拼命令字符串”的脚本更像一个受控媒体编排层。

### 8.4 Dashboard 映射

目前没有独立面板。

### 8.5 推荐阅读顺序

1. `src/cli/video.rs`
2. `src/commands/dispatch/misc.rs`
3. `src/commands/video/mod.rs`
4. `src/commands/video/probe.rs`
5. `src/commands/video/compress.rs`
6. `src/commands/video/remux.rs`
7. `src/commands/video/common.rs`
8. `src/commands/video/plan.rs`
9. `src/commands/video/ffmpeg.rs`
10. `src/commands/video/types.rs`

## 9. 图像处理模块：`img`

### 9.1 命令定义层：`src/cli/img.rs`

`img` 的 CLI 面非常宽，说明它不是一个单格式转换器，而是一个图像处理管线入口：

- 输入 / 输出路径
- 输出格式：`jpeg/png/webp/avif/svg`
- SVG 矢量化方法：`bezier / visioncortex / potrace / skeleton / diffvg`
- JPEG / PNG / WebP / AVIF 各自的编码参数
- 尺寸限制、线程数、覆盖策略

### 9.2 命令入口与底层引擎的分离

`src/commands/img.rs` 主要负责：

- 参数合法性校验
- 字符串到类型的解析
- 输入文件收集
- 组装 `ProcessParams`
- 调用 `img::process::run()`
- 输出汇总报告

真正的底层引擎在 `src/img/*`。

### 9.3 底层引擎：`src/img/*`

从目录结构看，`img` 是整个仓库里很重的一块：

- `collect.rs`：收集输入图片
- `decode.rs`：读取与解码
- `process.rs`：整体处理流水线
- `encode/*`：按格式编码输出
- `vector/*`：SVG 矢量化的多个后端实现
- `report.rs`：汇总和性能阶段统计
- `types.rs`：`OutputFormat`、`SvgMethod`、`JpegBackend`、`ProcessParams`、`ProcessResult`

尤其 `types.rs` 里的 `StageDurationsMs` 很能说明问题：这块不只是做功能结果，还非常关心阶段级耗时。

### 9.4 这个模块最值得抓住什么

`img` 是一个 **多格式、多编码器、多矢量化后端** 的图像处理平台，而不是“小图转 webp”命令。

如果你要快速建立认知，建议先抓住三层：

1. 收集输入：`collect.rs`
2. 统一执行：`process.rs`
3. 多后端输出：`encode/*` 与 `vector/*`

### 9.5 Dashboard 映射

目前没有独立 Web 面板。

虽然它和 `serve / cstat` 一样走 `dispatch/dashboard.rs`，但这不意味着它已经接了 Dashboard 前端；这里只说明它被归为“较重型、独立”的命令类别。

### 9.6 推荐阅读顺序

1. `src/cli/img.rs`
2. `src/commands/dispatch/dashboard.rs`
3. `src/commands/img.rs`
4. `src/img/mod.rs`
5. `src/img/types.rs`
6. `src/img/collect.rs`
7. `src/img/process.rs`
8. `src/img/encode/*`
9. `src/img/vector/*`
10. `src/img/report.rs`

## 10. 代码统计模块：`cstat`

### 10.1 命令定义层：`src/cli/cstat.rs`

`cstat` 不只是统计代码行数，它还叠加了项目清理扫描能力：

- `empty`
- `large`
- `dup`
- `tmp`
- `all`
- 扩展名过滤、深度限制、输出格式、JSON 导出

所以它更准确的定位是：**代码统计 + 清理问题扫描器**。

### 10.2 执行层：`src/commands/cstat/*`

命令入口的控制流很清晰：

- `collect_files()`：基于 `ignore::WalkBuilder` 收集文件
- 并行扫描文件内容
- 调用 `crate::cstat::scanner::scan_bytes()` 做语言级统计
- 汇总 issues：空文件、大文件、重复文件、临时文件
- `render_output()`：按 `json/table/auto` 路由输出
- 在 `auto` 且交互环境、并且存在 issue 时，可进入 TUI

### 10.3 底层引擎：`src/cstat/*`

真正领域逻辑在：

- `lang.rs`：语言规则与临时文件规则
- `scanner.rs`：逐文件统计，包含对 Vue 文件的特殊处理
- `report.rs`：`LangStat`、`Issues`、`Report` 聚合

这说明 `cstat` 不是简单 `cloc` 包装，而是带项目卫生检查的扫描器。

### 10.4 Dashboard 映射

目前没有独立的 Dashboard 前端面板。

`cstat` 走 `dispatch/dashboard.rs`，主要是因为它支持更重的运行形态，例如 TUI，而不是因为它已经有 Web 工作台。

### 10.5 推荐阅读顺序

1. `src/cli/cstat.rs`
2. `src/commands/dispatch/dashboard.rs`
3. `src/commands/cstat/mod.rs`
4. `src/cstat/lang.rs`
5. `src/cstat/scanner.rs`
6. `src/cstat/report.rs`
7. `src/commands/cstat/render.rs`
8. `src/commands/cstat/tui/*`

## 11. Dashboard 服务入口：`serve`

### 11.1 命令定义层：`src/cli/dashboard.rs`

`serve` 的命令面非常薄，只定义监听端口。

但它的重要性不在参数数量，而在于它是整个 Web Dashboard 的 CLI 入口。

### 11.2 执行层：`src/commands/dashboard/mod.rs`

`cmd_serve()` 做的是完整的服务引导：

- 构建 tokio runtime
- 构建 axum router
- 挂载 `/api/*` 路由
- 挂载静态资源和 SPA fallback
- 在启用 `diff` 时构建额外的文件浏览 / diff / convert / validate / ws 能力
- 启动 env 自动快照调度器
- 监听本地端口并提供服务

### 11.3 这个模块最值得抓住什么

`serve` 不是一个小壳，而是 **把 CLI 世界和 Dashboard 世界接起来的边界模块**。

如果你已经读过 `Dashboard-Components.md`，再回头看 `cmd_serve()`，会更容易理解：

- 前端为什么能访问 `/api/config`、`/api/bookmarks`、`/api/proxy`、`/api/ports`
- `diff` feature 为什么会影响 Dashboard 能力面
- 为什么环境子系统还能挂自动快照调度器

### 11.4 推荐阅读顺序

1. `src/cli/dashboard.rs`
2. `src/commands/dispatch/dashboard.rs`
3. `src/commands/dashboard/mod.rs`
4. `src/commands/dashboard/handlers/*`
5. `dashboard-ui/src/App.vue`
6. `intro/dashboard/Dashboard-Components.md`

## 12. 把这些模块放在一起看，会更容易看懂什么

### 12.1 `xun` 的扩展能力组织方式

这组模块一起看，你会发现项目对“专项能力”有一套非常稳定的处理方式：

- 命令面始终接在同一套 `argh + dispatch` 骨架上
- 复杂度尽量下沉到独立目录或领域模块
- 平台相关逻辑单独封装，不污染 CLI 面
- 交互式形态（TUI / Web）通常叠在底层能力之上，而不是重新实现一套

### 12.2 `dispatch/dashboard.rs` 的真正含义

这一轮最值得建立的认知之一是：

- `dispatch/misc.rs`：常规命令族
- `dispatch/env.rs`：环境 / 会话类命令
- `dispatch/dashboard.rs`：较重、较独立的运行时命令

第三类不等于“有 Web 面板”，而更接近“单独运行形态”。

### 12.3 这些模块的成熟度差异

- `alias`、`img`、`video`：像完整领域工具
- `lock`、`protect`、`crypt`：像平台能力集成层
- `brn`、`cstat`：像交互友好的批处理 / 扫描工具
- `serve`：像整个 Dashboard 的运行时边界

这种差异能帮助你后续判断：读一个模块时到底该优先看 CLI 面、底层引擎、还是运行时装配。

## 13. 推荐整体阅读顺序

如果你准备继续按“从专项工具到系统边界”的顺序理解项目，建议：

1. `alias`
2. `lock`
3. `protect`
4. `crypt`
5. `brn`
6. `video`
7. `img`
8. `cstat`
9. `serve`

这个顺序的好处是：先读相对贴近本地终端工作流的工具，再读媒体 / 图像重模块，最后回到 Dashboard 服务入口。

## 14. 当前实现上的几个观察

- `alias` 已经明显超出“命令别名”范畴，更像一个入口编排与 shim 生成系统。
- `lock` 和 `protect` 之间存在明确联动，说明项目对文件安全操作是按体系设计的。
- `crypt` 采用 EFS 与 age 双后端并存的策略，非常实用主义。
- `brn`、`cstat` 都采用“先计划 / 先扫描，再执行 / 再渲染”的模式，工程味很重。
- `img` 和 `video` 都不是玩具命令，尤其 `img` 的模块规模已经接近独立工具。
- `serve` 是把大量底层能力真正暴露给 Dashboard 的边界层，单看前端组件很难替代对它的理解。
