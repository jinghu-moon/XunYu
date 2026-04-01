# xun bookmark — 产品需求与设计文档（PRD）

> **版本**：1.0 · **日期**：2026-03-31 · **状态**：最终版  
> **作者**：产品架构 & 技术文档工程师  
> **关联文档**：Bookmarks-Competitor-Review.md · Bookmarks-Feature-Roadmap.md · bookmark-Config-Spec.md · bookmark-Benchmark-Suite.md · bookmark-SQLite-Evaluation.md

---

## 目录

1. [产品定位与目标](#1-产品定位与目标)
2. [竞品分析](#2-竞品分析)
3. [核心设计原则](#3-核心设计原则)
4. [命令面规范（最终版）](#4-命令面规范最终版)
5. [功能需求详述](#5-功能需求详述)
6. [数据模型](#6-数据模型)
7. [搜索与排序算法](#7-搜索与排序算法)
8. [性能规格](#8-性能规格)
9. [Shell 集成设计](#9-shell-集成设计)
10. [交互体验设计](#10-交互体验设计)
11. [数据治理](#11-数据治理)
12. [分阶段实施路线图](#12-分阶段实施路线图)
13. [验收标准](#13-验收标准)

---

## 1. 产品定位与目标

### 1.0 实现状态同步（2026-03-30）

- 已完成：`xun bookmark <sub>` / `bm <sub>` 正式命名空间
- 已完成：`z / zi / o / oi`、query core、completion 对齐
- 已完成：`explicit / imported / learned / pinned` 新数据模型
- 已完成：主存储切换到 `~/.xun.bookmark.json`
- 已完成：自动学习、外部导入、`bm init`
- 已完成：持久化倒排索引（JSON sidecar）、delta-based `undo / redo`
- 已完成：bookmark 存储与 completion 实现收口到 `src/bookmark/`
- 已完成：`fuzzy` 公共命令删除
- 暂未完成：Dashboard 面板同步、SQLite

### 1.1 一句话定位

> **xun bookmark** 是一个面向开发者的命令行书签导航系统，将「显式命名书签管理」与「智能目录自动学习」融为一体，在 Windows / PowerShell 主场景下提供媲美 zoxide 的极速导航体验，同时提供 zoxide 完全不具备的显式书签、标签、工作区和可解释排序能力。

### 1.2 目标用户

| 用户层级 | 描述 | 核心诉求 |
|---|---|---|
| **入门用户** | 刚开始使用 CLI，习惯 GUI 文件管理器 | 低学习曲线，`bm z foo` 一秒上手 |
| **日常开发者** | 每天在多个项目目录间切换 | 极速跳转、frecency 自动排序 |
| **高阶用户** | 管理数十个命名书签与 workspace | 标签、pin、范围搜索、导出/导入 |
| **迁移用户** | 从 zoxide / autojump / z 切换而来 | 数据导入、命令习惯零迁移成本 |

### 1.3 三大核心产品目标

```
极致效能  ──  P99 < 50ms（z 跳转）/ P99 < 200ms（zi 首次渲染）
绝佳体验  ──  全键盘操作 / 零打断极速模式 / 歧义时主动提示
全面实用  ──  显式书签 + 自动学习 + tag + workspace + 可解释排序
```

---

## 2. 竞品分析

> 以下分析基于 2026 年 3 月公开资料，所有引用均附来源。

### 2.1 竞品概览

| 工具 | 类型 | 语言 | 核心能力 | 主要缺陷 | 来源 |
|---|---|---|---|---|---|
| **zoxide v0.9+** | 智能跳转器 | Rust | frecency、多 token 匹配、fzf 集成、`init <shell>` 模板 | 无显式书签、无 tag、无 workspace | [github.com/ajeetdsouza/zoxide](https://github.com/ajeetdsouza/zoxide) |
| **autojump** | 自动学习跳转 | Python | shell hook 自动学习、多参数查询 `j foo bar` | **维护明显减弱**、无 Windows 原生支持、无显式书签 | [github.com/wting/autojump](https://github.com/wting/autojump) |
| **bashmarks** | 显式书签脚本 | Shell | 极简 5 命令（s/g/p/d/l）、tab 补全 | 存储格式为 shell 变量文件（无结构化扩展）、无 frecency | [github.com/huyng/bashmarks](https://github.com/huyng/bashmarks) |
| **whoosh.yazi** | Yazi 插件书签管理 | Lua | 持久/临时书签、目录历史、fzf 模糊搜索、路径截断 | 依赖 Yazi，不能独立在 CLI 使用 | [github.com/WhoSowSee/whoosh.yazi](https://github.com/WhoSowSee/whoosh.yazi) |
| **yamb.yazi** | Yazi 插件 | Lua | 持久书签、key 跳转、fzf | 同上，插件生态限制 | [github.com/h-hg/yamb.yazi](https://github.com/h-hg/yamb.yazi) |
| **bunny.yazi** | Yazi 插件 | Lua | 持久+临时书签、模糊搜索、前一目录 | 同上 | [github.com/stelcodes/bunny.yazi](https://github.com/stelcodes/bunny.yazi) |
| **fzf-marks** | fzf + 书签 | Shell | fzf 交互选择 + 命名书签 | 无 frecency、无 tag | [github.com/urbainvaes/fzf-marks](https://github.com/urbainvaes/fzf-marks) |

### 2.2 zoxide 算法深度参考

> 来源：[zoxide Wiki — Algorithm](https://github.com/ajeetdsouza/zoxide/wiki/Algorithm) · [Arch Linux man page zoxide(1)](https://man.archlinux.org/man/zoxide.1.en)

zoxide 的 frecency 算法是业界验证最充分的同类实现，其核心逻辑：

- 目录首次访问得分为 `1`，此后每次访问 `+1`
- 查询时根据最后访问时间计算衰减系数（时间桶）
- 引入 `_ZO_MAXAGE`（默认 `10000`）控制数据库规模：当总分超过该值时，所有条目除以系数 `k` 使总分归约至约 `90% × _ZO_MAXAGE`，得分低于 `1` 的条目被删除
- 不存在于文件系统且超过 90 天未访问的条目被懒删除

**xun bookmark 的 frecency 设计以 zoxide 参数为基准**，在此之上叠加显式书签权重与 tag 加成（见第 7 节）。

### 2.3 能力差距矩阵

| 能力维度 | zoxide | autojump | bashmarks | **xun bookmark 目标** |
|---|:---:|:---:|:---:|:---:|
| 显式命名书签 | ✗ | ✗ | ✓ | **✓★** |
| 自动目录学习 | ✓ | ✓ | ✗ | **✓** |
| Frecency 排序 | ✓ | ✓ | ✗ | **✓** |
| 多 token 查询 | ✓ | ✓ | ✗ | **✓** |
| 交互式跳转（fzf） | ✓（zi） | △ | ✗ | **✓（zi / oi）** |
| 打开文件管理器 | ✗ | △（jo） | ✗ | **✓（o / oi）** |
| Tag 系统 | ✗ | ✗ | ✗ | **✓★** |
| Workspace 范围搜索 | ✗ | ✗ | ✗ | **✓★** |
| 可解释排序（--why） | ✗ | ✗ | ✗ | **✓★** |
| 跨生态数据导入 | ✓（autojump/z） | ✗ | ✗ | **✓** |
| Shell init 模板 | ✓ | △ | ✗ | **✓** |
| Windows / PowerShell | △ | ✗ | ✗ | **✓★（首要平台）** |
| 数据导出 / Dashboard API | ✗ | ✗ | ✗ | **✓★** |

> ★ = xun bookmark 独有差异化能力

### 2.4 竞品关键教训

**学 zoxide**：工程完成度、`init <shell>` 统一模板、fzf 优先降级内置选择器、`--cmd` 参数让用户自定义命令前缀。

**学 autojump**：多参数 AND 查询语义（`j foo bar`）、`jc` 子树优先模式。但**注意**：autojump 维护明显减弱，其多套 shell hack 的历史包袱不应照搬。

**学 bashmarks**：极简心智（5 个动词就够），但不学其 `.sdirs` 变量文件存储格式——该格式不支持结构化扩展。

**学 whoosh.yazi / yamb.yazi**：持久+临时书签分层、key 直接跳转的设计，证明"命名书签"场景真实存在且高频。

---

## 3. 核心设计原则

### 3.1 搜索层统一，动作层分离

所有查询逻辑（匹配、排序、frecency、tag、多 token、范围控制）收敛至唯一一个 `bookmark::query` 内核。`z`、`zi`、`o`、`oi` 四个命令**仅在最终执行动作上不同**，排序结果完全一致，用户不会遇到「z 跳 A，o 跳 B」的割裂。

```
bookmark::query(spec)
       │
       ├─ JumpFirst   →  z   (cd 到 Top-1)
       ├─ Interactive →  zi  (fzf 选择后 cd)
       ├─ OpenFirst   →  o   (用文件管理器打开 Top-1)
       └─ OpenInter   →  oi  (fzf 选择后用文件管理器打开)
```

### 3.2 显式书签优先于自动学习

hybrid 排序模型中，用户主动保存的显式书签（`explicit`）通过 `PinBoost` 和 `SourceBoost` 永远优先于自动学习结果（`learned`）。用户对导航意图的「主动表达」比算法推断更可信。

### 3.3 Windows-first

Windows + PowerShell 是首要打磨平台。路径大小写不敏感、`\`/`/` 统一、UNC 路径、`~` 展开、reparse point 处理均进入核心设计，不可后补。

### 3.4 命令面克制

**不新增**与现有命令高度重叠的子命令。`check / gc / dedup / export / import / tag / stats / recent` 保持原有职责，不因引入新能力而扩张。

### 3.5 可解释性

任何排序结果都应能通过 `--list --score` 或 `--why` 看见各维度得分。这是调参与用户信任的基础。

---

## 4. 命令面规范（最终版）

### 4.1 命名约定

| 规范项 | 值 | 说明 |
|---|---|---|
| 主命令 | `bookmark` | **单数**，严格统一 |
| 快捷别名 | `bm` | 官方短别名，等价于 `xun bookmark` |
| 子命令前缀 | `xun bookmark <sub>` 或 `bm <sub>` | 等价 |

### 4.2 四个导航核心子命令

这四个命令构成 `z × i × (cd|open)` 的完整对称矩阵：

| 命令 | 全称 | 动作 | 模式 |
|---|---|---|---|
| `z` | jump | `cd` 跳转 | 极速，取 Top-1 |
| `zi` | jump interactive | `cd` 跳转 | 交互，fzf 选择 |
| `o` | open | 文件管理器打开 | 极速，取 Top-1 |
| `oi` | open interactive | 文件管理器打开 | 交互，fzf 选择 |

> **对称性说明**：`i` 后缀统一表示「交互选择模式」。`z`/`zi` 负责 cd 维度，`o`/`oi` 负责 GUI 打开维度，两个维度各自完整、互不依赖。

### 4.3 `z` 命令完整规范

```
bm z [<keywords>...] [options]
```

**默认行为**：多 token AND 匹配，取 Top-1，输出 `__BM_CD__` 供 shell wrapper 执行 `cd`。

**选项列表**：

| 选项 | 短选项 | 说明 |
|---|---|---|
| `--list` | `-l` | 只打印候选列表，不跳转 |
| `--score` | `-s` | 与 `--list` 合用，显示各维度得分 |
| `--why` | | 解释 Top-1 推荐原因 |
| `--preview` | | 打印候选与原因，**不执行**跳转（dry-run） |
| `--global` | `-g` | 关闭 cwd 相关加权，纯全局搜索 |
| `--child` | `-c` | 当前目录子树优先（类 autojump `jc`） |
| `--base <dir>` | | 只在指定父路径下搜索 |
| `--workspace <name\|path>` | `-w` | workspace 范围搜索 |
| `--exclude-cwd` | | 排除当前目录本身 |
| `--tag <tag>` | `-t` | 仅在带有该 tag 的书签中搜索 |
| `--limit <n>` | `-n` | 限制返回数量（`--list` 时有效） |
| `--json` | | 结构化 JSON 输出 |
| `--tsv` | | TSV 输出（供脚本/Dashboard 使用） |
| `--pin` | | 只在 pinned 书签中搜索 |

**路径字面量兼容**（与 zoxide 一致）：

```bash
bm z ~/projects/foo   # 绝对路径直接跳转
bm z ..               # 相对路径
bm z -                # 返回上一个目录
```

### 4.4 `zi` 命令完整规范

```
bm zi [<keywords>...] [options]
```

继承 `z` 的全部 query 选项（`--tag / --child / --base / --workspace / --global` 等）。

**后端优先级**：
1. 若 `fzf >= v0.51.0` 存在，调用 fzf（支持 preview 窗口显示目录内容）
2. 否则降级到内置 `dialoguer::FuzzySelect`

**Windows 降级说明**：内置选择器需明确标注最低支持 Windows Terminal 版本（≥ 1.18），在低版本终端中可退化为编号选择（打印候选列表，用户输入数字）。

### 4.5 `o` 命令完整规范

```
bm o [<keywords>...] [options]
```

取 Top-1，调用平台文件管理器：
- Windows：`explorer.exe <path>` 或 `cmd /c start "" "<path>"`
- macOS：`open <path>`
- Linux：`xdg-open <path>`

继承 `z` 的全部 query 选项。

> **关于 `open` 子命令**：`open` 保留为 `bookmark` 命名空间内的正式子命令，可复用 `o` 的实现，但不再作为旧顶层命令存在。

### 4.6 `oi` 命令完整规范

```
bm oi [<keywords>...] [options]
```

交互选择后调用文件管理器打开所选目录，后端优先级与 `zi` 相同。

### 4.7 管理命令（保持现有职责）

| 命令 | 功能 |
|---|---|
| `bm set <name> [path]` | 保存当前目录或指定路径为显式书签 |
| `bm save` | 保存当前目录或指定路径为显式书签的便捷子命令 |
| `bm tag <name> <tag...>` | 为书签添加/移除标签 |
| `bm pin <name>` | 置顶书签（设置 `pinned=true`） |
| `bm unpin <name>` | 取消置顶 |
| `bm delete <name>` | 删除显式书签 |
| `bm rename <old> <new>` | 重命名书签 |
| `bm list` | 列出所有书签（支持 `--json / --tsv`） |
| `bm recent` | 最近访问记录（支持 `--tag / --workspace / --since`） |
| `bm stats` | 访问统计 |
| `bm check` | 扫描 missing / stale / duplicate |
| `bm gc` | 清理 dead link |
| `bm dedup` | 去重 |
| `bm export` | 导出（JSON / TSV） |
| `bm import` | 导入（自有格式 / autojump / zoxide / z / fasd） |
| `bm init <shell>` | 输出 shell 初始化脚本 |
| `bm touch <name>` | 手动刷新书签 frecency |

**命名约束**：

- `explicit` 条目的 `name` 全局唯一，按大小写不敏感规则比较
- `imported / learned` 条目允许无显式名称（`name = null`）
- 所有 `<name>` 型管理命令只作用于具名 `explicit` 条目

**说明**：独立 `workspace` 动作子命令已移除，统一改为 `z / o / recent` 等命令上的 `--workspace` 查询范围。

### 4.8 历史命令说明

- `bm fuzzy` 已从正式方案删除
- 机器输出查询统一使用 `bm z --list`
- 文档与 help 不再保留 `fuzzy` 过渡期描述

### 4.9 `--cmd` 参数（学习 zoxide）

```bash
bm init powershell --cmd j   # 输出 j / ji / jo / joi 的 alias
bm init bash --cmd j
```

允许用户将命令前缀改为 `j`（对 autojump 迁移用户友好）或任意自定义前缀，默认仍为 `z / zi / o / oi`。

---

## 5. 功能需求详述

### 5.1 P0 — 地基能力

#### 5.1.1 命令面一致性收敛

**背景**：当前代码中 bookmark 相关能力仍散落在顶层 parser；vNext 需要整体收束到 `xun bookmark <sub>` 命名空间。

**需求**：
- parser 正式暴露 `bookmark` 子命令树
- `z / zi / o / oi / open / save / set / ...` 全部下沉到 `bookmark` 命名空间
- 删除旧顶层 `xun z / o / ws / sv / fuzzy / ...` 公共入口
- parser / help / completion / shell init / dispatch / 文档 同步更新
- 验收：用户看到的 help 与 parser 行为严格一致，`xun bookmark ...` 与 `bm ...` 为唯一正式入口

#### 5.1.2 统一 query core

**Rust 接口设计**：

```rust
pub struct BookmarkQuerySpec {
    pub keywords:   Vec<String>,
    pub tag:        Option<String>,
    pub scope:      QueryScope,
    pub action:     QueryAction,
    pub limit:      Option<usize>,
    pub explain:    bool,   // --why / --score
    pub preview:    bool,   // --preview dry-run
    pub output_fmt: QueryFormat,
}

pub enum QueryScope {
    Auto,                   // 默认，允许 cwd 加权
    Global,                 // 关闭 cwd 偏置
    Child,                  // 当前目录子树优先
    BaseDir(PathBuf),       // 只在指定父目录范围内
    Workspace(String),      // workspace 根范围
}

pub enum QueryAction {
    JumpFirst,
    Interactive,
    OpenFirst,
    OpenInteractive,
    List,
    Complete,
}

pub enum QueryFormat {
    Text,
    Tsv,
    Json,
}
```

**验收**：`z` 与 `o` 结果排序完全一致；completion 不再单独维护排序逻辑。

#### 5.1.3 多 token + name/path/tag 混合匹配

**需求**：

- 输入切分为空格分隔的 token，token 间采用 AND 语义
- 每个 token 依次尝试匹配以下字段（优先级从高到低）：
  1. `name` exact / prefix
  2. `path basename` exact / prefix
  3. `path segment` ordered match
  4. `tag` hit（加分，不过滤）
  5. subsequence fuzzy（兜底）
- 最后一个 token 对 `basename` / 末级目录有更高敏感度

**示例场景验收**：

```bash
bm z client api     # → 匹配 name/path 同时含 "client" 和 "api" 的书签
bm z repo docs      # → 匹配 /repos/xxx/docs 类路径
bm z work rust      # → 匹配 tag=work 且 path 含 "rust" 的书签
```

#### 5.1.4 `explicit / imported / learned / pinned` 数据模型

新增字段（在现有 JSON schema 上增量扩展）：

```jsonc
{
  "name": "my-project",        // explicit 必填；learned/imported 可为 null
  "name_norm": "my-project",   // explicit 名称唯一键（大小写不敏感）
  "path": "C:\\Users\\dev\\projects\\my-project",
  "source": "explicit",      // explicit | imported | learned
  "pinned": false,
  "tags": ["work", "rust"],
  "desc": "",                // 预留，P1 填充
  "created_at": 1743292800,
  "last_visited": 1743379200,
  "visit_count": 42,
  "frecency_score": 87.3,
  "schema_version": 1
}
```

**唯一性规则**：

- `explicit` 条目按 `name_norm` 全局唯一
- `bm set <name>` 若命中同名 explicit 条目，则更新该条目而不是新建
- `bm rename <old> <new>` 若 `new` 已存在，则报错并拒绝覆盖
- `imported / learned` 条目默认不要求具名，不参与 name 唯一性检查

**排序公式（乘法形式，各因子均为无量纲系数）**：

```
FinalScore = MatchScore × FrecencyMult × ScopeMult × SourceMult × PinMult
```

| 因子 | 含义 | 范围 |
|---|---|---|
| `MatchScore` | 结构化匹配得分（见第 7 节） | 0 ~ 100 |
| `FrecencyMult` | `1 + norm(frecency) × 0.25` | 1.0 ~ 1.25 |
| `ScopeMult` | 上下文相关度系数 | 1.0 ~ 2.5 |
| `SourceMult` | explicit=1.20 / imported=1.05 / learned=1.00 | — |
| `PinMult` | pinned=1.50 / normal=1.00 | — |

> 选用乘法形式而非加权和，理由：pinned + explicit 书签的优先级通过系数相乘自然拉开，不需要手工调节绝对分值；各因子独立，可解释。

**验收**：`learned` 不压过 `pinned explicit`；`imported` 可在排序和 `--list` 中单独可见。

#### 5.1.5 自动学习目录访问（含噪声排除）

**需求**：

- 通过 shell hook 自动记录用户进入过的目录
- 先支持 PowerShell，再扩展 bash / zsh / fish
- `learned` 数据单独入库，与 `explicit` 并存不覆盖
- 冷启动：初次启用时扫描 shell history 预填充
  - PowerShell：读取 `$env:APPDATA\Microsoft\Windows\PowerShell\PSReadline\ConsoleHost_history.txt`，提取 `cd <path>` 类指令
  - Bash：读取 `~/.bash_history`，提取 `cd <path>` / `z <path>` / `j <path>` 类指令
  - Zsh：读取 `~/.zsh_history`（EXTENDED_HISTORY 格式），同上
  - 预填充条目 frecency 初始值为「真实访问条目平均值的 30%」（低于真实使用数据）
  - 该操作为一次性，可通过 `bm import --from-history` 手动重新触发

**默认排除目录**（可通过 `_BM_EXCLUDE_DIRS` 环境变量覆盖）：

```
node_modules, dist, build, target, .git, tmp, temp,
%TEMP%, %TMP%, /tmp, /var/tmp
```

**必须同时具备的控制能力**（上线时不可缺）：
- `_BM_EXCLUDE_DIRS` glob 排除配置
- `bm gc --learned` 清理 learned 记录
- `bm learn --off` 全局关闭自动学习
- `bm list --source learned` 查看 learned 记录

#### 5.1.6 导入外部生态

| 来源 | 命令 | 数据文件位置 | 特殊处理 |
|---|---|---|---|
| autojump | `bm import --from autojump` | 按 OS/env 探测标准路径 | 直接读取 |
| zoxide | `bm import --from zoxide` | 按 OS/env 探测标准路径 | 调用 `zoxide query --list --score` 导出文本再解析 |
| rupa/z | `bm import --from z` | 按 OS/env 探测标准路径 | `rank\|time\|path` 格式 |
| z.lua | `bm import --from z` | 按 OS/env 探测标准路径 | 类 rupa/z，字段顺序略有差异 |
| zsh-z | `bm import --from z` | 按 OS/env 探测标准路径 | 兼容 rupa/z |
| fasd | `bm import --from fasd` | 按 OS/env 探测标准路径 | **只导入 d（目录）类型**，文件类型丢弃 |

**导入流程**（统一）：
1. 读取源文件
2. 路径标准化（分隔符/大小写/尾随斜杠/`~` 展开）
3. score 归一化映射（各工具分值量级不同，统一映射至 `[1, 100]`）
4. dedup
5. source 标记为 `imported`
6. 写入 `learned` 库（不覆盖同路径的 `explicit` 书签）

**zoxide 导入字段语义**：

- `zoxide query --list --score` 可稳定提供 `path` 与导出分数
- 对这类导入条目：
  - `frecency_score` 直接写入归一化后的导入值
  - `visit_count` / `last_visited` 允许为 `null`
  - 第一次本地命中或访问后，再开始填充本地访问历史
- 查询阶段如果 `visit_count` / `last_visited` 缺失，则 `FrecencyMult` 直接由持久化 `frecency_score` seed 归一化得到

#### 5.1.7 路径标准化全链路

所有入库操作（`explicit` / `imported` / `learned`）在写入前统一经过 normalization pipeline：

```
原始路径
  → 展开 ~ / $HOME
  → 相对路径转绝对路径
  → 分隔符统一（Windows: 保留 \ 为主，存储时统一为 /，展示时按平台还原）
  → 大小写（Windows: 统一为小写比较键，显示原始大小写）
  → 移除尾随分隔符（根路径 C:\ 除外）
  → UNC 路径 \\server\share 合法性验证
  → reparse point / symlink 解析（可选，受 _BM_RESOLVE_SYMLINKS 控制）
```

**验收**：同一目录不因大小写、分隔符、尾随斜杠差异重复入库。

---

### 5.2 P1 — 体验闭环

#### 5.2.1 `zi` 交互式跳转

- `bm zi [keywords...]` 强制进入 fzf 交互
- 共用 query core，Top-K 候选（默认 20）作为 fzf 输入
- fzf preview 窗口：显示目录内容（调用 `ls` / `dir`）
- 选中后输出 `__BM_CD__ <path>` 供 shell wrapper 执行

#### 5.2.2 `oi` 交互式打开

- 同 `zi`，但选中后调用平台文件管理器打开而非 cd

#### 5.2.3 显式范围搜索

详见第 4.3 节选项表。`--auto`（默认）延续当前 `cwd_boost` 心智，但在新 query core 中成为正式 scope。

#### 5.2.4 `--list / --score / --why`

`bm z --list --score foo` 输出示例：

```
 # │ Score  │ Match  │ Frecency │ Scope │ Source │ Pin │ Path
───┼────────┼────────┼──────────┼───────┼────────┼─────┼─────────────────────────────
 1 │  87.3  │  72.0  │   1.21   │  1.50 │  1.20  │ 1.0 │ C:\dev\projects\my-client
 2 │  61.4  │  60.0  │   1.10   │  1.00 │  1.05  │ 1.0 │ D:\work\client-api
 3 │  44.2  │  48.0  │   1.08   │  1.00 │  1.00  │ 1.0 │ C:\repos\client-tools
```

`bm z --why foo` 输出示例：

```
→ 跳转至：C:\dev\projects\my-client
原因：
  MatchScore  72.0  (name prefix 命中 "my-client" → 80; basename 命中 "my-client" → 80; 取最高)
  FrecencyMult 1.21 (42 次访问, 最近 2 小时内, 衰减系数 4.0)
  ScopeMult   1.50  (当前目录 C:\dev\projects 是书签父路径, +child boost)
  SourceMult  1.20  (explicit 书签)
  PinMult     1.00  (未置顶)
  FinalScore  87.3
```

#### 5.2.5 歧义提示

当 Top-1 与 Top-2 的 `FinalScore` 差距 < 15% 时，在执行跳转后追加一行提示（不阻断跳转）：

```
→ C:\dev\projects\my-client
  提示：还有 2 个相近候选，使用 'bm zi foo' 查看
```

#### 5.2.6 Completion 与 query core 对齐

- Tab completion 调用 `bookmark::query(spec)` 生成候选，排序逻辑与 `bm z --list` 完全一致
- 候选项展示格式：`<name>  <path>（frecency: xx）`
- PowerShell completion 使用 `Register-ArgumentCompleter`

#### 5.2.7 主动 dead-link 提示

- `z / zi / o / oi` 命中书签时：若路径不存在，立即提示并中止（不执行 cd/open）
- 网络路径（UNC）检测加超时保护：**300ms 超时则跳过检测**，不阻塞主路径
- 提示格式：

```
[xun] 错误：书签 'my-project' 指向的路径已不存在
  路径：C:\dev\projects\my-project
  建议：运行 'bm gc' 清理死链，或 'bm set my-project' 更新路径
```

#### 5.2.8 Shell init 模板化

```bash
bm init powershell   # 输出 PowerShell 完整 init 脚本
bm init bash
bm init zsh
bm init fish
```

每个模板输出内容包含：
1. `z / zi / o / oi` shell wrapper 函数（捕获 `__BM_CD__` 输出并执行 cd）
2. 自动学习 hook（`prompt` / `pwd` 模式可选）
3. tab completion 注册
4. 命令别名（`bm` = `xun bookmark`）
5. 可选 alias（注释形式，用户手动启用）：`alias cd='bm z'`

#### 5.2.9 `--preview / --dry-run`

`bm z --preview foo` 等价于 `bm z --list --score foo`，但添加标题提示「预览模式，不会执行跳转」。用于调试和演示新排序行为。

---

### 5.3 P2 — 长期增强

| 功能 | 说明 |
|---|---|
| `desc` 字段 | `bm set --desc "主项目"` 添加说明；在 `--list / zi / oi` 中展示 |
| `recent` 增强 | 支持 `--tag / --workspace / --since 7d` 过滤 |
| SQLite 评估/迁移 | 当前已完成评估；当数据量持续上升或并发读写出现后再进入迁移 |
| 倒排索引 | 持久化倒排索引（JSON sidecar）已落地；SQLite 索引化后端未做 |
| `undo / redo` | `set / save / rename / delete / import / pin / unpin / tag / gc / dedup` 操作可撤销与重做 |
| schema migration | 自 vNext 新 schema 起，后续版本由 `schema_version` 驱动增量迁移 |
| Benchmark 套件 | 同机对比 zoxide，数据量 > 5000，覆盖 Windows + PowerShell |

---

## 6. 数据模型

### 6.1 主存储（`~/.xun.bookmark.json`）

```jsonc
{
  "schema_version": 1,
  "bookmarks": [
    {
      "id": "uuid-v4",
      "name": "my-project",                     // explicit 必填；learned/imported 可为 null
      "name_norm": "my-project",                // explicit 名称唯一键
      "path": "C:/dev/projects/my-project",     // 统一 /，显示时按平台还原
      "path_norm": "c:/dev/projects/my-project", // 比较键（Windows 小写）
      "source": "explicit",                      // explicit | imported | learned
      "pinned": false,
      "tags": ["work", "rust"],
      "desc": "",
      "created_at": 1743292800,
      "last_visited": 1743379200,
      "visit_count": 42,
      "frecency_score": 87.3,
      "workspace": "xunyu"                       // 可为 null
    }
  ]
}
```

补充说明：

- `name` / `name_norm` 仅对具名 `explicit` 条目强制要求
- imported / learned 条目可使用 `name = null` 与 `name_norm = null`
- `visit_count` / `last_visited` 在 imported 条目上可为 `null`

### 6.2 访问日志（`~/.xun.bookmark.visits.jsonl`）

```jsonc
{"id": "uuid-v4", "ts": 1743379200, "action": "jump"}
{"id": "uuid-v4", "ts": 1743382800, "action": "open"}
```

- append-only，超过阈值（N 次或 T 秒）后回灌主库并压缩
- 自动学习的 learned 条目写入同一文件，通过 `source` 字段区分

### 6.3 Dirty save 策略

```
访问增量累计 > 50 次  →  触发 compact & save
或距上次 save > 600s  →  触发 compact & save
learned 条目允许更激进的 compact：增量 > 100 次或 > 300s
```

写盘使用 temp file + atomic rename，防止中途崩溃导致数据损坏。

---

## 7. 搜索与排序算法

### 7.1 候选召回（两阶段）

**第一阶段：候选召回**

- 按 token 查倒排索引（name / basename / segment / tag）
- token 间取交集（AND 语义）
- 候选集上限：500 条（防止大库精排开销过大）

**第二阶段：综合精排**

对候选集计算 `FinalScore`，取 Top-K。

### 7.2 MatchScore 分层打分

| 层级 | 类型 | 分值 |
|---|---|---|
| **强匹配** | name exact | 100 |
| | name prefix | 80 |
| | path basename exact | 70 |
| | path basename prefix | 60 |
| **结构匹配** | path segment ordered match | 40 ~ 55 |
| | multi-token cross-field | 35 ~ 50 |
| | tag hit | +10 ~ +15 |
| **弱 fuzzy** | subsequence fuzzy（兜底） | 10 ~ 35 |

**设计原则**：
- 强匹配永远压过弱 fuzzy
- tag 只加分，不单独压过名称强匹配
- 最后一个 token 对 basename 有额外 +10 权重

### 7.3 FrecencyMult 计算

基于 zoxide 验证参数，在 xun 中适配为乘法因子：

```
raw_frecency = log(1 + visit_count) × time_decay(last_visited)
FrecencyMult = 1.0 + normalize(raw_frecency, global_max) × 0.25
```

如果条目来自导入源且缺少本地访问历史（`visit_count = null` 或 `last_visited = null`），则：

```text
FrecencyMult = 1.0 + normalize(frecency_score, global_max) × 0.25
```

也就是说，`frecency_score` 既是运行时缓存，也是 imported 条目的 seed 分值。

时间衰减系数（与 zoxide 一致）：

| 最近访问时间 | 衰减系数 |
|---|---|
| < 1 小时 | 4.0 |
| < 1 天 | 2.0 |
| < 7 天 | 1.0 |
| < 30 天 | 0.5 |
| ≥ 30 天 | 0.2 |

> 来源参考：[zoxide Wiki — Algorithm](https://github.com/ajeetdsouza/zoxide/wiki/Algorithm)

### 7.4 ScopeMult 计算

| 上下文关系 | 系数 |
|---|---|
| 书签路径 == 当前目录 | 2.5 |
| 书签是当前目录的父路径 | 2.0 |
| 书签是当前目录的子路径 | 1.8 |
| 同一 workspace | 1.3 |
| 无关 | 1.0 |

范围模式对 ScopeMult 的影响：
- `--global`：ScopeMult 固定为 1.0（关闭）
- `--child`：非子树结果 ScopeMult 降为 0.5，子树结果升至 3.0
- `--base <dir>`：非 base 范围结果直接过滤

### 7.5 数据库老化（对齐 zoxide `_ZO_MAXAGE`）

```
_BM_MAXAGE = 10000（默认，可配置）

当 sum(frecency_score for all learned entries) > _BM_MAXAGE:
    k = sum / (0.9 × _BM_MAXAGE)
    for each entry: score /= k
    remove entries with score < 1
```

`explicit` 和 `pinned` 书签**不参与**老化删除，只有 `learned` 条目受此约束。

---

## 8. 性能规格

### 8.1 延迟预算（硬性要求）

| 操作 | 目标 | 说明 |
|---|---|---|
| `bm z <keyword>` 单次跳转 | **P99 < 50ms** | Windows + PowerShell，5000 条数据 |
| `bm zi` 首次候选渲染 | **P99 < 200ms** | 同上 |
| `bm o <keyword>` | **P99 < 50ms** | 同 z |
| 自动学习 hook 写入 | **不阻塞 prompt** | 异步后台写入 |
| Completion Top-K 生成 | **< 80ms** | 用户按 Tab 到候选出现 |

### 8.2 Benchmark 规格（P2 验收门槛）

任何宣称"达到 zoxide 级体验"的迭代必须满足：

- 同机环境对比 zoxide v0.9+
- 数据规模 > 5000 条
- 平台：Windows 11 + PowerShell 7.x
- 覆盖命令：`z` / `zi` / completion
- 输出：回归基准脚本（可 CI 集成）

### 8.3 内存与存储

- 主库（5000 条）JSON 预估 < 2MB
- 运行时内存：目标 < 20MB（不含 fzf 进程）
- visits.jsonl 压缩后每 1000 条 < 50KB

---

## 9. Shell 集成设计

### 9.1 PowerShell init 模板（核心）

```powershell
# 由 'bm init powershell' 生成
# 版本：1.0 · 平台：Windows + PowerShell 7.x

# ── 核心 wrapper 函数 ──────────────────────────────────────

$__bm_exe = if ($env:XUN_EXE) { $env:XUN_EXE } else { "xun.exe" }

function bm { & $__bm_exe bookmark @args }

function z {
    $result = & $__bm_exe bookmark z @args
    if ($result -match '^__BM_CD__ (.+)$') {
        Set-Location $Matches[1]
    } elseif ($result) {
        Write-Output $result
    }
}

function zi {
    $result = & $__bm_exe bookmark zi @args
    if ($result -match '^__BM_CD__ (.+)$') {
        Set-Location $Matches[1]
    }
}

function o  { & $__bm_exe bookmark o  @args }
function oi { & $__bm_exe bookmark oi @args }

# ── 自动学习 hook ─────────────────────────────────────────

$__bm_prev_pwd = $PWD.Path
function __bm_hook {
    $cur = $PWD.Path
    if ($cur -ne $__bm_prev_pwd) {
        # 异步子进程写入，不在会话内累积 PowerShell Job
        Start-Process -FilePath $__bm_exe `
            -ArgumentList @('bookmark', 'learn', '--path', $cur) `
            -WindowStyle Hidden | Out-Null
        $script:__bm_prev_pwd = $cur
    }
}

# 注册到 prompt hook
if (-not (Get-Variable -Name _bm_hooked -Scope Global -ErrorAction SilentlyContinue)) {
    $Global:_bm_hooked = $true
    $originalPrompt = (Get-Command prompt -ErrorAction SilentlyContinue)?.ScriptBlock
    function prompt {
        __bm_hook
        if ($originalPrompt) { & $originalPrompt } else { "PS $($PWD.Path)> " }
    }
}

# ── Completion ────────────────────────────────────────────

Register-ArgumentCompleter -CommandName @('z','zi','o','oi') -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $candidates = & $__bm_exe bookmark z --list --tsv $wordToComplete 2>$null
    $candidates | ForEach-Object {
        $parts = $_ -split "`t"
        [System.Management.Automation.CompletionResult]::new(
            $parts[0], $parts[0], 'ParameterValue', $parts[1]
        )
    }
}

$__bm_subcommands = @('z','zi','o','oi','open','save','set','delete','tag','pin','unpin','rename','list','recent','stats','check','gc','dedup','export','import','init','touch','learn','keys','all')
Register-ArgumentCompleter -CommandName 'bm' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $tokens = @($commandAst.CommandElements | ForEach-Object { $_.Extent.Text })
    if ($tokens.Count -le 2) {
        $__bm_subcommands |
            Where-Object { $_ -like "$wordToComplete*" } |
            ForEach-Object {
                [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
            }
        return
    }
    $sub = $tokens[1]
    if ($sub -in @('z','zi','o','oi')) {
        $candidates = & $__bm_exe bookmark z --list --tsv $wordToComplete 2>$null
        $candidates | ForEach-Object {
            $parts = $_ -split "`t"
            [System.Management.Automation.CompletionResult]::new(
                $parts[0], $parts[0], 'ParameterValue', $parts[1]
            )
        }
    }
}

# ── 可选 alias（取消注释以启用）─────────────────────────

# Set-Alias cd z   # 用 bm z 完全替代 cd（谨慎使用）
```

### 9.2 Bash init 模板

```bash
# 由 'bm init bash' 生成

# 核心 wrapper
function z() {
    local result
    result=$(xun bookmark z "$@")
    if [[ "$result" == __BM_CD__* ]]; then
        builtin cd "${result#__BM_CD__ }"
    else
        echo "$result"
    fi
}
function zi() { local r=$(xun bookmark zi "$@"); [[ "$r" == __BM_CD__* ]] && builtin cd "${r#__BM_CD__ }"; }
function o()  { xun bookmark o  "$@"; }
function oi() { xun bookmark oi "$@"; }
alias bm='xun bookmark'

# 自动学习 hook
__bm_hook() { xun bookmark learn --path "$PWD" &>/dev/null & }
[[ "$PROMPT_COMMAND" != *__bm_hook* ]] && PROMPT_COMMAND="__bm_hook;${PROMPT_COMMAND}"

# Completion
_bm_query_complete() {
    COMPREPLY=( $(xun bookmark z --list --tsv "${COMP_WORDS[COMP_CWORD]}" 2>/dev/null | cut -f1) )
}
_bm_root_complete() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local sub="${COMP_WORDS[1]}"
    local subcommands="z zi o oi open save set delete tag pin unpin rename list recent stats check gc dedup export import init touch learn keys all"
    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "$subcommands" -- "$cur") )
        return
    fi
    if [[ "$sub" == "z" || "$sub" == "zi" || "$sub" == "o" || "$sub" == "oi" ]]; then
        COMPREPLY=( $(xun bookmark z --list --tsv "$cur" 2>/dev/null | cut -f1) )
        return
    fi
    COMPREPLY=()
}
complete -F _bm_query_complete z zi o oi
complete -F _bm_root_complete bm

# alias cd='z'  # 可选
```

### 9.3 init wrapper 覆盖时机约定

> **关键约定**：Phase 2 完成 PowerShell hook 模板化时，wrapper 必须同时为 `z / zi / o / oi` 预留 `__BM_CD__` 接口。即使 `zi / oi` 本身是 P1 功能，wrapper 也必须在 Phase 2 就位，避免 Phase 3 上线 `zi` 时回头修改 init 模板引入回归。

---

## 10. 交互体验设计

### 10.1 全键盘操作矩阵

| 场景 | 键位 | 行为 |
|---|---|---|
| 快速跳转 | `z foo <Enter>` | 极速 Top-1 cd |
| 交互跳转 | `zi foo <Enter>` | 进入 fzf 选择 |
| fzf 中上下选择 | `↑ / ↓` 或 `Ctrl-P / Ctrl-N` | 移动光标 |
| fzf 中预览 | 自动显示 preview 窗口（目录内容） | |
| fzf 中确认 | `Enter` | 跳转/打开 |
| fzf 中取消 | `Esc` 或 `Ctrl-C` | 退出不操作 |
| 打开文件管理器 | `o foo <Enter>` | 极速 Top-1 open |
| 交互打开 | `oi foo <Enter>` | 进入 fzf 选择后 open |
| Tab 补全 | `z foo<Tab>` | 展开候选（与 `--list` 排序一致） |
| 查看候选 + 得分 | `z foo --list --score` | 打印排序列表及各维度分值 |
| 解释推荐原因 | `z foo --why` | 打印 Top-1 推荐原因 |
| 预览不执行 | `z foo --preview` | Dry-run，不跳转 |

### 10.2 fzf 集成规范

- 最低 fzf 版本：`v0.51.0`（与 zoxide 保持一致，来源：[github.com/ajeetdsouza/zoxide](https://github.com/ajeetdsouza/zoxide)）
- fzf 调用参数模板：

```bash
fzf --height 40% --reverse \
    --preview 'ls -la {2}' \
    --preview-window 'right:40%' \
    --bind 'ctrl-/:toggle-preview'
```

- 支持 `_BM_FZF_OPTS` 环境变量覆盖 fzf 选项

### 10.3 降级选择器规范

当 fzf 不存在时：

1. **首选**：`dialoguer::FuzzySelect`（要求 Windows Terminal ≥ 1.18 / VTE 兼容终端）
2. **降级**：编号列表选择

```
请选择目录（输入编号后按 Enter）：
  1. C:\dev\projects\my-client        [work, rust]  score: 87.3
  2. D:\work\client-api               [work]        score: 61.4
  3. C:\repos\client-tools                          score: 44.2
> _
```

### 10.4 颜色与样式规范

| 元素 | 样式 |
|---|---|
| 路径 | 青色（`\x1b[36m`） |
| 书签名 | 白色加粗 |
| tag | 黄色（`\x1b[33m`） |
| score 数值 | 暗灰色（`\x1b[90m`） |
| 错误/警告 | 红色（`\x1b[31m`） |
| 提示信息 | 暗灰色斜体 |

颜色输出遵循 `NO_COLOR` 环境变量标准。

---

## 11. 数据治理

### 11.1 治理命令职责分工

| 命令 | 触发时机 | 作用对象 |
|---|---|---|
| `bm check` | 手动 / 可定时 | 扫描并报告 missing / stale / duplicate |
| `bm gc` | 手动 | 删除 dead link（可加 `--learned` 只清理 learned） |
| `bm gc --dry-run` | 随时 | 预览将被删除的条目，不执行 |
| `bm dedup` | 手动 | 合并重复路径书签 |
| `z / zi / o / oi` | 每次命中时 | 即时检测死链，超时 300ms 跳过 |

### 11.2 schema_version 策略

```
schema_version = 1  （vNext 新主存储格式，含 source / pinned / tags / desc / workspace）
schema_version = 2+ （vNext 之后的未来版本按需递增）
```

本轮重构允许直接切换到新的 `schema_version = 1` 主存储格式，不以兼容旧格式为目标。后续版本如再扩字段或切换底层实现，再由 `schema_version` 驱动增量迁移。

---

## 12. 分阶段实施路线图

### Phase 1a — 工程收口（无新用户功能）

目标：消灭裂缝，建立可信代码基线。

- [x] 命令面一致性收敛（parser / help / completion / init / dispatch 五处同步）
- [x] 路径标准化全链路接入
- [x] `schema_version` 字段引入

**验收**：用户看到的 help 与 parser 行为严格一致；同路径书签不重复入库。

### Phase 1b — 功能地基

- [x] 统一 query core（`BookmarkQuerySpec` + `bookmark::query`）
- [x] `explicit / imported / learned / pinned` 数据模型
- [x] 多 token + name/path/tag 混合匹配

**验收**：`bm z client api` 稳定返回可解释结果；`bm z` 与 `bm o` 排序一致。

### Phase 2 — Hybrid 成立

- [x] 自动学习目录访问（PowerShell hook 优先）
- [x] 排除目录配置（`_BM_EXCLUDE_DIRS`）
- [x] Shell history 预填充冷启动
- [x] 导入外部生态（autojump / zoxide / z / fasd）
- [x] PowerShell init 模板化（包含 `zi / oi` wrapper 预留接口）

**验收**：进入目录后自动记录 learned；`bm import --from zoxide` 成功导入数据。

### Phase 3 — 体验闭环

- [x] `zi` 交互式跳转（当前为 `dialoguer::FuzzySelect` + 非交互回落）
- [x] `oi` 交互式打开
- [x] 显式范围搜索（`--child / --base / --workspace / --global`）
- [x] `--list / --score / --why`
- [x] 歧义提示
- [x] Completion 与 query core 对齐
- [x] `--preview / --dry-run`
- [x] `bm fuzzy` 已删除

**验收**：`zi` / `oi` 在 Windows Terminal 上流畅运行；`--why` 输出各维度得分。

### Phase 4 — 治理闭环

- [x] dead-link 即时提示（含网络路径超时保护）
- [ ] Dashboard dead / stale / duplicate 视图
- [x] `bm check / gc` 强化
- [x] `desc` 字段 + `recent` 增强
- [x] bash / zsh / fish init 模板

### Phase 5 — 长期性能与迁移

- [x] SQLite 评估
- [ ] SQLite 迁移（触发条件见 SQLite 评估文档）
- [x] 倒排索引（持久化倒排索引 JSON sidecar 已落地；SQLite 索引化后端未做）
- [x] Benchmark 套件（本地已落地，待 CI 化）
- [x] `undo / redo` 机制（当前为 delta-based history）
- [x] `bm fuzzy` 彻底移除
- [x] `--cmd <prefix>` 参数

---

## 13. 验收标准

### 13.1 功能验收

| 场景 | 验收条件 |
|---|---|
| 极速跳转 | `bm z client api` 在 5000 条数据下 P99 < 50ms |
| 多 token 匹配 | `bm z repo docs` / `bm z work rust` 稳定返回可解释 Top-1 |
| 显式书签名称 | `explicit` 条目名称全局唯一；重名 `set` = 更新，`rename` 冲突 = 报错 |
| learned 不覆盖 explicit | 同路径 learned 条目不会把 explicit 书签从 Top-1 挤下 |
| pinned 书签优先 | pinned + explicit 书签在任何查询中排名高于同路径 learned |
| fzf 集成 | `bm zi` 在有 fzf v0.51+ 时调用 fzf，无 fzf 时降级选择器正常工作 |
| dead-link 即时提示 | 命中死链时提示清晰，网络路径检测不超过 300ms |
| Windows 路径 | 大小写不同、分隔符不同的同一路径不重复入库 |
| 命令面一致 | `bm z --help` 与实际 parser 行为完全一致 |
| 导入外部数据 | zoxide / autojump / z 数据导入后可直接参与搜索 |

### 13.2 性能验收

| 指标 | 要求 |
|---|---|
| `bm z` P99 延迟 | < 50ms（Windows + PowerShell，5000 条） |
| `bm zi` 首次渲染 | < 200ms |
| 自动学习 hook | 不阻塞 prompt（异步，< 5ms 影响） |
| completion | < 80ms（Tab 到候选出现） |
| 内存占用 | < 20MB（运行时，不含 fzf） |

当前本机参考结果见 [bookmark-Benchmark-Suite.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Benchmark-Suite.md)：

- `xun bookmark z --list` release 平均约 `41ms`
- `bm z --list` release 平均约 `31ms`
- `xun __complete bookmark z` release 平均约 `44ms`

### 13.3 切换边界验收

| 场景 | 要求 |
|---|---|
| bookmark 命名空间 | `xun bookmark ...` 与 `bm ...` 为唯一正式入口 |
| 旧顶层命令 | `xun z / o / ws / sv / fuzzy / ...` 不再作为公共 CLI |
| 主存储格式 | 直接写入新的 `schema_version = 1` 主存储结构 |
| Dashboard / panel | 暂不纳入本轮验收范围 |

---

## 附录 A — 环境变量参考

| 变量 | 默认值 | 说明 |
|---|---|---|
| `_BM_DATA_FILE` | 见 `bookmark.dataFile` | bookmark 主存储文件路径 |
| `_BM_VISIT_LOG_FILE` | 见 `bookmark.visitLogFile` | bookmark 访问日志路径 |
| `_BM_MAXAGE` | `10000` | learned 数据库老化阈值 |
| `_BM_EXCLUDE_DIRS` | 见 5.1.5 节 | 自动学习排除目录（glob，OS 路径分隔符分隔） |
| `_BM_RESOLVE_SYMLINKS` | `0` | 为 1 时入库前解析 symlink |
| `_BM_ECHO` | `0` | 为 1 时 `z` 跳转前打印目标路径 |
| `_BM_FZF_OPTS` | （空） | 追加到 fzf 调用的自定义选项 |
| `NO_COLOR` | （未设置） | 遵循 no-color.org 标准，设置时关闭颜色输出 |

---

## 附录 B — 竞品参考来源汇总

| 来源 | 用途 |
|---|---|
| [github.com/ajeetdsouza/zoxide](https://github.com/ajeetdsouza/zoxide) | 算法、fzf 版本要求、init 模板设计、`--cmd` 参数 |
| [github.com/ajeetdsouza/zoxide/wiki/Algorithm](https://github.com/ajeetdsouza/zoxide/wiki/Algorithm) | frecency 算法公式与 `_ZO_MAXAGE` 机制 |
| [man.archlinux.org/man/zoxide.1.en](https://man.archlinux.org/man/zoxide.1.en) | 环境变量规范、老化策略参数 |
| [github.com/wting/autojump](https://github.com/wting/autojump) | 多参数查询语义、`jc` 子树模式 |
| [github.com/huyng/bashmarks](https://github.com/huyng/bashmarks) | 显式书签极简设计参考 |
| [github.com/WhoSowSee/whoosh.yazi](https://github.com/WhoSowSee/whoosh.yazi) | 持久/临时书签分层设计、路径截断显示 |
| [github.com/h-hg/yamb.yazi](https://github.com/h-hg/yamb.yazi) | key 直接跳转、fzf 集成方式 |
| [github.com/stelcodes/bunny.yazi](https://github.com/stelcodes/bunny.yazi) | 持久+临时书签 UX 设计 |
| [yazi-rs.github.io/docs/resources](https://yazi-rs.github.io/docs/resources/) | Yazi 书签生态全览 |
| [batsov.com — zoxide tips and tricks](https://batsov.com/articles/2025/06/12/zoxide-tips-and-tricks/) | `j / jj` alias 实践、fzf SPACE 触发补全 |
