# xun 命令文档

**全局约定**
- `stdout` 输出机器可读内容，`stderr` 输出交互 UI 与表格。
- `XUN_UI=1` 强制表格输出（即便被管道重定向）。
- 所有命令支持 `--help` 查看参数说明。
- 全局选项：`--no-color`（或 `NO_COLOR=1`）、`--version`、`-q/--quiet`、`-v/--verbose`、`--non-interactive`。
- 对应环境变量：`XUN_QUIET`、`XUN_VERBOSE`、`XUN_NON_INTERACTIVE`。

---

## Shell 集成

| 命令 | 说明 | 备注 |
| --- | --- | --- |
| `xun init powershell` | 输出 PowerShell 集成脚本 | 配合 `Invoke-Expression` 执行 |
| `xun init bash` | 输出 Bash 集成脚本 | 适用于 Git Bash/MSYS2 |
| `xun init zsh` | 输出 Zsh 集成脚本 | 适用于 Zsh |

---

## 补全（Completion）

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun __complete <args...>` | 内部补全入口（Shell 已分词参数）。 | - |
| `xun completion <shell>` | 生成 Shell 补全脚本。 | - |

---

## 配置管理

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun config edit` | 在编辑器中打开配置文件。 | - |
| `xun config get <key>` | 按点路径读取配置值（如 proxy.defaultUrl）。 | - |
| `xun config set <key> <value>` | 按点路径写入配置值（如 tree.defaultDepth 3）。 | - |

---

## Context Switch（ctx）

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun ctx del <name>` | 删除 profile。 | - |
| `xun ctx list` | 列出 profile 列表。 | `--format <auto|table|tsv|json>` |
| `xun ctx off` | 停用当前 profile。 | - |
| `xun ctx rename <old> <new>` | 重命名 profile。 | - |
| `xun ctx set <name>` | 定义或更新上下文 profile。 | `--path <path>`；`--proxy <url|off|keep>`；`--noproxy <url>`；`--tag <tag>`；`--env <env>`；`--env-file <env-file>` |
| `xun ctx show [name]` | 显示 profile 详情（默认当前激活）。 | `--format <auto|table|tsv|json>` |
| `xun ctx use <name>` | 激活上下文 profile。 | - |

---

## 书签命令

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun bookmark z [keywords...]` | 跳转到 Top-1 书签。 | `--list`；`--score`；`--why`；`--preview`；`--tag`；`--global`；`--child`；`--base`；`--workspace`；`--json`；`--tsv` |
| `xun bookmark zi [keywords...]` | 交互式跳转。 | 与 `z` 共享 query 选项；非交互模式回落 Top-1 |
| `xun bookmark o [keywords...]` | 用文件管理器打开 Top-1。 | 与 `z` 共享 query 选项 |
| `xun bookmark oi [keywords...]` | 交互式打开。 | 与 `zi` 共享行为 |
| `xun bookmark open [keywords...]` | `o` 的长命令形式。 | 与 `o` 等价 |
| `xun bookmark save [name]` | 保存当前目录为显式书签。 | `--tag <tag>`；`--desc <text>`；`-w/--workspace <name>` |
| `xun bookmark set <name> [path]` | 保存当前目录或指定路径为显式书签。 | `--tag <tag>`；`--desc <text>`；`-w/--workspace <name>` |
| `xun bookmark tag add|remove|list|rename ...` | 书签标签管理。 | - |
| `xun bookmark pin <name>` | 置顶显式书签。 | - |
| `xun bookmark undo` | 撤销最近的书签变更。 | `--steps <n>` |
| `xun bookmark redo` | 重做最近撤销的书签变更。 | `--steps <n>` |
| `xun bookmark rename <old> <new>` | 重命名显式书签。 | - |
| `xun bookmark list` | 列出书签。 | `--tag <tag>`；`--sort <name|last|visits>`；`--limit <limit>`；`--offset <offset>`；`--reverse`；`--tsv`；`--format <auto|table|tsv|json>` |
| `xun bookmark recent` | 显示最近访问书签。 | `--limit <limit>`；`--tag <tag>`；`--workspace <name>`；`--since <duration>`；`--format <auto|table|tsv|json>` |
| `xun bookmark stats` | 显示统计信息。 | `--format <auto|table|tsv|json>` |
| `xun bookmark check` | 检查书签健康（缺失/重复/过期）。 | `--days <days>`；`--format <auto|table|tsv|json>` |
| `xun bookmark gc` | 清理无效路径。 | `--purge`；`--dry-run`；`--learned`；`--format <auto|table|tsv|json>` |
| `xun bookmark dedup` | 书签去重。 | `--mode <path|name>`；`--format <auto|table|tsv|json>`；`--yes` |
| `xun bookmark export` | 导出书签。 | `--format <json|tsv>`；`--out <path>`；轻量交换格式，保留 `workspace`，不是全量备份 |
| `xun bookmark import` | 导入书签或外部导航数据。 | `--format <json|tsv>`；`--from <autojump|zoxide|z|fasd|history>`；`--input <path>`；`--mode <merge|overwrite>`；`--yes`；原生 `json/tsv` 会读取 `workspace` |
| `xun bookmark learn --path <path>` | 手动记录一次目录访问。 | 受 `bookmark.autoLearn.enabled` 与 `_BM_EXCLUDE_DIRS` 控制 |
| `xun bookmark init <shell>` | 生成 bookmark shell 集成脚本。 | `--cmd <prefix>` |
| `xun bookmark touch <name>` | 更新显式书签访问频次。 | - |
| `xun bookmark keys` | 输出所有具名书签名（补全用）。 | - |
| `xun bookmark all [tag]` | 机器输出所有书签。 | - |

删除书签请使用 `xun del -bm <name>` 或 `xun delete -bm <name>`。

Scope 说明：
- `-g/--global`：全局候选，不参考当前目录上下文。
- `-c/--child`：优先当前目录及其子目录。
- `--base <path>`：限制到指定基路径下。
- `-w/--workspace <name>`：限制到指定 workspace。

---

## 代理命令

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun poff` | 关闭代理（poff）。 | `--msys2 <msys2>` |
| `xun pon [url]` | 开启代理（pon）。 | `--no-test`；`--noproxy <url>`；`--msys2 <msys2>` |
| `xun proxy del` | 删除代理。 | `--msys2 <msys2>`；`--only <only>` |
| `xun proxy detect` | 检测系统代理。 | `--format <auto|table|tsv|json>` |
| `xun proxy get` | 读取当前 git 代理配置。 | - |
| `xun proxy set <url>` | 设置代理。 | `--noproxy <url>`；`--msys2 <msys2>`；`--only <only>` |
| `xun proxy test <url>` | 测试代理延迟。 | `--targets <targets>`；`--timeout <timeout>`；`--jobs <jobs>` |
| `xun pst` | 代理状态（pst）。 | `--format <auto|table|tsv|json>` |
| `xun px <cmd...>` | 代理执行（px）。 | `--url <url>`；`--noproxy <url>` |

---

## 端口命令

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun kill <ports>` | 终止占用端口的进程。 | `--force`；`--tcp`；`--udp` |
| `xun ports` | 列出监听端口（默认 TCP）。 | `--all`；`--udp`；`--range <start-end>`；`--pid <pid>`；`--name <name>`；`--format <auto|table|tsv|json>` |

---

## 进程命令

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun ps [pattern]` | 列出或筛选进程。 | `--pid <pid>`；`-w/--win <window-title>` |
| `xun pkill <target>` | 按名称/PID/窗口标题终止进程。 | `-w/--window`；`-f/--force` |

---

## 备份命令

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun backup <op-args...>` | 增量项目备份。`xun bak` 为别名。 | `list`；`verify <name>`；`find [tag]`；`--msg <msg>`；`--dir <path>`；`--dry-run`；`--no-compress`；`--retain <retain>`；`--include <include>`；`--exclude <exclude>`；`--incremental`；`--skip-if-unchanged`；配置项 `skipIfUnchanged` |
| `xun restore <name-or-path>` | 从备份恢复文件。`xun rst` 为别名。 | `--file <path>`；`--glob <pattern>`；`--to <path>`；`--snapshot`；`--dir <path>`；`--dry-run`；`-y/--yes` |

---

## 目录树命令

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun tree [path]` | 生成目录树。 | `--depth <depth>`；`--output <path>`；`--hidden`；`--no-clip`；`--plain`；`--stats-only`；`--fast`；`--sort <name|mtime|size>`；`--size`；`--max-items <max-items>`；`--include <include>`；`--exclude <exclude>` |

---

## Find 命令

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun find [paths...]` | 按模式/元数据查找文件与目录。 | `--include/-i`；`--exclude/-e`；`--regex-include`；`--regex-exclude`；`--extension`；`--not-extension`；`--name`；`--size`；`--mtime/--ctime/--atime`；`--depth`；`--attribute`；`--empty-files`；`--empty-dirs`；`--case`；`--count`；`--format <auto|table|tsv|json>` |

---

## ACL 权限命令

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun acl view -p <path>` | 查看 ACL 摘要/明细。 | `--detail`；`--export <csv>` |
| `xun acl add -p <path>` | 添加 ACL 规则。 | `--principal`；`--rights`；`--ace-type`；`--inherit`；`-y/--yes` |
| `xun acl remove -p <path>` | 删除显式 ACL 规则（交互多选）。 | - |
| `xun acl purge -p <path>` | 清理某主体的全部显式规则。 | `--principal`；`-y/--yes` |
| `xun acl diff -p <path> -r <ref>` | 比较两路径 ACL 差异。 | `-o/--output <csv>` |
| `xun acl batch` | 批量 ACL 操作。 | `--file`；`--paths`；`--action <repair|backup|orphans|inherit-reset>`；`--output`；`-y/--yes` |
| `xun acl effective -p <path>` | 查看用户在目标路径上的有效权限。 | `-u/--user <user>` |
| `xun acl copy -p <path> -r <ref>` | 把参考路径 ACL 覆盖到目标。 | `-y/--yes` |
| `xun acl backup -p <path>` | 备份 ACL 到 JSON。 | `-o/--output <json>` |
| `xun acl restore -p <path> --from <json>` | 从 JSON 恢复 ACL。 | `-y/--yes` |
| `xun acl inherit -p <path>` | 开关继承。 | `--disable`；`--enable`；`--preserve <bool>` |
| `xun acl owner -p <path>` | 查看或修改所有者。 | `--set <principal>`；`-y/--yes` |
| `xun acl orphans -p <path>` | 扫描/清理孤儿 SID。 | `--recursive <bool>`；`--action <none|export|delete|both>`；`--output`；`-y/--yes` |
| `xun acl repair -p <path>` | 强制修复 ACL（接管所有权并授予控制权）。 | `--export-errors`；`-y/--yes` |
| `xun acl audit` | 查看 ACL 审计日志。 | `--tail <n>`；`--export <csv>` |
| `xun acl config` | 查看或修改 ACL 配置。 | `--set KEY VALUE` |

---

## 文件删除（delete/del）

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun delete [path]` | 删除文件（默认仅匹配 Windows 保留名）。 | `--any` 允许任意文件名；`--dry-run/--what-if`；`--on-reboot`；`--format <auto|table|tsv|json>` |
| `xun del [path]` | `delete` 的别名。 | 同上 |

删除书签请使用 `-bm/--bookmark`。

---

## Redirect 分类引擎（feature: `redirect`）

> 需 `--features redirect` 编译。规则存储在 `~/.xun.config.json` 的 `redirect.profiles.<profile>`。

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun redirect [source]` | 按规则将目录文件分类到子目录。 | `--profile <profile>`；`--explain <explain>`；`--stats`；`--confirm`；`--review`；`--log`；`--tx <tx>`；`--last <last>`；`--validate`；`--plan <plan>`；`--apply <apply>`；`--undo <undo>`；`--watch`；`--status`；`--simulate`；`--dry-run`；`--copy`；`--yes`；`--format <auto|table|tsv|json>` |

Watch 调优环境变量：`XUN_REDIRECT_WATCH_DEBOUNCE_MS`、`XUN_REDIRECT_WATCH_SETTLE_MS`、`XUN_REDIRECT_WATCH_RETRY_MS`、`XUN_REDIRECT_WATCH_SCAN_RECHECK_MS`、`XUN_REDIRECT_WATCH_MAX_BATCHES`、`XUN_REDIRECT_WATCH_MAX_PATHS`、`XUN_REDIRECT_WATCH_MAX_RETRY_PATHS`、`XUN_REDIRECT_WATCH_MAX_SWEEP_DIRS`、`XUN_REDIRECT_WATCH_SWEEP_MAX_DEPTH`；网络共享（UNC）约束：`nBufferLength <= 64KB`。

---

## 文件解锁与操作（feature: `lock`）

> 需 `--features lock` 编译。`rm`/`mv`/`ren` 为文件系统操作；`delete`/`del` 默认仅处理保留名（`--any` 允许任意文件名），书签删除请用 `-bm/--bookmark`。

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun lock who <path>` | 显示占用文件的进程。 | `--format <auto|table|tsv|json>` |
| `xun mv <src> <dst>` | 移动文件或目录。 | `--unlock`；`--force-kill`；`--dry-run`；`--yes`；`--force`；`--reason <reason>` |
| `xun ren <src> <dst>` | 重命名文件或目录。 | `--unlock`；`--force-kill`；`--dry-run`；`--yes`；`--force`；`--reason <reason>` |
| `xun rm <path>` | 删除文件或目录。 | `--unlock`；`--force-kill`；`--on-reboot`；`--dry-run`；`--yes`；`--format <auto|table|tsv|json>`；`--force`；`--reason <reason>` |

退出码：`0` 成功 / `2` 参数错误 / `3` 权限不足 / `10` 占用未授权 / `11` 解锁失败 / `20` 已登记重启

---

## 防误操作保护（feature: `protect`）

> 需 `--features protect` 编译。规则存储在 `~/.xun.config.json` 的 `protect.rules` 字段。

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun protect clear <path>` | 清除保护规则。 | `--system-acl` |
| `xun protect set <path>` | 设置保护规则。 | `--deny <deny>`；`--require <require>`；`--system-acl` |
| `xun protect status [path]` | 显示保护状态。 | `--format <auto|table|tsv|json>` |

---

## 文件加密（feature: `crypt`）

> 需 `--features crypt` 编译。`--efs` 走 Windows EFS 系统加密，否则走 age 应用层加密。

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun decrypt <path>` | 解密文件。 | `--efs`；`--identity <identity>`；`--passphrase`；`--out <path>` |
| `xun encrypt <path>` | 使用 Windows EFS 加密文件（或其他提供者）。 | `--efs`；`--to <to>`；`--passphrase`；`--out <path>` |

---

## Dashboard（feature: `dashboard`）

> 需 `--features dashboard` 编译。

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun serve` | 启动 Web Dashboard 服务。 | `--port <port>` |

补充说明：
- 若同时编译 `--features "dashboard,diff"`，Dashboard 会额外启用 Diff 文件管理 API：`/api/files`、`/api/files/search`、`/api/info`、`/api/content`、`/api/diff`、`/api/convert`、`/api/validate`、`/ws`。
- `xun serve` 当前仅绑定 `127.0.0.1`。

---

## Diff（feature: `diff`）

> 需 `--features diff` 编译。支持 `CLI` 直接对比；若同时启用 `dashboard`，可在 Web 端使用可视化文件管理与 Diff。

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun diff <old> <new>` | 比较两个文件差异。 | `--mode <auto|line|ast>`；`--diff-algorithm <histogram|myers|minimal|patience>`；`--format <text|json>`；`--context <n>`；`--max-size <512K|1M|2G...>`；`--ignore-space-change`；`--ignore-all-space`；`--ignore-blank-lines`；`--strip-trailing-cr`；`--text` |

---

## Alias（feature: `alias`）

> 需 `--features alias` 编译；若需要更完整 shell 集成可加 `alias-shell-extra`。

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun alias setup` | 安装 alias 运行时（shim + shell）。 | `--no-cmd`；`--no-ps`；`--no-bash`；`--no-nu`；`--core-only` |
| `xun alias add <name> <command>` | 添加命令别名。 | `--mode <auto|exe|cmd>`；`--desc`；`--tag`；`--shell`；`--force` |
| `xun alias rm <names...>` | 删除命令别名。 | - |
| `xun alias ls` | 列别名。 | `--type <cmd|app>`；`--tag`；`--json` |
| `xun alias find <keyword>` | 模糊查找别名。 | - |
| `xun alias which <name>` | 查看别名目标与 shim 信息。 | - |
| `xun alias sync` | 同步 shim / app paths / shell。 | - |
| `xun alias export` | 导出 alias 配置。 | `-o/--output <file>` |
| `xun alias import <file>` | 导入 alias 配置。 | `--force` |
| `xun alias app ...` | app alias 子命令（`add/rm/ls/scan/which/sync`）。 | `scan` 支持 `--source <reg|startmenu|path|all>`、`--filter`、`--all`、`--no-cache` |

---

## Batch Rename（feature: `batch_rename`）

> 需 `--features batch_rename` 编译。默认 dry-run，执行需加 `--apply`。

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun brn [path]` | 批量重命名。 | `--regex/--replace`；`--case <kebab|snake|pascal|upper|lower>`；`--prefix`；`--suffix`；`--strip-prefix`；`--seq`；`--start`；`--pad`；`--ext`；`-r/--recursive`；`--apply`；`-y/--yes` |

---

## Cstat（feature: `cstat`）

> 需 `--features cstat` 编译。

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun cstat [path]` | 代码统计与项目清理扫描。 | `--empty`；`--large <n>`；`--dup`；`--tmp`；`--all`；`--ext`；`--depth`；`--format <auto|table|json>`；`-o/--output` |

---

## Img（feature: `img`）

> 需 `--features img` 编译；JPEG 编码后端需额外启用 `img-moz` 或 `img-turbo`。
> 可选 `--features img-zen` 启用 AVIF 解码实验后端（zenavif，AGPL 许可证需单独评估）。
> AVIF 解码后端可通过环境变量 `XUN_AVIF_BACKEND=<auto|dll|zen|image>` 切换，默认 `auto`（优先 `dll`，再尝试 `zen`，最后回退 `image`）。

| 命令 | 说明 | 选项/备注 |
| --- | --- | --- |
| `xun img -i <input> -o <output>` | 图像压缩与格式转换（文件/目录）。 | `-f/--format <jpeg|png|webp|avif|svg>`；`--svg-method <bezier|visioncortex|potrace|skeleton|diffvg>`；`--svg-diffvg-iters`；`--svg-diffvg-strokes`；`--jpeg-backend <auto|moz|turbo>`；`-q/--quality`；`--png-lossy`；`--png-dither-level`；`--webp-lossy`；`--mw`；`--mh`；`-t/--threads`；`--avif-threads`；`--overwrite` |

---

## Shell wrapper 别名

`xun init` 输出的脚本中包含：
`x`、`ctx`、`bm`、`z`、`zi`、`o`、`oi`、`delete`、`pon`、`poff`、`pst`、`px`、`bak`、`xtree`、`xr`、`redir`。
其中 `xtree` 用于避免覆盖系统 `tree` 命令；`z / zi / o / oi` 由 bookmark wrapper 提供，等价于 `xun bookmark ...`。
