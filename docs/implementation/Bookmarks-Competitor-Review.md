# [已废弃] Bookmarks 竞品对标与借鉴分析（2026-03 复核 + 审核整合版）

> 状态：**已废弃，仅供历史参考**
>
> 说明：
>
> - 本文档基于旧的顶层命令结构（`xun z / o / ws / sv / ...`）
> - 当前 bookmark 组件的正式方案已迁移到 `docs/implementation/bookmark/`
> - 请以以下文档为准：
>   - [Bookmark 竞品对标与借鉴分析](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/Bookmarks-Competitor-Review.md)
>   - [Bookmark 功能路线图](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/Bookmarks-Feature-Roadmap.md)
>   - [xun bookmark PRD](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-PRD.md)

> 更新时间：2026-03-29
>
> 本轮复核依据：
>
> - 当前代码：`src/cli/bookmarks.rs`、`src/commands/bookmarks/*`、`src/fuzzy.rs`、`src/store.rs`、`src/util.rs`、`src/path_guard/*`、`src/commands/completion/*`、`src/commands/dashboard/handlers/bookmarks.rs`
> - 官方/公开资料：zoxide README / release / repo metadata、autojump README / repo metadata、`rupa/z`、`fasd`、`z.lua`、`zsh-z`
> - 复核方式：网页检索 + 官方 GitHub 页面 + GitHub API metadata（截至 2026-03-29）

---

## 1. 执行摘要

### 1.1 先校准当前代码现状

在讨论竞品与路线之前，必须先把 `xun` 当前 bookmarks 子系统的真实基线说清楚：

1. **当前真实 CLI 命令面仍是短命令优先**
   `src/cli/bookmarks.rs` 解析器实际暴露的是：
   `z / o / ws / sv / set / list / recent / stats / dedup / export / import / check / gc / touch / rename / tag / fuzzy / keys / all`

   这意味着：

   - `open / save / workspace` 目前**不是 parser 层正式子命令**
   - 但 completion / init / dispatch 的静态子命令表已经出现了 `open / save`
   - 当前存在一条真实的“命令面对外暴露不一致”问题链

2. **`z` 与 `o` 已经共用同一套搜索入口，但 `z` 仍内嵌交互选择**
   `src/commands/bookmarks/navigation.rs` 中，`cmd_z` 与 `cmd_open` 都调用 `FuzzyIndex::search(...)`；
   但 `cmd_z` 在多结果且可交互时，会直接进入 `dialoguer::FuzzySelect`。

   结论很明确：

   - 项目已经具备“统一搜索层”的雏形
   - 但还没有完成 `z` / `zi` 的动作分离

3. **当前搜索主语义仍然是“按 bookmark name fuzzy”**
   `src/fuzzy.rs` 当前索引和匹配核心只围绕：

   - `name`
   - `tag` 过滤
   - `frecency`
   - `cwd_boost`

   当前主排序公式实际是：

   ```text
   combined = fuzzy_score * (1 + frecency * 0.15) * cwd_boost
   ```

   也就是说：

   - 还没有多 token AND
   - 路径 basename / segment 还没有进入主匹配语义
   - tag 还只是过滤条件，不是主检索字段

4. **当前已经有一套轻量的 delayed write + aging 雏形**
   `src/store.rs` 当前存储模型为：

   - 主库：`~/.xun.json`
   - 访问日志：`visits.jsonl`
   - `append_visit()` 先写增量日志
   - 日志超过 `64 KiB` 再触发回灌保存
   - `FRECENCY_MAX_AGE = 10_000`
   - 保存路径使用 temp file + rename

   这说明未来应该做的是：

   - 扩展当前 store 思路
   - 而不是把现有模型简单视为“必须推翻的历史包袱”

5. **当前显式书签体系已经相当完整**
   当前已具备：

   - 显式命名书签：`sv / set / rename / delete --bookmark`
   - 标签：`tag add / remove / list / rename`
   - 统计与审计：`recent / stats / check / gc / dedup`
   - 机器输出：`all / fuzzy / export / import`
   - Dashboard API：list / upsert / delete / rename / import / export / batch
   - Windows 原生 `o` 行为：目录走 `explorer.exe`，文件走 `cmd /C start`

6. **当前路径治理有基础，但 bookmarks 还没有形成全链路标准化**
   当前代码现状是：

   - `set` 会通过 `PathPolicy::for_output()` 做路径校验
   - `util::normalize_path()` 已用于去重/检查
   - 但 imported / explicit / future learned 数据还没有统一 canonical normalization pipeline

一句话总结当前基线：

> **xun 当前已经是一个“显式书签系统”，但距离“现代智能导航器”仍差自动学习、统一 query core、多 token 路径语义和命令面收敛这几块地基。**

### 1.2 本轮复核后的总体判断

对标 2026 年目录导航赛道，最值得直接对标的对象仍然是 **zoxide**。

原因不是它功能最多，而是它在下面四点上依然最成熟：

- 自动学习目录访问
- 多 token / 路径语义搜索
- `z` / `zi` 分层心智
- `init <shell>` + import + aging + exclusion 的工程闭环

同时，`xun` 的最佳胜位也非常明确：

> **不要把 xun 做成 zoxide 的克隆，而是把它做成“显式书签系统 + 智能导航层”的 hybrid。**

这条定位本轮复核后不但没有削弱，反而更确定了。

### 1.3 本轮审核建议中应当直接吸收的结论

结合竞品复核和当前代码现状，本轮建议里有几条应该直接写进后续设计：

1. **`pin / high-priority` 必须前置到 P0**
   learned 引入后，如果没有 pin，用户核心目录会被短期热点目录挤掉。

2. **噪声排除必须和自动学习一起上线**
   不能先上 hook、后补 exclusion，否则 `node_modules / dist / build / target / tmp` 会快速污染 learned 库。

3. **保留 `o` 这条独立动作链**
   当前代码已经有 Windows 原生 `open` 行为，后续可以补 `open` 长别名，但不应该把“打开”重新折回一个独立实现。

4. **dead-link 批量治理应建立在现有 `check / gc / o` 上**
   当前已经有缺失检测、批量清理、命中时提示，不需要再为了“像别家”新造一组低价值同义命令。

5. **benchmark 必须与 zoxide 在同机环境对齐**
   至少覆盖：
   - `>5000` 条记录
   - Windows + PowerShell
   - `z` / `zi` / completion

6. **`--preview / --dry-run` 值得纳入体验闭环**
   特别是在 query core 改造初期，它能降低用户切换新匹配逻辑时的心理成本。

---

## 2. 2026 年竞品格局

### 2.1 最新竞品信号总表

| 工具 | 2026-03 复核信号 | 结论 |
|------|------|------|
| zoxide | 最新 release `v0.9.9` 发布于 **2026-01-31**；repo 最近 push 为 **2026-03-23** | 事实标准，主对标对象 |
| autojump | repo 最近 push 为 **2025-02-27**；README 语义仍完整；无新的 GitHub Release 闭环 | 语义参考仍有价值，工程参考价值下降 |
| `rupa/z` | 最新 release `v1.12` 发布于 **2023-12-09**；repo 最近 push 为 **2024-06-19** | 历史祖师爷，活跃度低 |
| fasd | GitHub 已标记 `archived=true`；repo 最近 push 为 **2020-06-04** | 仅剩历史思路参考 |
| z.lua | 最新 release `v1.8.25` 发布于 **2026-03-09**；repo 最近 push 同日 | 活跃、跨平台、Windows 参考价值高 |
| zsh-z | repo 最近 push 为 **2026-02-24**；README 仍强调 frecency completion 与 Zsh 原生实现 | Zsh 生态与 smartcase / completion 细节样本 |

### 2.2 zoxide：事实标准，主对标对象

截至 2026-03，zoxide 仍然是这一赛道最强、最稳的参考系：

- 最新 release：`v0.9.9`
- 发布时间：**2026-01-31**
- repo 最近 push：**2026-03-23**
- README 主心智仍然是：
  - `z foo`
  - `z foo bar`
  - `zi foo`
  - `zoxide init <shell>`
  - `zoxide import --from=autojump`
  - `zoxide import --from=z`
- README 明确暴露：
  - `_ZO_EXCLUDE_DIRS`
  - `_ZO_MAXAGE`
  - `fzf` 交互选择

最重要的是：

> **zoxide 代表的是“隐式学习 + 极简导航”的完成态，而不是“显式管理”的完成态。**

它强在：

- 快
- 稳
- 心智简单
- 搜索和 shell 集成一致

但它也明显缺：

- 显式命名书签
- 标签
- pin / favorite
- Dashboard
- dead-link 审计与批量治理
- 结构化的显式管理体验

这正是 xun 的机会窗口。

### 2.3 autojump：语义样本仍有价值，但不再适合作为长期主对标

autojump 到 2026 年的现实状态是：

- README 语义仍然很有代表性：
  - `j foo`
  - `jc foo`
  - `jo foo`
  - `j w in`
- repo 最近 push：**2025-02-27**
- 最新 tags 仍停留在 `release-v22.5.3` 这一代
- README 继续把不少平台/外壳能力描述为 community supported

因此它的合理定位是：

- **学查询语义**
- **学 shell hook 入口心智**
- **不学整体工程路线**

### 2.4 `rupa/z`：历史基线，不再是现代上限

`rupa/z` 仍然重要，但重要性主要在历史和兼容层：

- 最新 release：`v1.12`
- 发布时间：**2023-12-09**
- repo 最近 push：**2024-06-19**

这说明它并不是“完全无人维护”，但确实已经不是节奏活跃的现代参考系。

它更适合提供：

- 历史 frecency 语义参考
- `.z` 数据兼容参考
- 生态迁移兼容参考

不适合作为产品形态上限。

### 2.5 fasd：已归档，只剩思路价值

fasd 当前 GitHub metadata 明确显示：

- `archived = true`
- repo 最近 push：**2020-06-04**

所以 fasd 的合理使用方式是：

- 借鉴“文件与目录统一 frecency”的历史思路
- 不再作为主路线参考

### 2.6 z.lua：仍活跃，且对 Windows / 脚本化场景很有参考价值

z.lua 本轮复核结果：

- 最新 release：`v1.8.25`
- 发布时间：**2026-03-09**
- repo 最近 push：**2026-03-09**
- README 继续强调：
  - Windows / PowerShell / cmd / Nushell 支持
  - `fzf` 集成
  - 多参数匹配
  - interactive 选择
  - enhanced matching

这对 xun 的启发非常直接：

- Windows 不是边角场景
- 脚本型项目也可以把跨 shell 体验做得很好
- `fzf + enhanced matching + multi-shell init` 是用户已经接受的现代基线

### 2.7 zsh-z：适合借鉴 completion / smartcase / keep dirs 细节

zsh-z 本轮复核结果：

- repo 最近 push：**2026-02-24**
- README 继续强调：
  - tab completion 按 frecency 排序
  - `smartcase`
  - `trailing slash`
  - `keep dirs`
  - `exclude dirs`

它对 xun 的主要价值不是全平台路线，而是：

- Zsh 原生 completion 排序一致性
- smartcase / trailing slash 等高级匹配细节
- keep dirs / exclude dirs 之类边界条件治理

---

## 3. 关键能力对比（按产品决策相关性）

| 能力 | xun 当前 | zoxide | autojump | 说明 |
|------|------|------|------|------|
| 显式命名书签 | ✅ | ❌ | ❌ | xun 原生优势 |
| 标签 | ✅ | ❌ | ❌ | xun 原生优势 |
| Dashboard API | ✅ | ❌ | ❌ | xun 原生优势 |
| JSON / TSV 导出导入 | ✅ | ◑ | ❌ | xun 更偏“可管理系统” |
| 自动学习目录访问 | ❌ | ✅ | ✅ | xun 当前最大缺口 |
| `z` / `zi` 分离 | ❌ | ✅ | ◑ | xun 当前 `z` 仍内嵌交互 |
| 多 token 查询 | ❌ | ✅ | ✅ | xun 当前未实现 |
| 路径主语义匹配 | ❌ | ✅ | ✅ | xun 当前仍以 name fuzzy 为主 |
| 导入旧生态数据库 | ❌ | ✅ | ❌ | zoxide 迁移成本最低 |
| `pin / high-priority` | ❌ | ❌ | ❌ | xun 有机会成为差异化卖点 |
| completion 与 query 一致 | ❌ | ✅ | ◑ | xun 当前 completion 独立 |
| 显式死链检查 / 清理 | ✅ | ◑ | ◑ | xun 具备明显治理优势 |
| `--score / --why` 可解释排序 | ❌ | ◑ | ❌ | xun 值得重点补 |
| `--preview / --dry-run` 查询演练 | ❌ | ❌ | ❌ | xun 可做体验护栏 |
| Windows 路径治理 | ◑ | ✅ | ◑ | xun 有 path_guard 基础，但未全链路接入 |

结论仍然很清楚：

- **xun 赢在显式管理层**
- **zoxide 赢在智能导航层与工程闭环**
- **xun 的正确方向不是二选一，而是融合两者**

---

## 4. xun 当前真正已经做对的事

### 4.1 显式书签模型已经明显强于大多数导航器

当前 `xun` 已经具备：

- 命名书签
- 标签
- recent / stats
- check / gc / dedup
- import / export
- Dashboard CRUD / batch

这已经不是“会跳目录的小工具”，而是一套可治理的书签系统。

### 4.2 `z` / `o` 共用搜索逻辑的方向是对的

`cmd_z` 与 `cmd_open` 都直接走 `FuzzyIndex::search`，这说明项目已经在朝：

> **统一搜索层，分离动作层**

这条正确方向前进。

缺的不是方向，而是收口范围：

- completion 还没并进来
- `zi` 还不存在
- `fuzzy` 还没有收敛为 query core 的一种输出模式

### 4.3 现有 store 足以支撑第一阶段 hybrid 改造

当前 `src/store.rs` 已经给了三个非常有价值的基础：

- 主库与访问日志分离
- aging 阈值
- temp file + rename 的原子保存思路

这意味着：

- P0/P1 先不需要因为“想做智能导航”就立即切 SQLite
- 可以先把 source / learned / import / query core 跑通
- 再在 P2 决定是否升级到底层索引化存储

### 4.4 `check / gc / o` 已经构成 dead-link 闭环的起点

当前已有：

- `check`：missing / stale / duplicate 报告
- `gc`：dead link 批量清理
- `o`：命中不存在路径时给修复提示

这组能力是 xun 和大多数导航器之间最容易拉开差距的地方，后续应该强化，不应该弱化。

### 4.5 `o` 已经是一个正确的独立动作样本

当前 `o` 并不是“多余命令”，而是很有价值的动作分层：

- `z`：跳转
- `o`：打开
- `ws`：成组打开

这和未来推荐的“统一 query core，动作层分离”完全一致。

---

## 5. 当前最关键的缺口

### 5.1 还没有自动学习目录访问

这是最大缺口，没有之一。

没有 learned source，就很难做到：

- 冷启动后自然变好用
- 不经手工保存也能导航
- 真正意义上的智能推荐
- 与 zoxide 同频使用

### 5.2 当前搜索本质仍是“搜名字”

当前 `src/fuzzy.rs` 的索引对象就是 `bookmark name`。

这意味着当前搜索体验本质上还是：

- 搜名字
- 用 tag 过滤
- 用 frecency 和 cwd 做排序修正

而不是：

- 搜目录语义
- 搜路径结构
- 搜 tag 语义
- 搜 workspace 范围

### 5.3 completion / init / dispatch / parser 之间存在真实裂缝

当前代码里可以明确观察到：

- parser：`src/cli/bookmarks.rs` 只认 `o / ws / sv`
- completion：`src/commands/completion.rs`、`shell_powershell.rs`、`shell_bash.rs` 静态列出了 `open / save`
- completion candidates：`flags.rs`、`positionals.rs` 也按 `open / save` 写了规则
- init / dispatch：`src/commands/dispatch/core.rs` 同样把 `open / save` 当成正式子命令

这不是文档小问题，而是工程一致性问题。

### 5.4 还没有 source 语义与 pin 语义

如果要做 hybrid 模型，至少需要区分：

- `explicit`
- `imported`
- `learned`

同时还需要：

- `pinned`

否则下面这些都会变得不稳：

- 显式书签优先级
- imported 数据治理
- learned 数据清理策略
- Dashboard 来源展示
- 用户对“为什么这个排第一”的信任感

### 5.5 还没有可解释的排序与低风险演练入口

当前没有：

- `--list --score`
- `--why`
- `--preview / --dry-run`

这会导致 query core 改造期间：

- 调参困难
- 用户难以理解排序变化
- 新行为上线风险更高

---

## 6. 最值得直接吸收的设计点

### 6.1 自动学习 + 排除列表必须一起落地

优先借鉴 zoxide / zsh-z / z.lua：

- `init <shell>` 输出 hook
- 默认通过 prompt / cwd 变更采集真实进入目录
- 支持 `_ZO_EXCLUDE_DIRS` 风格的 glob 配置
- learned 数据单独建 source

对 xun 的建议：

- PowerShell 先做
- 后续补 bash / zsh / fish
- 默认排除：
  - `node_modules`
  - `dist`
  - `build`
  - `target`
  - `tmp`
  - 系统临时目录
- 支持用户级 glob 配置

### 6.2 `z` / `zi` / `o(open)` 应保持动作层清晰分离

建议明确收敛成：

- `z`：极速 Top-1，不打断
- `zi`：交互式选择
- `o`：打开最佳匹配
- `open`：如要提供，仅作为 `o` 的长别名，不单独实现第二套逻辑

当前代码已经给出了正确雏形：

- `z` 和 `o` 共用搜索
- `o` 有独立动作

后续要做的是把 `z` 内嵌交互逻辑抽走给 `zi`，而不是把动作层重新打散。

### 6.3 多 token AND + `name/path/tag` 混合匹配

这是体验飞跃最大的单项。

建议把搜索对象从“仅 name”升级为：

1. `name`
2. `path basename`
3. `path segment`
4. `tag`

并采用 AND 语义：

- token 可分散命中不同字段
- 最后一个 token 对 basename / 末级目录更敏感

### 6.4 显式范围控制，默认 `--auto`

建议直接引入：

- `--auto`
- `--global`
- `--child`
- `--base <dir>`
- `--workspace <name|path>`
- `--exclude-cwd`

其中：

- `--auto` 作为默认行为，兼容当前 `cwd_boost`
- 其他 scope 作为显式语义增强，而不是替代品

### 6.5 导入生态按“兼容面”设计，而不是按单工具打补丁

建议至少覆盖：

- `autojump`
- `zoxide`
- `z-compatible`

其中 `z-compatible` 一次覆盖：

- `rupa/z`
- `fasd`
- `z.lua`
- `zsh-z`

这样迁移设计会更接近 zoxide 的完成度。

### 6.6 `pin / high-priority` 应立即纳入地基设计

这条不该拖到后期。

原因是：

- xun 的定位不是“完全把选择权交给推荐系统”
- 显式书签本身就是用户主动表达偏好的结果
- learned 引入后，没有 pin 会直接破坏用户信任

推荐做法：

- `pinned: bool`
- 排序时单独常量 boost
- Dashboard / list / zi 中有清晰标识

### 6.7 `--score / --why`、歧义提示和 `--preview`

这是 xun 比竞品更容易做好的体验项。

建议直接支持：

- `z --list`
- `z --list --score`
- `z --why foo`
- `z --preview foo`

并在 Top-1 与 Top-2 分差过小时提示：

- “候选接近，可使用 `zi foo` 查看”

### 6.8 completion 与 query core 统一

这不是锦上添花，而是工程上必须做的收敛。

长期目标应该是：

- `z`
- `zi`
- `o`
- `fuzzy`（过渡期）
- completion

都来自同一个 query pipeline，只是输出形态不同。

### 6.9 dead-link 批量治理应增强现有链路，而不是新增同义命令

建议把当前能力收敛成一条完整治理链：

- 查询命中 dead link
- 即时提示删除 / 更新 / 相似候选
- `check` 做批量扫描
- `gc` 做批量清理
- Dashboard 展示 dead / stale / duplicate 视图

这里不建议再新增一套“bookmarks check / bookmarks gc”同义命令。

原因很简单：

- 当前已有 `check / gc`
- 再造一层只会扩张命令面复杂度
- 与 KISS / YAGNI 不一致

---

## 7. 不建议照搬的部分

### 7.1 不要照搬 bashmarks 的存储模型

只借鉴“简单心智”，不借鉴 `~/.sdirs` 这类纯环境变量导出方案。

### 7.2 不要照搬 autojump 的历史 shell 包袱

只借鉴：

- hook 思路
- `j / jc / jo / jco` 的动作分层
- 多参数查询心智

不借鉴大量 shell 特化兼容本身。

### 7.3 不要退回去做纯导航器

xun 当前的价值，不只是“跳得快”，还包括：

- 可管理
- 可检查
- 可导出导入
- 可 Dashboard 化

所以产品定位必须保持：

> **显式书签系统 + 智能目录导航层**

不是在二者之间二选一。

---

## 8. 对当前代码的直接启发

### 8.1 统一 query core 必须早于大部分体验优化

原因很现实：

- 现在 `z / o / fuzzy / completion` 已经分裂出两套半逻辑
- 如果先加 `zi / --child / --score / --why / import / learned`，后面一定返工

### 8.2 命令面收敛本身就是 P0 工程任务

当前已经形成的裂缝是：

- parser：短命令
- completion / init / dispatch：长命令

因此后续如果要引入：

- `open`
- `save`
- `workspace`
- `zi`

最稳妥的方式是：

1. 保留 `o / sv / ws` 兼容
2. 决定是否新增长别名
3. 同步修正 parser / help / completion / init / dispatch / 文档

### 8.3 当前排序公式可以直接作为 hybrid 排序的 baseline

当前 `src/fuzzy.rs` 的排序公式已经给了一个很好的起点：

```text
FinalScore = MatchScore * (1 + Frecency * W) * ScopeBoost
```

后续 hybrid 第一版可以自然扩展为：

```text
FinalScore =
  MatchScore
  * (1 + Frecency * Wf)
  * ScopeBoost
  * SourceBoost
  * PinBoost
```

这比直接重写一套黑盒排序更稳，也更方便做 `--why`。

### 8.4 当前 store 可以扩展，不必立即重写

`src/store.rs` 已经提供了：

- visit log 增量写
- aging 阈值
- 原子保存

所以建议：

- P0/P1 先扩模型与 query core
- P2 再在有真实性能压力时评估 SQLite / 倒排索引

### 8.5 `check / gc / o` 应统一为 dead-link 闭环

当前这三个入口已经存在，后续只需要把它们统一成一条治理链：

- 查询命中 dead link
- 即时提示删除 / 更新 / 相似候选
- `check` 负责批量体检
- `gc` 负责批量清理
- Dashboard 暴露 dead / stale / duplicate

这会成为 xun 明显强于 zoxide 的体验点。

### 8.6 `o` 这条动作链不应被浪费

当前 `o` 已经完成了：

- 搜索
- 命中
- 缺失路径提示
- Windows 打开行为

未来正确做法是：

- 保留 `o`
- 如有需要再增加 `open` 作为别名
- 不要重造一条与 `o` 并行的“打开”实现

---

## 9. 优先级建议（结合审核意见与代码现状）

### P0：先把地基补齐

1. 命令面一致性收敛
2. 统一 query core
3. 多 token + `name/path/tag` 混合匹配
4. `explicit / imported / learned / pinned` 数据模型
5. 自动学习目录访问 + 默认排除目录
6. 导入 autojump / zoxide / z-compatible 生态
7. 路径标准化全链路接入

### P1：做完整体验闭环

1. `zi`
2. 范围搜索：`--auto / --global / --child / --base / --workspace`
3. `--list / --score / --why`
4. `--preview / --dry-run`
5. completion 与 query core 对齐
6. 歧义提示
7. dead-link 主动提示与 Dashboard 暴露
8. shell init 模板化增强

### P2：再上长期性能和治理

1. SQLite / 索引化存储
2. segment / tag 倒排索引
3. recent 过滤增强
4. 备注 / 描述
5. undo / 变更历史
6. schema migration / data versioning
7. benchmark 与回归脚本：
   - 同机环境对比 zoxide
   - `>5000` 条记录
   - Windows + PowerShell
   - `z / zi / completion`

---

## 10. 最终判断

如果只看“显式书签管理”，xun 当前已经明显超过 bashmarks。  
如果看“现代智能导航体验”，xun 当前仍明显落后于 zoxide。  
如果看“2026 年最有机会形成差异化的新形态”，xun 的位置反而非常好。

最合理的下一步不是继续堆低价值 CRUD，而是：

> **保留现有显式书签系统作为底盘，再补自动学习、统一搜索内核、范围语义、交互入口、pin 和生态导入，把 xun 做成真正的 hybrid 书签导航器。**

这是对 2026 年竞品格局最有胜率的路线。

---

## 11. 证据索引

### 内部代码

- `src/cli/bookmarks.rs`：当前 bookmarks CLI 子命令真实命名（`z / o / ws / sv / fuzzy ...`）
- `src/commands/bookmarks/navigation.rs`：`cmd_z`、`cmd_open`、交互选择、dead-link 提示、Windows Explorer 打开逻辑
- `src/fuzzy.rs`：name-only fuzzy、frecency、cwd boost、tag 过滤、当前排序公式
- `src/store.rs`：`.xun.json`、`visits.jsonl`、aging、延迟落盘
- `src/util.rs`：`normalize_path()`
- `src/commands/completion.rs`：completion 静态子命令表包含 `open / save`
- `src/commands/completion/shell_powershell.rs`：PowerShell completion 子命令表与 `o` 单独注册
- `src/commands/completion/shell_bash.rs`：Bash completion 子命令表包含 `open / save`
- `src/commands/completion/candidates/positionals.rs`：positionals 按 `open` 做候选匹配
- `src/commands/completion/candidates/flags.rs`：flags 按 `open / save` 做规则
- `src/commands/dispatch/core.rs`：init / dispatch 静态子命令表包含 `open / save`
- `src/commands/dashboard/handlers/bookmarks.rs`：Dashboard bookmarks API
- `src/commands/bookmarks/maintenance/check.rs`：missing / stale / duplicate 扫描
- `src/commands/bookmarks/maintenance/cleanup.rs`：dead link 清理

### 外部资料

- zoxide README：<https://github.com/ajeetdsouza/zoxide/blob/main/README.md>
- zoxide `v0.9.9` release：<https://github.com/ajeetdsouza/zoxide/releases/tag/v0.9.9>
- zoxide repo metadata：<https://api.github.com/repos/ajeetdsouza/zoxide>
- autojump README：<https://github.com/wting/autojump/blob/master/README.md>
- autojump repo metadata：<https://api.github.com/repos/wting/autojump>
- `rupa/z` repo：<https://github.com/rupa/z>
- `rupa/z` `v1.12` release：<https://github.com/rupa/z/releases/tag/v1.12>
- `rupa/z` repo metadata：<https://api.github.com/repos/rupa/z>
- fasd repo：<https://github.com/clvv/fasd>
- fasd repo metadata：<https://api.github.com/repos/clvv/fasd>
- z.lua README：<https://github.com/skywind3000/z.lua/blob/master/README.md>
- z.lua `v1.8.25` release：<https://github.com/skywind3000/z.lua/releases/tag/v1.8.25>
- z.lua repo metadata：<https://api.github.com/repos/skywind3000/z.lua>
- zsh-z README：<https://github.com/agkozak/zsh-z/blob/master/README.md>
- zsh-z repo metadata：<https://api.github.com/repos/agkozak/zsh-z>
