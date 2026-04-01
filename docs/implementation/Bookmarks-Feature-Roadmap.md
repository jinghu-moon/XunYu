# [已废弃] Bookmarks 功能路线图（2026-03 复核 + 审核整合版）

> 状态：**已废弃，仅供历史参考**
>
> 说明：
>
> - 本文档基于旧的顶层命令结构与渐进式收口前提
> - 当前 bookmark 组件的正式方案已迁移到 `docs/implementation/bookmark/`
> - 请以以下文档为准：
>   - [Bookmark 功能路线图](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/Bookmarks-Feature-Roadmap.md)
>   - [Bookmark 竞品对标与借鉴分析](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/Bookmarks-Competitor-Review.md)
>   - [xun bookmark PRD](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-PRD.md)

> 关联分析文档：
>
> - [Bookmarks-Competitor-Review.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Bookmarks-Competitor-Review.md)
>
> 更新时间：2026-03-29

---

## 1. 目标

这份路线图用于把 bookmarks 组件后续值得做的能力收敛成**可执行、可迁移、可验证**的优先级计划。

总方向保持不变：

> **把当前“显式书签管理器”，升级为“显式书签系统 + 智能目录导航层”。**

但本次整合审核意见后，路线图需要再补三条硬要求：

1. 建立在**当前代码事实**之上，而不是理想命令面之上
2. 建立在**2026 年竞品最新状态**之上，而不是历史印象之上
3. 每一阶段都必须同时考虑：
   - 用户心智
   - 工程一致性
   - Windows + PowerShell 主场景

---

## 2. 本轮调整重点

相较上一版路线图，本轮需要明确前置的变化如下：

### 2.1 `pin / high-priority` 提前到 P0

原因：

- hybrid 模型一旦引入 learned source，没有 pin 就没有用户可控性
- 显式书签本身就是用户主动表达偏好的结果
- 这不是锦上添花，而是排序可信度地基

### 2.2 `o` 保留为独立动作链，`open` 只考虑做兼容别名

原因：

- 当前代码已经有 `cmd_open`
- 当前 Windows 打开逻辑已正确分离
- 不应为了“看起来更像竞品”而重造一条并行实现

### 2.3 dead-link 批量治理应增强现有 `check / gc / o`

原因：

- 当前已有缺失扫描、清理、命中时提示
- 再造新命令只会扩大命令面
- 不符合 KISS / YAGNI

### 2.4 `--auto` 与 `--preview` 应进入正式设计

原因：

- `--auto` 能平滑承接当前 `cwd_boost` 语义
- `--preview` 能在 query core 改造期间显著降低试错成本

### 2.5 benchmark 规格需要显式写死

至少要求：

- 同机环境与 zoxide 对比
- 数据规模 `>5000` 条
- Windows + PowerShell
- 覆盖 `z` / `zi` / completion

---

## 3. 当前代码基线（作为路线图前置约束）

### 3.1 当前真实命令面

当前 parser 真实支持的是：

- `z`
- `o`
- `ws`
- `sv`
- `set`
- `list / recent / stats / dedup / export / import / check / gc / touch / rename / tag / fuzzy / keys / all`

这意味着：

- `open / save / workspace` 目前还不是 parser 层正式长命令
- 后续如果要主推长命令，必须同时修改：
  - CLI parser
  - help
  - completion
  - shell init wrapper
  - 文档

### 3.2 当前搜索实现

当前 bookmarks 搜索链路的关键现状：

- `z` 与 `o` 已共用 `FuzzyIndex::search`
- `z` 当前在多结果且可交互时会直接弹 `dialoguer::FuzzySelect`
- `fuzzy` 仍是单独 machine output 子命令
- completion 仍是独立的 prefix + frecency + cwd boost 逻辑

当前主排序模型等价于：

```text
FinalScore = MatchScore * (1 + Frecency * 0.15) * ScopeBoost
```

其中 `ScopeBoost` 当前只体现为 `cwd_boost`。

### 3.3 当前数据层

当前 bookmarks 数据层已经具备基础治理能力：

- 主库存储：`~/.xun.json`
- 访问日志：`visits.jsonl`
- aging：`FRECENCY_MAX_AGE = 10_000`
- 保存：主库 temp file + rename
- 访问记录：append log，超过阈值后回灌保存

结论：

- 后续应**扩展当前 store 思路**
- 不应在 P0/P1 就假设“必须先切 SQLite”

### 3.4 当前治理能力

当前已经具备：

- `check`：missing / stale / duplicate
- `gc`：dead link 清理
- `o`：命中 dead link 时给出修复提示
- Dashboard bookmarks CRUD / import / export / batch

结论：

- 后续路线图不应忽视这些现成能力
- 应把它们纳入 hybrid 系统的正式治理闭环

### 3.5 当前命令面存在真实不一致

当前存在一条必须优先收敛的裂缝：

- parser：短命令 `o / ws / sv`
- completion：静态子命令表里已有 `open / save`
- init / dispatch：静态子命令表里也已有 `open / save`
- completion candidates：positionals / flags 规则按 `open / save` 写死

结论：

> **命令面收敛不是文档工作，而是 P0 工程任务。**

---

## 4. 设计原则

### 4.1 组件名保持 `bookmarks`

不为了“更像 zoxide”而重命名组件。

### 4.2 搜索层统一，动作层分离

未来的：

- `z`
- `zi`
- `o`
- future `open`
- `fuzzy`（过渡期）
- completion

都应共享同一个 query core，只在最终动作上分离。

### 4.3 保持短命令兼容

当前真实用户接口已经有：

- `o`
- `ws`
- `sv`

后续可以新增长命令，但**不应轻易破坏旧短命令兼容性**。

### 4.4 默认 `--auto`，显式 scope 为增强项

不应直接移除当前 `cwd_boost` 心智，而应让它升级为：

- 默认 `--auto`

再在此基础上增加：

- `--global`
- `--child`
- `--base`
- `--workspace`

### 4.5 Windows-first

Windows 仍然是 bookmarks 子系统的重要目标平台，因此：

- 大小写不敏感
- `\\` / `/` 统一
- UNC / `~` / 绝对路径 / 相对路径
- reparse point / symlink / 网络路径
- Explorer / `cmd /C start`

都必须进入正式设计，不可后补。

### 4.6 不扩张低价值命令面

不建议新增：

- 与 `check / gc` 同义的批量治理命令
- 与 `o` 同义但单独实现的打开命令
- 与 `fuzzy` 高度重叠的额外临时查询命令

优先级应转向：

- query core
- auto learn
- source / pin
- scope
- 体验闭环

### 4.7 保持可解释性

任何新排序能力都应尽量满足：

- 能 list
- 能 score
- 能 explain
- 能 preview

否则调参与回归会变得昂贵。

---

## 5. 硬约束

### 5.1 延迟预算

- `z` 单次跳转目标：`P99 < 50ms`
- `zi` 首次候选渲染目标：`< 200ms`
- 自动学习 hook：不能阻塞 prompt
- completion：Top-K 候选生成应保持近似即时

### 5.2 存储兼容与迁移边界

在没有明确 migration 方案之前：

- 现有 `~/.xun.json` 必须可继续读取
- 现有 Dashboard bookmarks API 不能被粗暴打断
- 现有显式书签数据不能因引入 learned source 而语义漂移

建议：

- 增加 `schema_version`
- 做增量迁移，不做硬切库

### 5.3 路径标准化策略

当前已有 `PathPolicy` 与 `normalize_path()` 基础，但后续必须升级为**全链路入库前标准化**：

- `explicit`
- `imported`
- `learned`

都走统一 normalization pipeline。

最少应统一：

- 分隔符
- 大小写
- 尾随分隔符
- 相对路径转绝对路径
- `~` 展开
- UNC / device namespace 的合法性处理

### 5.4 dirty save 策略

当前已有 append visit log 的思路，后续应在此基础上继续演进：

- 访问增量累计超过 `N` 次触发
- 或距离上次 compact/save 超过 `T` 秒触发
- learned source 单独允许更激进的 compact / aging

换句话说：

> **延迟落盘应扩展当前机制，而不是退回“每次访问立刻写主库”。**

### 5.5 命令面一致性约束

任何引入：

- `open`
- `save`
- `workspace`
- `zi`
- `--workspace`

的设计，都必须同步考虑：

- parser
- help
- completion
- shell init
- release note

否则会继续出现“文档说可以，解析器却不支持”的裂缝。

### 5.6 benchmark / 回归门槛

任何宣称“达到 zoxide 级体验”的迭代，至少要满足：

- 同机环境对比 zoxide
- 数据规模 `>5000`
- Windows + PowerShell
- 覆盖 `z` / `zi` / completion

### 5.7 自动学习必须可控

自动学习能力上线时，必须同时具备：

- exclusion
- source 区分
- 关闭开关
- 最小可用的查看/清理入口

不能先上“黑盒记录”，后补治理。

---

## 6. P0：地基能力

### 6.1 命令面一致性收敛

目标：

- 解决当前 `o/ws/sv` 与 `open/save/workspace` 在 parser / completion / init / dispatch / 文档上的不一致

建议范围：

- 第一阶段保持短命令兼容
- 统一决定是否新增长命令别名
- help、completion、init、dispatch、文档同时更新

验收标准：

- 用户看到的帮助、completion、文档与 parser 一致
- 不再出现“脚本里列出了 open，但 CLI 实际不识别”的情况

### 6.2 统一 query core

目标：

- 让 `z / o / future zi / future open / fuzzy / completion` 共用同一套查询内核

建议最小抽象：

```rust
struct BookmarkQuerySpec {
    keywords: Vec<String>,
    tag: Option<String>,
    scope: QueryScope,
    action: QueryAction,
    limit: Option<usize>,
    explain: bool,
    preview: bool,
}

enum QueryScope {
    Auto,
    Global,
    Child,
    BaseDir(PathBuf),
    Workspace(String),
}

enum QueryAction {
    JumpFirst,
    OpenFirst,
    Interactive,
    List,
    Complete,
}
```

验收标准：

- `z` 与 `o` 排序一致
- completion 不再单独维护独立排序规则
- `fuzzy` 先代理到 query core，再视版本策略决定是否移除

### 6.3 多 token + `name/path/tag` 混合匹配

目标：

- 从“只按书签名 fuzzy”升级到“名称 + 路径 + tag”的混合搜索

建议范围：

- 支持多 token AND 查询
- 让下面字段都参与匹配：
  - `name`
  - `path basename`
  - `path segment`
  - `tag`
- 第一版先做分层匹配：
  1. `name exact/prefix`
  2. `basename exact/prefix`
  3. `segment ordered match`
  4. `subsequence fuzzy` 兜底

验收标准：

- `z client api`
- `z repo docs`
- `z work rust`

都能稳定返回可解释结果。

### 6.4 `explicit / imported / learned / pinned` 数据模型

目标：

- 为 hybrid 排序与治理奠定统一数据语义

建议新增字段：

- `source: explicit | imported | learned`
- `pinned: bool`
- 预留 `desc: Option<String>`

排序 baseline 建议从当前公式平滑扩展为：

```text
FinalScore =
  MatchScore
  * (1 + Frecency * Wf)
  * ScopeBoost
  * SourceBoost
  * PinBoost
```

验收标准：

- learned 不压过 pinned explicit
- imported 在排序与治理上可单独区分
- Dashboard / list / zi 能显示来源和 pin 状态

### 6.5 自动学习目录访问 + 噪声排除

目标：

- 通过 shell hook 自动记录用户真正进入过的目录

建议范围：

- 先支持 PowerShell
- 后续扩展 bash / zsh / fish
- learned 数据单独入库
- 支持 `_ZO_EXCLUDE_DIRS` 风格 glob 排除配置
- 默认排除：
  - `node_modules`
  - `dist`
  - `build`
  - `target`
  - `tmp`
  - 系统临时目录
- 初次启用时允许从 shell history 预填充

验收标准：

- 进入目录后能自动记录 learned 数据
- learned 不覆盖 explicit
- 冷启动不会完全无结果
- 噪声目录不会快速污染结果

### 6.6 导入外部生态

目标：

- 降低迁移成本，解决冷启动问题

建议范围：

- `autojump` importer
- `z-compatible` importer：覆盖 `rupa/z`、`fasd`、`z.lua`、`zsh-z`
- `zoxide` importer
- 导入时统一做：
  - 路径标准化
  - dedup
  - rank / score 映射
  - source=`imported`

验收标准：

- 常见旧数据库可导入
- imported 数据能直接参与搜索
- imported 不会破坏 explicit 优先级

### 6.7 路径标准化全链路接入

目标：

- 把当前“局部校验 + 局部比较”升级为统一 normalization pipeline

建议范围：

- explicit / imported / learned 入库前统一处理
- dedup / check / gc / ranking / dashboard 统一使用标准化结果

验收标准：

- 同一目录不会因大小写、分隔符、尾随斜杠差异重复入库
- Windows 网络路径 / UNC 行为可预测

---

## 7. P1：体验闭环

### 7.1 `zi` 交互式跳转

目标：

- 把交互选择从 `z` 隐式行为中剥离出来

建议范围：

- `zi <keywords...>`
- 共用 query core
- 优先 `fzf`
- 缺失时降级到内置选择器
- 交互选择后继续输出 `__CD__`

验收标准：

- `z` 保持极速 Top-1
- `zi` 明确进入交互
- 排序与 `z --list` 一致

### 7.2 显式范围搜索

建议引入：

- `--auto`
- `--global`
- `--child`
- `--base <dir>`
- `--workspace <name|path>`
- `--exclude-cwd`

验收标准：

- `--auto` 保持当前 cwd 语义兼容
- `--global` 关闭 cwd 偏置
- `--child` 明显偏向当前目录子树
- `--base` 能严格限定搜索边界
- `--workspace` 能稳定映射 workspace 根

### 7.3 `--list / --score / --why`

目标：

- 提供可解释、可调试的排序输出

建议范围：

- `z --list`
- `z --list --score`
- `z --why foo`
- 输出至少包含：
  - MatchScore
  - FrecencyScore
  - ScopeScore
  - SourceBoost
  - PinBoost
  - TagBonus

验收标准：

- 可以看见统一分值
- Top-1 推荐原因可以解释
- 开发者可用其调参

### 7.4 歧义提示

目标：

- 在不打断极速体验的前提下降低误跳

建议范围：

- Top-1 与 Top-2 分差低于阈值时
- `z foo` 仍默认跳 Top-1
- 同时提示：`可使用 zi foo 查看相近候选`

### 7.5 completion 与 query core 对齐

目标：

- 让补全与执行一致

验收标准：

- tab completion 排序与 `z --list` 主序一致
- completion 也能吃到 scope / source / pin / frecency 结果

### 7.6 主动 dead-link 提示 + 批量治理

目标：

- 形成查询时治理 + 批量治理闭环

建议范围：

- `z / o / zi` 命中 dead link 时即时提示
- 保留并增强：
  - `check`
  - `gc`
- Dashboard 增加 dead / stale / duplicate 视图

验收标准：

- 用户命中死链时不再只看到模糊失败
- 批量扫描结果可被 CLI 与 Dashboard 共用

### 7.7 Shell init 模板化

目标：

- 把自动学习、completion、命令别名、`zi` 集成到统一 init 体验

建议范围：

- 先增强 PowerShell
- 再补 bash / zsh / fish
- 对齐 zoxide 的 `init <shell>` 心智

验收标准：

- Windows / PowerShell 成为优先打磨样本
- hook、completion、命令别名来自同一套模板化配置

### 7.8 `--preview / --dry-run`

目标：

- 给 query core 改造提供低风险演练模式

建议范围：

- `z --preview foo`
- `o --preview foo`
- 只打印候选与原因，不执行跳转 / 打开

验收标准：

- 用户可以先看结果再切换新行为
- 排序调试无需借助临时脚本

---

## 8. P2：长期增强

### 8.1 备注 / 描述 / 视图增强

目标：

- 为 list / Dashboard / zi 提供更高可读性

建议范围：

- `desc`
- `recent` 支持 tag / workspace / 时间窗口
- source / pin / desc 联合展示

### 8.2 SQLite / 索引化存储

目标：

- 在数据量上升后保证性能与一致性

建议范围：

- 先设阈值：`>5000` 条记录或 completion/zi 出现明显延迟
- 评估 SQLite
- 建立索引：
  - `name`
  - `path basename`
  - `path segment`
  - `tag`
- 对齐 `_ZO_MAXAGE` 风格 aging 策略

### 8.3 Top-K 精排与倒排索引

目标：

- 从“优化字符打分”转向“先缩候选集，再精排”

建议范围：

- segment 倒排索引
- tag 倒排索引
- top-k only 精排
- completion / zi / list 共享候选集策略

### 8.4 undo / 变更历史

目标：

- 为 `set / rename / delete / import / pin` 提供撤销能力

### 8.5 schema migration / data versioning

目标：

- 为 source、pin、desc、future SQLite 演进保留稳定迁移能力

### 8.6 Benchmark 与回归验证

目标：

- 在真实数据规模下验证体验目标

建议范围：

- 与 zoxide 在同机环境对比
- 重点测：
  - Windows + PowerShell
  - `>5000` 条记录
  - `z`
  - `zi`
  - completion
- 输出回归基准脚本

---

## 9. 建议实施顺序

### Phase 1：地基收口

- 命令面一致性收敛
- 统一 query core
- 多 token + `name/path/tag` 混合匹配
- source / pin 数据模型
- 路径标准化全链路接入

### Phase 2：让 hybrid 真正成立

- 自动学习目录访问
- 排除目录配置
- 导入外部生态
- PowerShell hook 模板化

### Phase 3：体验闭环

- `zi`
- 范围搜索
- `--list / --score / --why`
- completion 对齐
- 歧义提示
- `--preview`

### Phase 4：治理闭环

- 主动 dead-link 提示
- Dashboard dead / stale / duplicate 视图
- `check / gc` 强化
- `desc / recent` 增强

### Phase 5：长期性能与迁移

- SQLite / 倒排索引
- benchmark 脚本
- undo / schema migration

---

## 10. 当前最值得立即启动的一轮

如果只做一轮、追求最大边际收益，建议直接启动：

1. 命令面一致性收敛
2. 统一 query core
3. 多 token + `name/path/tag` 混合匹配
4. `explicit / imported / learned / pinned` 模型
5. 路径标准化全链路接入
6. PowerShell 自动学习 hook + 排除目录
7. 导入外部生态

原因：

- 这几项会决定：
  - `zi`
  - 范围搜索
  - 推荐解释
  - completion 一致性
  - hybrid 排序
  后续能不能做稳
- 它们也是从“显式管理工具”跨到“智能导航器”的最短路径

---

## 11. 一句话版本

Bookmarks 组件后续路线图可以概括为：

> **先统一命令面与搜索内核，再补 source / pin / 自动学习 / 导入与全链路路径标准化，随后完成 `zi` / 范围搜索 / 推荐解释 / preview / dead-link 治理闭环，最后再升级 SQLite 与索引化性能基建。**
