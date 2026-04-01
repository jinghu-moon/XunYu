# Bookmark 竞品对标与借鉴分析

> **版本**：2.0 · **更新时间**：2026-03-31  
> 关联文档：Bookmarks-Feature-Roadmap.md · bookmark-PRD.md  
>
> 参考项目：
>
> - `refer/bookmarks/autojump-master`
> - `refer/bookmarks/bashmarks-master`
> - `refer/bookmarks/zoxide-main`

---

## 1. 结论摘要

### 1.0 实现状态同步（2026-03-30）

- 已完成：正式入口收口到 `xun bookmark <sub>` / `bm <sub>`
- 已完成：`z / zi / o / oi` 与 completion 共用 query core
- 已完成：`fuzzy` 公共命令删除
- 已完成：新主存储 `~/.xun.bookmark.json`
- 已完成：持久化倒排索引（JSON sidecar）、delta-based `undo / redo`
- 已完成：bookmark 存储与 completion 逻辑收口到 `src/bookmark/`
- 暂未完成：Dashboard 书签面板、SQLite

当前 `xun` 的 bookmark 组件，在"显式命名书签管理"这条线上，已经明显强于 `bashmarks` 这类基础工具：

- 已有 `set / save / delete / list / z / o / tag / recent / stats / dedup / export / import / pin / unpin`
- 已有 frecency 基础能力
- 已有 dashboard API
- 已有 JSON / TSV 输出与导入导出能力

因此，后续不应再把 bookmark 组件理解成"另一个 bashmarks"。

真正值得借鉴的方向是：

1. 学 `zoxide` 的产品化完成度与 init 模板体系
2. 学 `autojump` 的自动学习与查询语义（**注意：autojump 维护明显减弱，只借鉴思路，不借鉴实现**）
3. 学 yazi 书签插件生态对「命名书签 + 交互选择」场景的验证
4. 保留 `bashmarks` 的极简心智，但不回退到它的简陋数据模型

一句话总结：

- `bashmarks` 值得学"简单"
- `autojump` 值得学"自动收集 + 查询行为"（思路层面）
- `zoxide` 最值得学"工程化与长期维护方式"
- yazi 插件生态验证了"显式命名书签"场景的真实需求

---

## 2. 竞品定位全览

### 2.1 bashmarks

**定位**：极简的 shell 目录书签脚本。

**核心能力**：

- `s` 保存当前目录
- `g` 跳转
- `p` 打印路径
- `d` 删除
- `l` 列表
- tab completion

**数据模型**：

- 单文件 `~/.sdirs`
- 本质是 `export DIR_name="path"` 变量仓库

参考源码：[bashmarks.sh](https://github.com/huyng/bashmarks)

---

### 2.2 autojump

> ⚠️ **维护状态警告**：autojump 维护明显减弱（GitHub repo 在 2025-02 仍有 push，但整体节奏显著弱于 zoxide），在部分环境下存在兼容性问题。**本文档中对 autojump 的所有借鉴均限于思路层面，不建议照搬其实现**。

**定位**：自动学习用户访问目录的"智能跳转器"。

**核心能力**：

- 自动把访问过的目录写入数据库
- `j foo` 跳到最高权重匹配目录
- `jc foo` 优先当前目录子树
- `jo foo` 打开文件管理器而不是 `cd`
- `jco foo` 子树优先 + 打开文件管理器
- 多参数查询，如 `j w in`
- shell hook + tab completion

**数据模型**：

- 文本数据库，维护目录权重
- 通过 shell hook 持续增量学习

参考资料：[github.com/wting/autojump](https://github.com/wting/autojump)

---

### 2.3 zoxide

**定位**：现代化、跨 shell、工程化程度很高的智能 `cd` 工具。当前仍以 0.9.x 系列为主。

**核心能力**：

- 自动学习目录访问
- `z foo` 智能跳转（多 token AND，路径分量语义）
- `zi foo` 交互式选择（依赖 fzf v0.51+）
- 支持多 shell 初始化模板（`init <shell>`）
- 支持从 `autojump` / `z` 导入数据
- 内建数据库治理：aging / dedup / dirty save
- `--cmd <prefix>` 让用户自定义命令前缀（如 `j / ji`）
- 交互补全和 query 行为一致

**数据模型**：

- 自有数据库格式
- 分数、最后访问时间、去重、老化策略一体化管理

参考资料：[github.com/ajeetdsouza/zoxide](https://github.com/ajeetdsouza/zoxide) · [zoxide Wiki — Algorithm](https://github.com/ajeetdsouza/zoxide/wiki/Algorithm)

---

### 2.4 Yazi 书签插件生态

以下三个插件均为 Yazi 文件管理器的书签扩展，**验证了"显式命名书签 + 交互选择"这一细分场景的真实用户需求**。

| 插件 | 核心特性 | 参考价值 |
|---|---|---|
| **whoosh.yazi** | 持久/临时书签分层、目录历史、fzf 模糊搜索、路径截断显示 | 持久+临时双层设计 |
| **yamb.yazi** | 持久书签、key 直接跳转、fzf | key 直接跳转心智 |
| **bunny.yazi** | 持久+临时书签、模糊搜索、前一目录 | 书签 UX 综合设计 |

尽管这三个工具均依赖 Yazi 无法独立使用，但其 UX 设计对 xun bookmark 的交互模式设计有直接参考价值。

参考资料：[yazi-rs.github.io/docs/resources](https://yazi-rs.github.io/docs/resources/)

---

### 2.5 其他参考

| 工具 | 特点 | 参考点 |
|---|---|---|
| **fzf-marks** | fzf + 命名书签 | fzf 与书签结合的基础验证 |
| **rupa/z** | 最早的 frecency 目录跳转 | 数据格式（`rank\|time\|path`）参考 |
| **fasd** | 文件 + 目录混合 frecency | 导入时需注意过滤目录类型 |
| **z.lua** | z 的 Lua 重写，兼容 rupa/z | 数据格式兼容性 |
| **zsh-z** | zsh 原生 z 实现 | 基本兼容 rupa/z 格式 |

---

## 3. 竞品能力对比矩阵

| 能力维度 | bashmarks | autojump | zoxide | **xun bookmark 目标** |
|---|:---:|:---:|:---:|:---:|
| 显式命名书签 | ✓ | ✗ | ✗ | **✓★** |
| 自动目录学习 | ✗ | ✓ | ✓ | **✓** |
| Frecency 排序 | ✗ | ✓ | ✓ | **✓** |
| 多 token 查询 | ✗ | ✓ | ✓ | **✓** |
| 交互式跳转（zi） | ✗ | △ | ✓ | **✓** |
| 交互式打开（oi） | ✗ | △（jco） | ✗ | **✓★** |
| 极速打开文件管理器（o） | ✗ | △（jo） | ✗ | **✓★** |
| Tag 系统 | ✗ | ✗ | ✗ | **✓★** |
| Workspace 范围搜索 | ✗ | ✗ | ✗ | **✓★** |
| 可解释排序（--why） | ✗ | ✗ | ✗ | **✓★** |
| 跨生态数据导入 | ✗ | ✗ | ✓（autojump/z） | **✓** |
| Shell init 模板 | ✗ | △ | ✓ | **✓** |
| Windows / PowerShell | ✗ | ✗ | △ | **✓★（首要平台）** |
| 数据导出 / Dashboard API | ✗ | ✗ | ✗ | **✓★** |
| Pin 置顶 | ✗ | ✗ | ✗ | **✓★** |

> ★ = xun bookmark 独有差异化能力

---

## 4. 值得借鉴的功能点

### 4.1 自动学习目录访问

这是 `autojump` 与 `zoxide` 最有价值、也是 `xun` 当前最缺的能力。

**现状**：

- `xun` 目前主要依赖显式 `set / sv`
- `z / o / workspace / touch` 只是操作已有书签
- 没有"进入目录就自动学习"的主路径

**竞品做法**：

- `autojump` 在 shell 中通过 `PROMPT_COMMAND`、`PWD` 变量或 clink hook 写入访问目录
- `zoxide` 通过 `init <shell>` 模板注入统一 hook，把用户真正进入过的目录喂给数据库

**借鉴建议**：

- 为 `xun bookmark` 增加一条"自动学习目录库"
- 不替代现有显式书签，而是作为补充信号源
- 保持显式书签与自动学习记录分层：
  - 显式书签：用户命名、可打标签、可导出导入、永远不被老化删除
  - 自动学习：用户不命名，仅用于排序和推荐，受 `_BM_MAXAGE` 老化约束
- 冷启动：初次启用时从 shell history 预填充

**优先级**：`P0`

---

### 4.2 四命令对称导航矩阵（最终确认方案）

最终确认的四个核心导航命令：

| 命令 | 动作 | 模式 | 对应 autojump |
|---|---|---|---|
| `z` | `cd` 跳转 | 极速，取 Top-1 | `j` |
| `zi` | `cd` 跳转 | 交互，fzf 选择 | — |
| `o` | 文件管理器打开 | 极速，取 Top-1 | `jo` |
| `oi` | 文件管理器打开 | 交互，fzf 选择 | `jco`（部分对应） |

**设计逻辑**：`i` 后缀统一表示「交互选择模式」。两个动作维度（cd / 打开）× 两个模式维度（极速 / 交互）= 完整 2×2 对称矩阵。

**与 zoxide 的关系**：

- `z / zi` 与 zoxide 完全对齐，熟悉 zoxide 的用户零成本迁移
- `o / oi` 是 xun bookmark 独有的差异化能力，zoxide 没有对应命令

**关于 `open` 命名**：`open` 保留为 `bookmark` 命名空间内的正式子命令，可复用 `o` 的实现，但不再作为旧顶层命令存在。

**优先级**：`zi` → P1，`oi` → P1

---

### 4.3 fzf 集成策略（交互模式后端）

`zi` 和 `oi` 均采用以下后端优先级：

1. 若 `fzf >= v0.51.0` 存在：调用 fzf（最低版本与 zoxide 保持一致）
   - 支持 preview 窗口（显示目录内容）
   - 支持 `_BM_FZF_OPTS` 环境变量覆盖 fzf 选项
2. 否则降级到内置 `dialoguer::FuzzySelect`
   - 要求 Windows Terminal ≥ 1.18
   - 在低版本终端中退化为编号选择（打印候选列表 + 输入数字）

---

### 4.4 支持导入 autojump / zoxide / z 数据

`zoxide` 的 `import` 做得非常成熟：能读旧生态数据库、对不同系统的 rank 做归一化、导入后自动 dedup。

**借鉴建议**：

| 来源 | 数据文件 | 特殊处理 |
|---|---|---|
| autojump | `~/.local/share/autojump/autojump.txt` | 直接读取 |
| zoxide | 按 OS/env 探测标准路径 | 调用 `zoxide query --list --score` 导出文本再解析 |
| rupa/z | `~/.z` | `rank\|time\|path` 格式 |
| z.lua | `~/.zlua` | 类 rupa/z，字段顺序略有差异 |
| zsh-z | `~/.z` | 兼容 rupa/z |
| fasd | `~/.fasd` | **只导入目录类型（d）条目**，文件类型丢弃 |

导入流程：路径标准化 → score 归一化 → dedup → 标记 `source=imported`

**优先级**：`P0`

---

### 4.5 多 token 查询语义

`autojump` 的多参数查询很实用：`j foo`、`j foo bar`、`j w in`。这让用户可以逐步缩小范围，而不是只依赖一个模糊串。

**借鉴建议**：

- 输入先切成 token，token 之间采用 AND 语义
- 不同 token 可以命中不同字段（name / path segment / tag）
- 最后一个 token 对 basename / 末级目录有更高敏感度

**优先级**：`P0`

---

### 4.6 当前目录子树优先（`--child`）

`autojump` 的 `jc` / `jco` 语义很值得借鉴：优先当前目录的子树，很适合大仓库、多工作区场景。

**借鉴建议**：

- `z --child foo`：当前目录子树优先，非子树结果降权
- `o --child foo`：同语义，打开文件管理器
- 这会比现在的隐式 `cwd_boost` 更稳，也更符合用户预期

**优先级**：`P1`

---

### 4.7 shell 初始化模板化

`zoxide` 的 shell 支持方式比 `autojump` 更现代：不是手写零散脚本片段，而是 `init <shell>` 统一模板输出，bash / fish / zsh / powershell 行为一致。

**借鉴建议**：

- 用模板方式统一生成 shell init 片段
- 优先覆盖：PowerShell（首要平台）
- 其次覆盖：bash / zsh / fish
- 输出内容包含：`z / zi / o / oi` wrapper 函数、自动学习 hook、completion、`bm` alias
- 支持 `--cmd <prefix>` 参数（学 zoxide），例如 `bm init --cmd j` 输出 `j / ji / jo / joi` alias
- 在 init 模板中提供注释形式的可选 alias：`alias cd='bm z'`（用户手动启用，不强制）

**优先级**：`P1`

---

### 4.8 数据库治理：aging / dedup / dirty save

`zoxide` 最值得学的不是"会跳"，而是"数据库怎么长期稳定运转"。

**关键机制（参考 zoxide Wiki Algorithm）**：

- 数据库只在 dirty 时写盘（temp file + atomic rename）
- 支持老化（`_ZO_MAXAGE`，默认 10000）：总分超过阈值时等比缩减，得分 < 1 的条目被删除
- 内建 dedup
- 对导入和新增路径做统一治理

**现状**：`xun` 现在已经有 `visit_log`、`frecency aging`、`dedup`，但这些能力还偏"命令功能"，不是整个数据层的系统性设计。

**借鉴建议**：

- `explicit` 和 `pinned` 书签**不参与**老化删除
- 只有 `learned` 和 `imported` 受 `_BM_MAXAGE` 约束
- dirty save 策略：访问增量累计 > N 次，或距上次 save > T 秒，触发 compact & save

**优先级**：`P1`

---

## 5. 值得借鉴的交互设计

### 5.1 极简主命令

`bashmarks` 的优点是命令心智极简：保存、跳转、打印、删除、列表。yazi 插件也印证了这一点：用户最高频的操作就是「跳过去」和「打开」。

**对 xun 的启发**：

- `z / zi / o / oi` 四个核心导航命令必须保持短且直觉
- 不要让 tags / stats / export / import 的复杂度污染主跳转体验
- 管理命令是"需要时找得到"，导航命令是"每天用几十次"

---

### 5.2 "极速模式"和"交互模式"明确分离

`zoxide` 的产品设计优点：快的时候别打断用户；不确定时再进入选择器。

**对 xun 的启发**：

- `z / o` 偏向"零打断、极速"，永远取 Top-1
- `zi / oi` 专门用于"想看候选再选"的场景
- 两个维度都完整，用户心智清晰：「加 i 就是交互选择」

---

### 5.3 打开目录与打开文件管理器分离

`autojump` 区分了 `j`（cd）和 `jo`（文件管理器）。`xun` 的 `z / o` 沿用这一设计，并在两个维度上都提供交互版：`zi / oi`。

**重要约定**：`z / zi` 底层 query 逻辑与 `o / oi` 完全共享同一 query core，差异仅在最终动作。

---

### 5.4 补全和查询一致

`zoxide` 的补全复用 query 逻辑，不是另一套独立逻辑。

**对 xun 的启发**：

- completion 的候选应该和 `z --list` 的结果排序完全一致
- 否则用户会看到：补全推荐 A，真正执行时跳到 B

---

### 5.5 可解释排序（xun 独有差异化）

`zoxide` 没有 `--why` 或 `--score`，xun 应该做到：

- `bm z --list --score`：列出候选及各维度得分
- `bm z --why foo`：解释 Top-1 推荐原因（MatchScore / FrecencyMult / ScopeMult / SourceMult / PinMult）

这既是用户信任的来源，也是开发者调参的工具。

---

## 6. 不建议照搬的部分

### 6.1 不要照搬 bashmarks 的 `.sdirs` 存储格式

本质是 shell 变量文件，不适合 tags / visits / last_visited / dashboard API，不适合做结构化导入导出。只借鉴它的"简单心智"，不借鉴存储实现。

---

### 6.2 不要照搬 autojump 的多套 shell hack

autojump 的实现包含大量 shell 特定机制（`PROMPT_COMMAND`、fish `PWD` 变量 hook、clink lua 等）。维护成本高，且该项目维护明显减弱。学它的"自动学习思路"，不照搬历史兼容包袱。

---

### 6.3 不要退回去做纯导航器

`autojump / zoxide` 都偏目录导航器，没有显式书签、tag、workspace、pin、Dashboard API。

`xun bookmark` 当前的价值不只是"跳得快"，还包括：可管理、可检查、可导出导入、可 Dashboard 化。产品定位必须保持：

> **显式书签系统 + 智能目录导航层**

不是在二者之间二选一。

---

### 6.4 不要强制覆盖 `cd`（参考 zoxide 社区实践）

zoxide 社区反复强调：不强制替换 cd，原因：

- `cd` 是 bash / zsh / fish / PowerShell 的核心 builtin，强制覆盖会导致脚本和子 shell 失效
- RVM / nvm / conda 等工具已在 `cd` 上挂钩，叠加覆盖容易互相踩脚
- 卸载/禁用时残留，恢复麻烦

**正确做法**：在 init 模板里提供注释形式的可选 alias，让用户自己决定是否覆盖 `cd`。

---

## 7. 对当前代码的直接启发

### 7.1 统一 query core 必须早于大部分体验优化

本轮重构前，`z / o / fuzzy / completion` 曾经分裂出两套半逻辑。当前已经统一到 query core，因此后续优化应建立在现有统一内核之上，而不是再回退到并行实现。

---

### 7.2 命令面收敛本身就是 P0 工程任务

当前已经形成的裂缝：

- parser：短命令 `o / ws / sv`
- completion / init / dispatch：长命令 `open / save / workspace`

最终确认的命令面收敛方向：

- 正式入口切换为 `xun bookmark <sub>`
- `bm <sub>` 作为官方短别名
- `z / zi / o / oi / open / save / ...` 全部下沉到 `bookmark` 命名空间
- 旧顶层 `xun z / o / ws / sv / fuzzy / ...` 不再作为公共 CLI
- parser / help / completion / init / dispatch / 文档五处同步更新

---

### 7.3 当前排序公式已经进入 hybrid 形态

当前实现已经落在 `src/bookmark/core.rs` / `src/bookmark/query.rs`，并采用统一乘法形式（各因子均为无量纲系数）：

```text
FinalScore =
  MatchScore
  × FrecencyMult          // 1.0 ~ 1.25，基于 zoxide 时间桶参数
  × ScopeMult             // 1.0 ~ 2.5（--global 时固定为 1.0）
  × SourceMult            // explicit=1.20 / imported=1.05 / learned=1.00
  × PinMult               // pinned=1.50 / normal=1.00
```

> **重要**：`ScopeMult` 是无量纲乘法系数（如 1.0 ~ 2.5），不是绝对加分值（如 +30）。两种形式不可混用，否则 FinalScore 数量级会错乱。所有相关文档均采用此乘法形式。

这比直接重写一套黑盒排序更稳，也更方便做 `--why`。

---

### 7.4 当前 store 可以扩展，不必立即重写

`src/bookmark/storage.rs` 已经提供了 visit log 增量写、aging 阈值、原子保存。建议：

- P0/P1 先扩模型与 query core
- P2 再在有真实性能压力时评估 SQLite / 持久化索引

---

### 7.5 `check / gc / o` 应统一为 dead-link 闭环

当前 `check / gc / o` 三个入口已经存在，后续统一成一条治理链：

- `z / zi / o / oi` 命中 dead link 时即时提示（网络路径检测加 300ms 超时保护）
- `check` 负责批量体检
- `gc` 负责批量清理
- Dashboard 暴露 dead / stale / duplicate 视图

这会成为 xun 明显强于 zoxide 的体验点。

---

### 7.6 历史命令清理

`fuzzy` 是历史遗留命令，与 `z --list` 高度重叠。当前已经从正式 CLI 删除，后续只需要在 release note 和历史迁移说明中保留一次性说明。

---

## 8. 优先级建议（结合审核意见与代码现状）

### P0：先把地基补齐

1. 命令面一致性收敛（parser / help / completion / init / dispatch 五处同步）
2. 统一 query core（`BookmarkQuerySpec` + `bookmark::query`）
3. 多 token + `name / path / tag` 混合匹配
4. `explicit / imported / learned / pinned` 数据模型 + 排分公式升级
5. 自动学习目录访问 + 默认排除目录 + shell history 冷启动预填充
6. 导入外部生态（autojump / zoxide / z / fasd）
7. 路径标准化全链路接入

### P1：做完整体验闭环

1. `zi` 交互式跳转（fzf 优先 + 降级选择器）
2. `oi` 交互式打开（与 `zi` 共享后端策略）
3. 显式范围搜索：`--auto / --global / --child / --base / --workspace`
4. `--list / --score / --why`
5. `--preview`（dry-run 模式）
6. completion 与 query core 对齐
7. 歧义提示（Top-1 与 Top-2 分差 < 15% 时提示）
8. dead-link 主动提示与 Dashboard 视图增强
9. shell init 模板化（PowerShell 优先，含 `bm init --cmd <prefix>`）

### P2：再上长期性能和治理

1. SQLite / 索引化存储（触发条件：> 5000 条或明显延迟）
2. 持久化 segment / tag 索引（JSON sidecar 已落地，后续仅考虑 SQLite 场景下继续增强）
3. `recent` 过滤增强（`--tag / --workspace / --since`）
4. `desc` 备注字段
5. `undo / redo` 变更历史（已切换为 delta-based，已覆盖 `unpin`）
6. schema migration / data versioning
7. benchmark 与回归脚本（同机对比 zoxide，> 5000 条，Windows + PowerShell）
8. Dashboard 与历史说明层面的遗留清理

---

## 9. 最终判断

如果只看"显式书签管理"，xun 当前已经明显超过 bashmarks。  
如果看"现代智能导航体验"，xun 当前仍明显落后于 zoxide，主要差距在：自动学习、shell 集成、query 语义完整度、生态迁移入口。  
如果看"2026 年最有机会形成差异化的新形态"，xun 的位置反而非常好。

最合理的下一步：

> **保留现有显式书签系统作为底盘，补自动学习、统一搜索内核、四命令对称导航矩阵（z/zi/o/oi）、pin、范围语义和生态导入，把 xun 做成真正的 hybrid 书签导航器。**

这是对 2026 年竞品格局最有胜率的路线。

---

## 10. 证据索引

### 内部代码

- `src/bookmark/cli_namespace.rs` / `src/bookmark/cli_commands.rs`：bookmark 正式子命令树（`z / zi / o / oi / open / save / ...`）
- `src/bookmark/commands/navigation.rs`：`cmd_z / cmd_zi / cmd_open / cmd_oi`、交互选择、dead-link 提示、Windows Explorer 打开逻辑
- `src/bookmark/query.rs`：统一 query core、scope、排序与 completion 对齐
- `src/bookmark/core.rs`：匹配、frecency、scope、source、pin 排分公式
- `src/bookmark/storage.rs`：`.xun.bookmark.json`、`.xun.bookmark.visits.jsonl`、schema_version、延迟落盘
- `src/bookmark/completion.rs`：bookmark completion 候选生成逻辑
- `src/util.rs`：`normalize_path()`
- `src/commands/completion/candidates/positionals.rs`：positionals 按 `open` 做候选匹配
- `src/commands/completion/candidates/flags.rs`：flags 按 `open / save` 做规则
- `src/commands/dashboard/handlers/bookmarks.rs`：Dashboard bookmarks API
- `src/bookmark/commands/maintenance/check.rs`：missing / stale / duplicate 扫描
- `src/bookmark/commands/maintenance/cleanup.rs`：dead link 清理

### 外部资料

- zoxide README：<https://github.com/ajeetdsouza/zoxide/blob/main/README.md>
- zoxide Wiki — Algorithm：<https://github.com/ajeetdsouza/zoxide/wiki/Algorithm>
- zoxide releases：<https://github.com/ajeetdsouza/zoxide/releases>
- autojump README：<https://github.com/wting/autojump/blob/master/README.md>（维护明显减弱）
- `rupa/z` repo：<https://github.com/rupa/z>
- `fasd` repo：<https://github.com/clvv/fasd>
- `z.lua` README：<https://github.com/skywind3000/z.lua/blob/master/README.md>
- `zsh-z` README：<https://github.com/agkozak/zsh-z/blob/master/README.md>
- whoosh.yazi：<https://github.com/WhoSowSee/whoosh.yazi>
- yamb.yazi：<https://github.com/h-hg/yamb.yazi>
- bunny.yazi：<https://github.com/stelcodes/bunny.yazi>
- Yazi 插件生态：<https://yazi-rs.github.io/docs/resources/>
- zoxide tips and tricks（batsov.com）：参考 `j / jj` alias 实践
