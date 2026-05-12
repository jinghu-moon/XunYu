# XunYu CLI 现状说明

> 版本: 2.0
> 日期: 2026-05-12
> 关联: [CLI 重构方案](./CLI-Refactor-Plan.md)
> 审核状态: 已审核，融入架构建议

---

## 一、全局概览

### 1.1 基本信息

| 项目 | 值 |
|------|-----|
| CLI 库 | argh 0.1.x |
| 命令风格 | `xun <command> [<args>]` |
| 描述 | `bookmark + proxy CLI` (已过时) |
| Shell 补全 | 自己实现 (powershell/bash/zsh) |
| 输出格式 | 分散实现 (table/json/tsv) |
| Feature gate | 12 个模块 |

### 1.2 顶层全局参数

```
xun [--no-color] [--version] [-q] [-v] [--non-interactive] <command>
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `--no-color` | switch | 禁用 ANSI 颜色 |
| `--version` | switch | 显示版本 |
| `-q, --quiet` | switch | 静默模式 |
| `-v, --verbose` | switch | 详细输出 |
| `--non-interactive` | switch | 强制非交互模式 |

---

## 二、命令清单

### 2.1 核心命令 (始终可用)

| 命令 | 别名 | 说明 | 子命令数 |
|------|------|------|----------|
| `bookmark` | `bm` | 书签管理与导航 | 26 |
| `proxy` | — | 代理管理 | 5 |
| `pon` | — | 代理开启 (快捷) | 0 |
| `poff` | — | 代理关闭 (快捷) | 0 |
| `pst` | — | 代理状态 (快捷) | 0 |
| `px` | — | 代理执行 (快捷) | 0 |
| `env` | — | 环境变量管理 | 27 |
| `acl` | — | Windows ACL 管理 | 15 |
| `config` | — | 配置管理 | 3 |
| `ctx` | — | 上下文切换 | 7 |
| `ports` | — | 端口列表 | 0 |
| `kill` | — | 按端口杀进程 | 0 |
| `ps` | — | 进程列表 | 0 |
| `pkill` | — | 按名称/PID 杀进程 | 0 |
| `backup` | `bak` | 增量备份 | 6 |
| `tree` | — | 目录树 | 0 |
| `find` | — | 文件查找 | 0 |
| `delete` | `del` | 强制删除 | 0 |
| `rm` | — | 删除文件/目录 | 0 |
| `video` | — | 视频操作 | 3 |
| `init` | — | Shell 集成初始化 | 0 |
| `completion` | — | Shell 补全脚本 | 0 |
| `__complete` | — | 内部补全入口 | 0 |

### 2.2 Feature-Gated 命令

| 命令 | Feature | 说明 | 子命令数 |
|------|---------|------|----------|
| `alias` | `alias` | 命令别名管理 | 11 |
| `brn` | `batch_rename` | 批量重命名 | 0 (单命令) |
| `encrypt` | `crypt` | 文件加密 | 0 |
| `decrypt` | `crypt` | 文件解密 | 0 |
| `vault` | `crypt` | 文件保险库 | 8 |
| `protect` | `protect` | 保护规则 | 3 |
| `img` | `img` | 图片压缩/转换 | 0 (单命令) |
| `desktop` | `desktop` | 桌面管理 | 大量 |
| `serve` | `dashboard` | 仪表盘服务 | 0 |
| `diff` | `diff` | 差异比较 | 0 |
| `redirect` | `redirect` | 重定向 | 0 |
| `lock` | `lock` | 文件锁 | 3 |
| `mv` | `lock` | 移动文件 | 0 |
| `renfile` | `lock` | 重命名文件 | 0 |
| `rm` | `fs` | 删除 (增强版) | 0 |
| `verify` | `xunbak` | 备份验证 | 0 |
| `xunbak` | `xunbak` | xun 备份工具 | 5 |

---

## 三、主要命令详解

### 3.1 bookmark — 书签管理 (26 个子命令)

```
xun bookmark <command>
```

| 子命令 | 别名 | 说明 | 参数特点 |
|--------|------|------|----------|
| `z` | `go`, `j` | 模糊跳转 | patterns, --tag, --list, --score, --json, --tsv, --global, --child, --base, --workspace, --preset |
| `zi` | — | 交互式跳转 | 同 z |
| `o` | — | 资源管理器打开 | 同 z |
| `oi` | — | 交互式打开 | 同 z |
| `open` | — | 文件管理器打开 | 同 z |
| `save` | — | 保存当前目录 | name (可选) |
| `set` | — | 设置书签 | name, path |
| `delete` | `rm` | 删除书签 | name |
| `list` | `ls` | 列出所有 | --tag, --sort, --limit, --offset, --reverse, --tsv, --format |
| `recent` | — | 最近使用 | --limit |
| `stats` | — | 统计信息 | — |
| `check` | — | 健康检查 | — |
| `gc` | — | 清理死链 | — |
| `dedup` | — | 去重 | — |
| `export` | — | 导出 | --format |
| `import` | — | 导入 | path |
| `rename` | — | 重命名 | old, new |
| `pin` | — | 固定 | name |
| `unpin` | — | 取消固定 | name |
| `tag` | — | 标签管理 | 子命令组 |
| `undo` | — | 撤销 | — |
| `redo` | — | 重做 | — |
| `init` | — | Shell 集成 | shell 类型 |
| `learn` | — | 记录访问 | path |
| `touch` | — | 更新频率 | name |
| `keys` | — | 列出所有键 | — |
| `all` | — | 全部输出 | — |

**问题:**
- `z`/`zi`/`o`/`oi` 参数完全重复，应该复用
- `save`/`set` 功能重叠
- `delete` 和 `rm` 都存在
- 输出格式参数不统一 (`--json`/`--tsv` vs `--format json`)

### 3.2 proxy — 代理管理 (5 个子命令 + 4 个快捷命令)

```
xun proxy <command>
```

| 子命令 | 说明 | 参数 |
|--------|------|------|
| `set` | 设置代理 | url, --noproxy, --msys2, --only |
| `del` | 删除代理 | --msys2, --only |
| `get` | 获取当前代理 | — |
| `detect` | 检测系统代理 | --format |
| `test` | 测试延迟 | url, --targets, --timeout, --jobs |

**快捷命令 (顶层):**

| 命令 | 等价于 | 参数 |
|------|--------|------|
| `pon` | `proxy on` | url, --no-test, --noproxy, --msys2 |
| `poff` | `proxy off` | --msys2 |
| `pst` | `proxy status` | --format |
| `px` | `proxy exec` | --url, --noproxy, cmd... |

**问题:**
- `pon`/`poff`/`pst`/`px` 作为顶层命令污染命名空间
- `proxy on`/`proxy off`/`proxy status`/`proxy exec` 不存在，只有快捷命令
- `detect` 用 `--format`，`test` 不用，不一致

### 3.3 env — 环境变量管理 (27 个子命令)

```
xun env <command>
```

| 子命令 | 说明 | 参数特点 |
|--------|------|----------|
| `status` | 状态概览 | — |
| `list` | 列出变量 | --scope, --format |
| `search` | 搜索 | pattern |
| `get` | 获取单个 | name |
| `set` | 设置单个 | name, value |
| `del` | 删除单个 | name |
| `check` | 检查 | — |
| `path` | PATH 操作 | 子命令组 |
| `path-dedup` | PATH 去重 | — |
| `snapshot` | 快照操作 | 子命令组 |
| `doctor` | 健康检查 | — |
| `profile` | 配置管理 | 子命令组 |
| `batch` | 批量操作 | 子命令组 |
| `apply` | 应用配置 | name |
| `export` | 导出 | --format |
| `export-all` | 导出全部 | — |
| `export-live` | 导出实时 | — |
| `env` | 合并输出 | — |
| `import` | 导入 | path |
| `diff-live` | 差异比较 | baseline |
| `graph` | 依赖图 | — |
| `validate` | 验证 | — |
| `schema` | Schema 管理 | 子命令组 |
| `annotate` | 注解管理 | 子命令组 |
| `config` | 配置 | 子命令组 |
| `audit` | 审计日志 | — |
| `watch` | 监听变化 | — |
| `template` | 模板展开 | template |
| `run` | 运行命令 | cmd... |
| `tui` | TUI 面板 | — |

**问题:**
- 27 个子命令过多，应分组
- `check`/`doctor` 功能重复
- `export`/`export-all`/`export-live`/`env` 都是导出，混乱
- `list` 用 `--format`，其他可能用 `--json`，不一致

### 3.4 acl — ACL 管理 (15 个子命令)

```
xun acl <command>
```

| 子命令 | 说明 | 参数 |
|--------|------|------|
| `view` | 查看 ACL | --path, --detail, --export |
| `add` | 添加权限 | 交互式向导 |
| `remove` | 删除权限 | 交互式多选 |
| `purge` | 清除所有 | principal |
| `diff` | 比较两个路径 | path1, path2 |
| `batch` | 批量处理 | file/list |
| `effective` | 有效权限 | user, path |
| `copy` | 复制 ACL | from, to |
| `backup` | 备份 ACL | path, --output |
| `restore` | 恢复 ACL | backup-file |
| `inherit` | 启用/禁用继承 | path, --enable/--disable |
| `owner` | 更改所有者 | path, owner |
| `orphans` | 孤儿 SID | path, --clean |
| `repair` | 强制修复 | path |
| `audit` | 审计日志 | --export |
| `config` | 配置 | 子命令组 |

### 3.5 alias — 别名管理 (11 个子命令)

```
xun alias [--config <config>] <command>
```

| 子命令 | 说明 | 参数 |
|--------|------|------|
| `setup` | 初始化运行时 | — |
| `add` | 添加别名 | name, command, --desc, --tags, --shells, --mode |
| `rm` | 删除别名 | name |
| `ls` | 列出别名 | --json, --tag |
| `find` | 模糊查找 | pattern |
| `which` | 显示目标 | name |
| `sync` | 同步 shim | — |
| `export` | 导出 | — |
| `import` | 导入 | path |
| `app` | 应用别名 | 子命令组 |

### 3.6 brn — 批量重命名 (单命令，30+ 参数)

```
xun brn [options] [<path>]
```

**参数分组:**

| 类别 | 参数 |
|------|------|
| 文本处理 | --trim, --trim-chars, --strip-brackets, --strip-prefix, --strip-suffix, --remove-chars |
| 查找替换 | --from, --to, --regex, --replace, --regex-flags |
| 大小写 | --case, --ext-case |
| 扩展名 | --rename-ext, --add-ext |
| 添加内容 | --prefix, --suffix, --insert-at, --template, --template-start, --template-pad |
| 切片 | --slice |
| 日期 | --insert-date, --ctime |
| 序列化 | --normalize-seq, --normalize-unicode, --seq, --start, --pad |
| 过滤 | --ext, --filter, --exclude, -r, --depth, --include-dirs |
| 输出 | --sort-by, --output-format, --apply, -y |
| 撤销 | --undo, --redo |

### 3.7 backup — 增量备份 (6 个子命令)

```
xun backup [options] [<command>]
```

| 子命令 | 说明 |
|--------|------|
| `create` | 创建备份 |
| `restore` | 恢复备份 |
| `convert` | 转换格式 |
| `list` | 列出备份 |
| `verify` | 验证完整性 |
| `find` | 查找备份 |

**顶层参数:**

| 参数 | 说明 |
|------|------|
| -m, --msg | 备份描述 |
| -C, --dir | 工作目录 |
| --container | 单文件容器 |
| --compression | 压缩配置 |
| --split-size | 分卷大小 |
| --dry-run | 干运行 |
| --list | 列出文件 |
| --no-compress | 跳过压缩 |
| --retain | 保留数量 |
| --include/--exclude | 包含/排除 |
| --incremental | 增量模式 |
| --skip-if-unchanged | 无变化跳过 |
| --diff-mode | 差异模式 |
| --json | JSON 输出 |

### 3.8 vault — 文件保险库 (8 个子命令)

```
xun vault <command>
```

| 子命令 | 说明 |
|--------|------|
| `enc` | 加密文件 |
| `dec` | 解密文件 |
| `inspect` | 检查结构 |
| `verify` | 验证完整性 |
| `resume` | 恢复中断任务 |
| `cleanup` | 清理临时文件 |
| `rewrap` | 重新包装 |
| `recover-key` | 恢复密钥 |

---

## 四、参数模式分析

### 4.1 输出格式参数 (3 种写法)

```rust
// 写法 1: --format (env, proxy detect, bookmark list)
--format auto|table|tsv|json

// 写法 2: --json / --tsv 开关 (bookmark z, backup)
--json
--tsv

// 写法 3: --export (acl view)
--export csv
```

**问题:** 同一功能，三种实现，用户困惑。

### 4.2 作用域参数 (bookmark 独有)

```rust
--global, -g      // 全局作用域
--child, -c       // 子作用域
--base            // 基目录限制
--workspace, -w   // 工作区
--preset          // 配置预设
```

**问题:** 只有 bookmark 有，其他命令没有统一的作用域概念。

### 4.3 排序/过滤参数

```rust
// bookmark list
--sort name|last|visits
--tag
--limit
--offset
--reverse

// env list
--scope user|system|all
--format

// 没有统一
```

---

## 五、命名规范问题

### 5.1 缩写不一致

| 命令 | 缩写 | 规则 |
|------|------|------|
| `proxy on` | `pon` | p + on |
| `proxy off` | `poff` | p + off |
| `proxy status` | `pst` | p + st |
| `proxy exec` | `px` | p + x |
| `bookmark` | 无 | 应该有 `bm`？ |
| `backup` | `bak` | 仅在 normalize_top_level_aliases 中 |
| `delete` | `del` | 仅在 run_from_env 中 |

**问题:** 缩写逻辑分散，有的在 CLI 定义，有的在运行时 normalize。

### 5.2 顶层命令重复

| 功能 | 命令 1 | 命令 2 | 说明 |
|------|--------|--------|------|
| 删除 | `delete` | `rm` | 功能重叠 |
| 进程 | `ps` | `ports` | 不同但易混淆 |
| 杀进程 | `kill` | `pkill` | 不同但易混淆 |
| 代理 | `proxy on` | `pon` | 同一功能 |

### 5.3 子命令命名不统一

| 模块 | 列出命令 | 删除命令 |
|------|----------|----------|
| bookmark | `list` | `delete` |
| env | `list` | `del` |
| acl | `view` | `remove` |
| alias | `ls` | `rm` |
| ctx | `list` | `del` |

**问题:** `list`/`ls`、`delete`/`del`/`rm`/`remove` 混用。

---

## 六、代码结构问题

### 6.1 Cmd struct 爆炸

```
总计: 370 个 Cmd struct
├── desktop: 55 个
├── bookmark: 23 个
├── env: 大量 (分布在 14 个文件)
├── acl: 17 个
├── alias: 17 个
├── proxy: 10 个
├── vault: 9 个
├── ctx: 8 个
├── backup: 7 个
└── 其他: ...
```

### 6.2 重复参数定义

```rust
// bookmark z
#[argh(option, short = 't')]
pub tag: Option<String>,

// bookmark list
#[argh(option, short = 't')]
pub tag: Option<String>,

// bookmark o
#[argh(option, short = 't')]
pub tag: Option<String>,

// ... 重复 N 次
```

### 6.3 输出处理分散

```rust
// 方式 1: 直接 println
println!("{}", item.name);

// 方式 2: comfy_table
let mut table = Table::new();
table.add_row(...);

// 方式 3: serde_json
println!("{}", serde_json::to_string(&item)?);

// 方式 4: 自定义格式
print_tsv(&items);
```

---

## 七、Feature Gate 矩阵

| 模块 | Feature | 编译命令 |
|------|---------|----------|
| alias | `alias` | `cargo build --features alias` |
| batch_rename | `batch_rename` | `cargo build --features batch_rename` |
| crypt | `crypt` | `cargo build --features crypt` |
| desktop | `desktop` | `cargo build --features desktop` |
| dashboard | `dashboard` | `cargo build --features dashboard` |
| diff | `diff` | `cargo build --features diff` |
| fs | `fs` | `cargo build --features fs` |
| img | `img` | `cargo build --features img` |
| lock | `lock` | `cargo build --features lock` |
| protect | `protect` | `cargo build --features protect` |
| redirect | `redirect` | `cargo build --features redirect` |
| xunbak | `xunbak` | `cargo build --features xunbak` |

---

## 八、统计摘要

| 指标 | 数值 |
|------|------|
| 顶层命令数 | 22 |
| Feature-gated 命令 | 12 |
| Cmd struct 总数 | 370 |
| 最大子命令组 | bookmark (26) |
| 最复杂单命令 | brn (30+ 参数) |
| 输出格式写法 | 3 种 |
| 命名规范 | 不统一 |

---

## 九、核心问题总结（深度诊断）

### 9.1 问题严重度矩阵

| 问题 | 严重度 | 影响范围 | 根因 |
|------|--------|----------|------|
| 缺乏类型级输出约束 | **致命** | 全部命令 | 无 `Renderable` trait，输出格式靠人工保证 |
| 参数重复定义 | 高 | bookmark/env/acl | argh 不支持 flatten，无法复用参数组 |
| 输出格式不统一 | 高 | 全部命令 | 三种写法并存，无编译期强制 |
| 错误处理过于简单 | 高 | 全部命令 | `CliError` 无分层，无法区分用户错误/内部错误/取消 |
| 命名规范混乱 | 中 | 用户侧 | 历史遗留，无自动化检查 |
| 顶层命令污染 | 中 | 命名空间 | `pon`/`poff`/`ps`/`kill` 等快捷命令暴露在顶层 |
| 缺乏统一执行上下文 | 高 | 全部命令 | 每个命令自己加载配置、判断交互、处理输出 |
| Shell 集成硬编码 | 中 | init 命令 | 脚本是字符串拼接，无法扩展新 shell |
| 编译时间隐患 | 低→中 | 迁移后 | clap derive 会增加 5-8s，需 workspace 拆分缓解 |

### 9.2 argh 的根本限制

argh 是 Google Fuchsia 项目的产物，设计目标与 XunYu 不匹配：

| argh 特性 | XunYu 需求 | 冲突 |
|-----------|-----------|------|
| 不支持 `--foo=bar` 语法 | Windows 用户习惯 `--key=value` | ❌ |
| 不支持短参数组合 `-abc` | Unix 标准 | ❌ |
| 不支持 flatten/参数组复用 | 370 个 Cmd struct 大量重复 | ❌ |
| 不支持 value_enum | OutputFormat 需要手动解析 | ❌ |
| 不支持 env 变量绑定 | `XUN_FORMAT`/`NO_COLOR` 需手动处理 | ❌ |
| 不支持 global 参数 | `--format` 需要在每个子命令重复定义 | ❌ |
| 无 shell completion 生成 | 当前自己实现，维护成本高 | ❌ |

结论：argh 已成为项目演进的**结构性瓶颈**，不是"可以改进"而是"必须替换"。

### 9.3 当前 runtime 全局状态的问题

```rust
static RUNTIME: OnceLock<RuntimeOptions> = OnceLock::new();
```

- 全局 `OnceLock` 无法在测试中重置 → 测试间状态泄漏
- `is_quiet()`/`is_verbose()` 是全局函数 → 无法为不同命令设置不同行为
- 配置加载时机不确定 → 有些命令在 dispatch 前加载，有些在执行中加载

### 9.4 输出处理的三重分裂

| 输出目标 | 当前实现 | 问题 |
|----------|---------|------|
| 结构化数据 | `println!` → stdout | 与 UI 信息混在一起 |
| UI 信息 | `eprintln!` via `ui_println` | quiet 模式下靠字符串匹配判断是否输出 |
| 表格 | `comfy_table` → stderr | 正确，但不是所有命令都用 |

正确的分离应该是：
- **stdout**: 仅结构化数据（可被管道消费）
- **stderr**: UI 信息、进度、错误
- 当前 `looks_like_error()` 函数通过字符串前缀判断是否是错误信息 — 这是 hack，不是设计

---

## 十、重构优先级建议（修订版）

### 10.1 优先级分层

```
P0 — 阻塞性问题（不解决则后续工作无法展开）
├── 替换 argh → clap 4 derive（解锁 flatten/global/env/completion）
├── 建立 Renderable trait + OutputFormat（编译期保证输出一致性）
└── 建立 CmdContext（替代全局 OnceLock 状态）

P1 — 架构统一（解决代码膨胀和维护成本）
├── 统一参数组：OutputArgs / ListArgs / FuzzyArgs / ScopeArgs
├── 统一错误类型：XunError 分层（User/Internal/Cancelled/ElevationRequired）
├── 统一命令执行：CommandSpec trait + Pipeline middleware
└── 迁移所有命令到新架构

P2 — 用户体验（解决命名混乱和命名空间污染）
├── 统一命名规范：rm/list/add/show/set
├── 快捷命令降级：pon/poff/pst/px → hidden alias
├── 命令重组：ports/kill/ps/pkill → port + proc 两个命令组
└── env 子命令分组（27 个太多）

P3 — 扩展性与性能
├── workspace 拆分（xun-cli / xun-core / xun-output）
├── Shell 集成 trait 化（支持 fish/nushell）
├── 命令自注册机制（inventory crate）
└── 编译时间 / 二进制大小基准测试
```

### 10.2 关键决策点

| 决策 | 选项 A | 选项 B | 建议 |
|------|--------|--------|------|
| CLI 库 | clap 4 derive | bpaf | clap（生态、completion、文档） |
| 命令 trait | XunCommand（OOP 钩子） | CommandSpec（泛型 Pipeline） | CommandSpec（零成本、编译期保证） |
| 输出抽象 | StructuredValue enum | Renderable trait | Renderable（开放扩展、零 match） |
| 错误处理 | 沿用 CliError | thiserror 分层 | thiserror（类型安全、exit code 自动化） |
| 实施节奏 | 逐模块迁移（6-8 周） | 一次性迁移（2-3 周） | 一次性（快速开发期，允许破坏性改动） |
| 删除命名 | `delete` | `rm` | `rm`（Unix 标准，更短） |

### 10.3 迁移风险评估

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| clap 编译时间增加 | 确定 | +5-8s | workspace 拆分 + feature 裁剪 |
| 499 个测试需要适配 | 确定 | 1-2 天工作量 | 大部分测试不涉及 CLI 解析层 |
| shell init 脚本兼容性 | 中 | 用户需重新 source | 保留旧命令为 hidden alias |
| 二进制大小增加 | 确定 | +300-500KB | LTO + strip（当前二进制已很大） |


---

## 十一、Operation 层缺失诊断

### 11.1 当前危险操作的处理方式

| 命令 | dry-run | undo | preview | audit | confirm |
|------|---------|------|---------|-------|---------|
| backup create | `--dry-run` (自己实现) | ❌ | `--list` (自己实现) | ❌ | ❌ |
| batch_rename | `--apply` 反转 (默认预览) | `--undo` (自己实现) | 默认输出 | ❌ | `-y` |
| env set/del | ❌ | ❌ | ❌ | `env audit` (独立命令) | ❌ |
| acl add/remove | ❌ | ❌ | ❌ | `acl audit` (独立命令) | 交互式向导 |
| delete | `--what-if` (自己实现) | ❌ | ❌ | ❌ | ❌ |
| vault enc/dec | ❌ | `vault resume` (部分) | ❌ | ❌ | ❌ |
| bookmark delete | ❌ | `bookmark undo` (自己实现) | ❌ | ❌ | ❌ |

**问题总结:**
- 每个命令自己发明 dry-run 参数名（`--dry-run`/`--what-if`/`--apply` 反转）
- undo 只有 bookmark 和 brn 有，且实现完全不同
- audit 只有 env 和 acl 有，且是独立命令而非自动记录
- 无统一的 preview → confirm → execute 流程
- Dashboard 无法复用这些逻辑（每个面板自己实现确认对话框）

### 11.2 缺失的统一抽象

```
当前：每个命令自己决定
├── 是否支持 dry-run（大部分不支持）
├── 如何展示预览（各自格式）
├── 是否需要确认（各自判断）
├── 是否记录审计（大部分不记录）
└── 是否可撤销（大部分不可）

需要：Operation trait 统一协议
├── preview() → Preview { changes, risk_level, reversible }
├── execute() → OperationResult
├── rollback() → 可选
└── run_operation() 统一流程：preview → confirm → execute → audit
```

### 11.3 对 Dashboard 的影响

当前 Dashboard 面板（`dashboard-ui/src/components/`）的问题：

| 面板 | 问题 |
|------|------|
| `TaskConfirmDialog.vue` | 每个任务自己定义确认内容，无统一 Preview 结构 |
| `RecentTasksPanel.vue` | 手动记录任务历史，无统一 OperationResult |
| `BatchGovernancePanel.vue` | 自己实现批量操作预览，无法复用 Operation.preview |
| `UnifiedConfirmDialog.vue` | 已有统一确认组件，但后端无统一 risk_level 数据 |
| `DiagnosticsCenterPanel.vue` | 诊断数据各模块自己提供，无统一 StructuredValue |

---

## 十二、Dashboard 现状与打通需求

### 12.1 当前 Dashboard 架构

```
dashboard-ui/ (Vite + Vue 3)
├── src/api.ts              — HTTP/WebSocket API 调用
├── src/types.ts            — TypeScript 类型定义
├── src/components/         — 面板组件（70+ 个）
│   ├── workspaces/         — 8 个工作台
│   ├── diff/               — Diff 可视化
│   └── ...                 — 各功能面板
├── src/features/           — 业务逻辑
│   ├── tasks/              — 任务系统（catalog/execution/receipt）
│   └── files-security/     — 文件安全上下文
└── src/ui/                 — 通用 UI 组件
```

### 12.2 当前 CLI ↔ Dashboard 的断裂

| 维度 | CLI 侧 | Dashboard 侧 | 断裂点 |
|------|--------|-------------|--------|
| 数据格式 | `println!` / `comfy_table` / `serde_json` | `api.ts` 手动解析 JSON | 无共享 schema |
| 命令执行 | 同步 `fn cmd_xxx()` | HTTP POST → 等响应 | 无流式进度 |
| 预览 | 各命令自己实现 | `TaskConfirmDialog` 手写 | 无统一 Preview 结构 |
| 审计 | `env audit` / `acl audit` 独立 | `AuditPanel.vue` 手动拉取 | 无自动记录 |
| 类型 | Rust struct | `types.ts` 手写 | 容易不同步 |

### 12.3 重构后的打通方案

**StructuredValue 作为数据总线：**
- Rust 侧 `Value` / `Table` 通过 `serde` 序列化
- Dashboard 侧 TypeScript 类型从 Rust struct 自动生成（`ts-rs` 或 `specta`）
- WebSocket 推送 `Table` 带 `ColumnDef` → 前端自动渲染列

**Operation 作为执行协议：**
- CLI: `run_operation()` → terminal preview → stdin confirm → execute
- Dashboard: WebSocket `preview` → 前端 `UnifiedConfirmDialog` → WebSocket `confirm` → execute
- 同一个 `Operation` impl，两个 adapter

**具体受益：**

| Dashboard 组件 | 重构前 | 重构后 |
|---------------|--------|--------|
| `BookmarksPanel.vue` | 手写列定义 | 从 `Table.columns` 自动生成 |
| `TaskConfirmDialog.vue` | 每任务自定义 | 消费统一 `Preview` 结构 |
| `RecentTasksPanel.vue` | 手动记录 | 自动消费 `OperationResult` 流 |
| `DiagnosticsCenterPanel.vue` | 各模块独立 API | 统一 `Value` 查询接口 |
| `EnvPanel.vue` | 专用 API | 通用 `CommandSpec` 调用 |
| `BatchGovernancePanel.vue` | 自己实现预览 | 复用 `Operation.preview()` |

### 12.4 TypeScript 类型自动生成

```toml
# Cargo.toml（未来）
[dependencies]
specta = { version = "2", features = ["derive"] }  # 或 ts-rs
```

```rust
// Rust 侧
#[derive(Serialize, specta::Type)]
pub struct Preview { ... }

#[derive(Serialize, specta::Type)]
pub struct Table { ... }
```

```typescript
// 自动生成 → dashboard-ui/src/generated/types.ts
export interface Preview {
  summary: string;
  changes: Change[];
  risk_level: "Low" | "Medium" | "High" | "Critical";
  reversible: boolean;
}

export interface Table {
  columns: ColumnDef[];
  rows: Record<string, Value>[];
}
```

这消除了 `types.ts` 手写与 Rust struct 不同步的问题。
