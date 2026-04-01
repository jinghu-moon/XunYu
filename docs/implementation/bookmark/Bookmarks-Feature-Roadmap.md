# Bookmark 功能路线图（2026-03 最终版）

> **版本**：2.0 · **更新时间**：2026-03-31  
> 关联文档：Bookmarks-Competitor-Review.md · bookmark-PRD.md · bookmark-Config-Spec.md · bookmark-Benchmark-Suite.md · bookmark-SQLite-Evaluation.md

---

## 1. 目标

这份路线图用于把 bookmark 组件后续值得做的能力收敛成**可执行、可迁移、可验证**的优先级计划。

总方向保持不变：

> **把当前"显式书签管理器"，升级为"显式书签系统 + 智能目录导航层"。**

本路线图建立在三条硬要求之上：

1. 建立在**当前代码事实**之上，而不是理想命令面之上
2. 建立在**2026 年竞品最新状态**之上（autojump 维护明显减弱；zoxide 0.9.x 仍是主要对标对象）
3. 每一阶段都必须同时考虑：用户心智 / 工程一致性 / Windows + PowerShell 主场景

### 1.1 实现状态同步（2026-03-30）

当前代码已经完成以下阶段：

- 已完成：命令面收口到 `xun bookmark <sub>` / `bm <sub>`
- 已完成：新主存储 `~/.xun.bookmark.json` + `schema_version = 1`
- 已完成：统一 query core、`z / zi / o / oi`、`--list / --score / --why / --preview`
- 已完成：`explicit / imported / learned / pinned` 数据模型
- 已完成：自动学习、排除目录、外部导入、`bm init`
- 已完成：completion 与 query core 对齐
- 已完成：bookmark 存储与 completion 逻辑收口到 `src/bookmark/`
- 已完成：`fuzzy` 公共命令删除，不再保留迁移壳
- 暂未完成：Dashboard 书签面板同步、SQLite
- 已实现：持久化倒排索引（JSON sidecar，大库候选召回优化）

---

## 2. 本轮核心调整

### 2.1 命名规范最终确认

- **组件名**：`bookmark`（**单数**，严格统一，禁止使用复数形式）
- **快捷别名**：`bm`（官方短别名，等价于 `xun bookmark`）
- **四个核心导航命令**：`z / zi / o / oi`

### 2.2 四命令对称导航矩阵（最终方案）

| 命令 | 全称语义 | 动作 | 模式 |
|---|---|---|---|
| `z` | jump | `cd` 跳转 | 极速，取 Top-1 |
| `zi` | jump interactive | `cd` 跳转 | 交互，fzf 选择 |
| `o` | open | 文件管理器打开 | 极速，取 Top-1 |
| `oi` | open interactive | 文件管理器打开 | 交互，fzf 选择 |

**设计逻辑**：`i` 后缀统一表示「交互选择模式」，形成完整 2×2 对称矩阵。`z/zi` 与 zoxide 完全对齐；`o/oi` 是 xun 独有差异化能力。

### 2.3 `bookmark` 命名空间内保留完整子命令集

- `xun bookmark` 是唯一正式入口，`bm` 是官方短别名
- `z / zi / o / oi` 保留为导航子命令
- `open / save` 也保留为 `bookmark` 子命令
- 独立 `workspace` 动作子命令移除，统一改用 `--workspace` 查询范围
- 旧顶层 `xun z / o / ws / sv / fuzzy / ...` 不再作为 vNext 公共 CLI

### 2.4 `pin` 提前到 P0

hybrid 模型一旦引入 `learned` source，没有 pin 就没有用户可控性。显式书签本身就是用户主动表达偏好的结果。这不是锦上添花，而是排序可信度的地基。

### 2.5 排分公式统一为乘法形式

**最终确认**：所有排分因子均采用无量纲乘法系数，禁止混用加法绝对分值。

```text
FinalScore =
  MatchScore
  × FrecencyMult          // 1.0 ~ 1.25
  × ScopeMult             // 1.0 ~ 2.5（--global 时固定为 1.0）
  × SourceMult            // explicit=1.20 / imported=1.05 / learned=1.00
  × PinMult               // pinned=1.50 / normal=1.00
```

### 2.6 Phase 1 拆分已经落地

原 Phase 1 被拆成 `1a / 1b` 的工程策略已经执行完成：

- **Phase 1a**：命令面、help、completion、init、dispatch 已收口
- **Phase 1b**：query core、数据模型、多 token 匹配已落地

### 2.7 `fuzzy` 已退出正式方案

- `bm fuzzy` 已从公共 CLI 删除
- 正式查询入口统一为 `bm z --list`
- 后续文档不再讨论 deprecation 过渡期

### 2.8 `--preview` 保持命名，不改为 `--dry-run`

`bm z --preview foo` = 打印候选与原因，不执行跳转。在 xun 的命令体系中无歧义（`--preview` 的 fzf 语义由 `_BM_FZF_OPTS` 覆盖，不冲突）。

---

## 3. 当前代码基线（路线图前置约束）

### 3.1 当前真实命令面

当前 parser 已正式支持：

- 顶层正式入口：`xun bookmark <sub>`
- 顶层短别名：`bm <sub>`（由 `init` / `bookmark init` 生成 shell wrapper）
- bookmark 子命令：`z / zi / o / oi / open / save / set / delete / tag / pin / unpin / rename / list / recent / stats / check / gc / dedup / export / import / init / touch / learn / keys / all`

这意味着：

- 旧顶层 `xun z / o / ws / sv / fuzzy / ...` 已不再是公共 CLI
- `workspace` 只保留为 `--workspace` 查询范围，不再是动作子命令

### 3.2 当前搜索实现

- `z / zi / o / oi / completion` 已共用统一 query core
- `zi / oi` 在可交互终端中使用 `dialoguer::FuzzySelect`，非交互模式回落 Top-1
- query 支持多 token、tag、scope、`--score`、`--why`、`--preview`

当前主排序公式已经是：

```text
FinalScore = MatchScore × FrecencyMult × ScopeMult × SourceMult × PinMult
```

### 3.3 当前数据层

- 主库存储：`~/.xun.bookmark.json`
- 访问日志：`~/.xun.bookmark.visits.jsonl`
- schema：根对象 `schema_version = 1`
- source：`explicit / imported / learned`
- 保存：主库 temp file + rename

结论：**当前 JSON store 已能支撑 v1；SQLite 只作为大规模数据量下的后续优化方向。**

### 3.4 当前治理能力

已具备：`check`（missing / stale / duplicate）、`gc`（dead link 清理）、`o`（命中 dead link 时给出修复提示）、Dashboard bookmarks CRUD / import / export / batch。后续路线图应把这些能力纳入 hybrid 系统的正式治理闭环。

### 3.5 当前剩余裂缝

当前剩余工作主要集中在：

- Dashboard 仍有部分旧 `store::Db` 消费路径
- 文档与自动生成说明仍有历史命令/路径残留
- benchmark 仍是本地基线，尚未形成 CI 化回归

---

## 4. 设计原则

### 4.1 组件名保持 `bookmark`（单数）

所有内部模块、文档、CLI 子命令均使用单数形式，严禁使用复数 `bookmarks`。

### 4.2 搜索层统一，动作层分离

`z / zi / o / oi / completion` 全部共享同一个 `bookmark::query` 内核，只在最终动作上分离：

```
bookmark::query(spec)
       │
       ├─ JumpFirst      →  z   (cd 到 Top-1)
       ├─ Interactive    →  zi  (fzf 选择后 cd)
       ├─ OpenFirst      →  o   (文件管理器打开 Top-1)
       └─ OpenInteractive→  oi  (fzf 选择后文件管理器打开)
```

### 4.3 命令空间 clean break

vNext 不保留旧顶层 `xun z / o / ws / sv / fuzzy / ...` 公共接口。唯一正式入口是 `xun bookmark <sub>`，`bm <sub>` 为官方短别名。`z / zi / o / oi` 仅存在于 `bookmark` 命名空间与 `bm init` 生成的 shell wrapper 中。

### 4.4 默认 `--auto`，显式 scope 为增强项

不应直接移除当前 `cwd_boost` 心智，而应让它升级为默认的 `--auto`，再在此基础上增加：`--global / --child / --base / --workspace`。

### 4.5 Windows-first

Windows + PowerShell 是首要打磨平台：

- 大小写不敏感
- `\\` / `/` 统一（存储用 `/`，显示按平台还原）
- UNC / `~` / 绝对路径 / 相对路径
- reparse point / symlink / 网络路径
- Explorer / `cmd /C start`

均进入核心设计，不可后补。

### 4.6 不扩张低价值命令面

**不建议新增**：与 `check / gc` 同义的批量治理命令、与 `o` 同义但单独实现的打开命令。优先级应转向：query core / auto learn / source+pin / scope / 体验闭环。

### 4.7 保持可解释性

任何新排序能力都应满足：能 list / 能 score / 能 explain / 能 preview。否则调参与回归会变得昂贵。

### 4.8 不覆盖 `cd`

在 init 模板里提供注释形式的可选 alias，让用户自己决定是否覆盖 `cd`。不强制，不默认。

---

## 5. 硬约束

### 5.1 延迟预算

| 操作 | 目标 |
|---|---|
| `bm z` 单次跳转 | P99 < 50ms（Windows + PowerShell，5000 条） |
| `bm zi / bm oi` 首次候选渲染 | P99 < 200ms |
| 自动学习 hook 写入 | 不阻塞 prompt（异步后台） |
| completion Top-K 生成 | < 80ms（Tab 到候选出现） |

### 5.2 存储切换边界

- bookmark vNext 允许直接切换到新主存储 schema
- 旧 `~/.xun.json` 结构不作为本轮兼容目标；正式主存储已切换为 `~/.xun.bookmark.json`
- Dashboard / panel API 暂不纳入本轮约束
- `schema_version` 仍保留，用于 vNext 之后的格式演进

### 5.3 路径标准化策略

`explicit / imported / learned` 入库前统一经过 normalization pipeline：展开 `~` → 相对路径转绝对 → 分隔符统一 → 大小写（Windows 小写为比较键，保留原始大小写显示）→ 移除尾随分隔符 → UNC 合法性验证。

### 5.4 dirty save 策略

```
访问增量累计 > 50 次  →  触发 compact & save
或距上次 save > 600s  →  触发 compact & save
learned 条目允许更激进：增量 > 100 次或 > 300s
```

延迟落盘应扩展当前机制，而不是退回"每次访问立刻写主库"。

### 5.5 命令面一致性约束

引入 `zi / oi / open / save / --workspace` 的任何设计，必须同步考虑：parser / help / completion / shell init / release note。

### 5.6 benchmark 回归门槛

任何宣称"达到 zoxide 级体验"的迭代，至少满足：

- 同机环境对比 zoxide v0.9+
- 数据规模 > 5000 条
- Windows + PowerShell 7.x
- 覆盖 `z / zi / oi / completion`

### 5.7 自动学习必须可控

自动学习能力上线时，必须同时具备：exclusion 配置、source 区分、关闭开关、最小可用查看/清理入口。不能先上"黑盒记录"，后补治理。

### 5.8 fzf 集成规范

| 条件 | 行为 |
|---|---|
| fzf ≥ v0.51.0 存在 | 调用 fzf（支持 preview 窗口） |
| fzf 缺失，终端支持 | 降级到 `dialoguer::FuzzySelect`（Windows Terminal ≥ 1.18） |
| fzf 缺失，终端不支持 | 退化为编号选择（打印候选列表 + 输入数字） |

适用于 `zi` 和 `oi` 两个命令。

---

## 6. P0：地基能力

### 6.1 命令面一致性收敛（Phase 1a）

**目标**：完成从旧顶层命令到 `xun bookmark <sub>` 命名空间的正式切换。

**范围**：

- parser 正式暴露 `bookmark` 组件入口
- `z / zi / o / oi / open / save / set / ...` 全部下沉到 `bookmark` 命名空间
- 移除旧顶层 `z / o / ws / sv / fuzzy / ...` 公共接口
- parser / help / completion / init / dispatch / 测试 / 文档同步更新

**验收**：用户看到的 help、completion、文档与 parser 行为严格一致；`xun bookmark ...` 与 `bm ...` 为唯一正式入口。

---

### 6.2 路径标准化全链路接入（Phase 1a）

**目标**：把当前"局部校验 + 局部比较"升级为统一 normalization pipeline。

**范围**：

- `explicit / imported / learned` 入库前统一处理
- dedup / check / gc / ranking / dashboard 统一使用标准化结果
- Windows：大小写统一小写比较键，保留原始大小写用于显示
- UNC / network 路径合法性验证

**验收**：同一目录不会因大小写、分隔符、尾随斜杠差异重复入库；Windows 网络路径行为可预测。

---

### 6.3 统一 query core（Phase 1b）

**目标**：让 `z / o / zi / oi / completion` 共用同一套查询内核。

**最小接口设计**：

```rust
pub struct BookmarkQuerySpec {
    pub keywords:   Vec<String>,
    pub tag:        Option<String>,
    pub scope:      QueryScope,
    pub action:     QueryAction,
    pub limit:      Option<usize>,
    pub explain:    bool,   // --why / --score
    pub preview:    bool,   // --preview
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

**验收**：`z` 与 `o` 排序结果完全一致；completion 不再单独维护独立排序规则。

---

### 6.4 `explicit / imported / learned / pinned` 数据模型（Phase 1b）

**目标**：为 hybrid 排序与治理奠定统一数据语义。

**新增字段**：

```jsonc
{
  "schema_version": 1,
  "name_norm": "my-project", // explicit 名称唯一键（全局、大小写不敏感）
  "source": "explicit",   // explicit | imported | learned
  "pinned": false,
  "tags": ["work", "rust"],
  "desc": "",             // 预留，P2 填充
  "workspace": null       // 可选，关联 workspace 名称
}
```

补充约束：

- `explicit` 条目的 `name` 全局唯一，按 `name_norm` 做大小写不敏感比较
- `imported / learned` 条目允许 `name = null`，不参与 name 唯一性约束
- `tag / pin / rename / touch` 等 `<name>` 型管理命令只作用于具名 `explicit` 条目

**排序公式（乘法形式，各因子均为无量纲系数）**：

```text
FinalScore =
  MatchScore
  × FrecencyMult          // 1.0 ~ 1.25，基于 zoxide 时间桶参数
  × ScopeMult             // 1.0 ~ 2.5（--global 时固定为 1.0）
  × SourceMult            // explicit=1.20 / imported=1.05 / learned=1.00
  × PinMult               // pinned=1.50 / normal=1.00
```

> **禁止混用加法绝对分值（如 +30）与乘法系数**。`ScopeMult` 是无量纲系数，取值范围 1.0 ~ 2.5。

**验收**：`learned` 不压过 `pinned explicit`；`imported` 在排序与 `--list` 中可单独可见；Dashboard / list / zi / oi 能显示来源和 pin 状态。

---

### 6.5 多 token + name/path/tag 混合匹配（Phase 1b）

**目标**：从"只按书签名 fuzzy"升级到"名称 + 路径 + tag"的混合搜索。

**分层匹配规则**：

| 层级 | 类型 | 分值 |
|---|---|---|
| 强匹配 | name exact | 100 |
| | name prefix | 80 |
| | path basename exact | 70 |
| | path basename prefix | 60 |
| 结构匹配 | path segment ordered match | 40 ~ 55 |
| | multi-token cross-field AND | 35 ~ 50 |
| | tag hit | +10 ~ +15（加分，不过滤） |
| 弱 fuzzy | subsequence fuzzy（兜底） | 10 ~ 35 |

**验收场景**：

```bash
bm z client api     # 稳定返回同时含 "client" 和 "api" 的候选
bm z repo docs      # 命中 /repos/xxx/docs 类路径
bm z work rust      # 命中 tag=work 且 path 含 "rust" 的书签
```

---

### 6.6 自动学习目录访问 + 噪声排除（Phase 2）

**目标**：通过 shell hook 自动记录用户真正进入过的目录。

**范围**：

- 先支持 PowerShell，再扩展 bash / zsh / fish
- `learned` 数据单独入库，与 `explicit` 并存不覆盖
- 支持 `_BM_EXCLUDE_DIRS` glob 排除配置
- 默认排除：`node_modules / dist / build / target / .git / tmp / temp` 及系统临时目录

**冷启动预填充**：

| shell | 历史文件 | 提取规则 |
|---|---|---|
| PowerShell | `$env:APPDATA\...\ConsoleHost_history.txt` | 提取 `cd <path>` / `z <path>` 类指令 |
| Bash | `~/.bash_history` | 提取 `cd / z / j` 类指令 |
| Zsh | `~/.zsh_history` | 同上，兼容 EXTENDED_HISTORY 格式 |

预填充条目 frecency 初始值 = 「真实访问条目平均值的 30%」（低于真实数据，避免污染排序）。一次性操作，可通过 `bm import --from-history` 手动重新触发。

**上线时必须同时具备的控制能力**：

- `_BM_EXCLUDE_DIRS` 环境变量
- `bm gc --learned` 清理 learned 记录
- `bm learn --off` 全局关闭自动学习
- `bm list --source learned` 查看 learned 记录

**验收**：进入目录后自动记录 learned；`learned` 不覆盖 `explicit`；冷启动不会完全无结果；噪声目录不快速污染结果。

---

### 6.7 导入外部生态（Phase 2）

**目标**：降低迁移成本，解决冷启动问题。

**支持来源**：

| 来源 | 命令 | 特殊处理 |
|---|---|---|
| autojump | `bm import --from autojump` | 直接读取文本数据库 |
| zoxide | `bm import --from zoxide` | 调用 `zoxide query --list --score` 导出后解析 |
| rupa/z + z.lua + zsh-z | `bm import --from z` | `rank\|time\|path` 格式，z.lua 字段顺序略有差异 |
| fasd | `bm import --from fasd` | **只导入目录类型（d）条目**，文件类型丢弃 |

**导入流程（统一）**：路径标准化 → score 归一化映射至 `[1, 100]` → dedup → source 标记为 `imported` → 写入 `learned` 库（不覆盖同路径 `explicit` 书签）。

导入语义补充：

- `zoxide query --list --score` 只能可靠提供 `path + score`
- 对 `zoxide` 导入条目：
  - `frecency_score` 直接使用归一化后的导入值
  - `visit_count` / `last_visited` 允许为空，直到第一次本地访问后再开始累积
- 查询阶段若本地访问历史缺失，则 `FrecencyMult` 由持久化 `frecency_score` seed 直接归一化得到

**验收**：常见旧数据库可导入；`imported` 数据能直接参与搜索；`imported` 不破坏 `explicit` 优先级。

---

## 7. P1：体验闭环

### 7.1 `zi` 交互式跳转

**目标**：把交互选择从 `z` 隐式行为中剥离出来。

- `bm zi [keywords...]`，共用 query core，Top-K 候选（默认 20）
- 后端优先级：fzf ≥ v0.51.0 → `dialoguer::FuzzySelect` → 编号选择
- fzf preview 窗口：显示目录内容（`ls / dir`）
- 选中后输出 `__BM_CD__ <path>` 供 shell wrapper 执行 `cd`

**验收**：`z` 保持极速 Top-1；`zi` 明确进入交互；排序与 `z --list` 一致。

---

### 7.2 `oi` 交互式打开

**目标**：为"打开文件管理器"场景提供完整的交互选择能力。

- `bm oi [keywords...]`，共用 query core
- 后端优先级与 `zi` 相同
- 选中后调用平台文件管理器打开（Explorer / open / xdg-open）

**验收**：`o / oi` 排序与 `z / zi` 完全一致；`oi` 在 Windows Terminal 上流畅运行。

---

### 7.3 显式范围搜索

引入：

| Flag | 语义 |
|---|---|
| `--auto`（默认） | 延续当前 cwd 心智，但作为新 query core 的正式 scope |
| `--global` | 关闭 cwd 偏置，`ScopeMult` 固定为 1.0 |
| `--child` | 当前目录子树优先，非子树结果 `ScopeMult` 降为 0.5 |
| `--base <dir>` | 只在指定父路径下搜索（严格过滤） |
| `--workspace <name\|path>` | workspace 范围搜索 |
| `--exclude-cwd` | 排除当前目录本身 |

**验收**：`--auto` 延续当前 cwd 心智；`--global` 关闭 cwd 偏置；`--child` 明显偏向子树；`--base` 严格限定边界。

---

### 7.4 `--list / --score / --why`

**目标**：提供可解释、可调试的排序输出。

`bm z --list --score foo` 输出格式：

```
 # │ Score  │ Match  │ FrecencyMult │ ScopeMult │ SourceMult │ PinMult │ Path
───┼────────┼────────┼──────────────┼───────────┼────────────┼─────────┼──────────────
 1 │  87.3  │  72.0  │     1.21     │    1.50   │    1.20    │   1.00  │ C:\dev\my-client
 2 │  61.4  │  60.0  │     1.10     │    1.00   │    1.05    │   1.00  │ D:\work\client-api
```

`bm z --why foo` 输出格式：

```
→ 跳转至：C:\dev\my-client
原因：
  MatchScore    72.0  (name prefix 命中 → 80; basename prefix 命中 → 80; 取最高 72 经归一化)
  FrecencyMult  1.21  (42 次访问, 最近 2 小时内, 衰减系数 4.0)
  ScopeMult     1.50  (当前目录是书签父路径，child boost)
  SourceMult    1.20  (explicit 书签)
  PinMult       1.00  (未置顶)
  FinalScore    87.3
```

**验收**：Top-1 推荐原因可解释；开发者可用其调参；各维度分值与实际排序严格一致。

---

### 7.5 歧义提示

**目标**：在不打断极速体验的前提下降低误跳。

当 Top-1 与 Top-2 的 `FinalScore` 差距 < 15% 时，在执行跳转后追加提示（不阻断跳转）：

```
→ C:\dev\my-client
  提示：还有 2 个相近候选，使用 'bm zi foo' 查看
```

---

### 7.6 completion 与 query core 对齐

**目标**：让 tab completion 与 `bm z --list` 排序完全一致。

- completion 调用 `bookmark::query(spec)` 生成候选
- PowerShell：使用 `Register-ArgumentCompleter`
- 候选展示：`<name>  <path>（frecency: xx）`

**验收**：tab completion 排序与 `bm z --list` 主序一致；completion 吃到 scope / source / pin / frecency 权重。

---

### 7.7 主动 dead-link 提示

**目标**：形成查询时治理 + 批量治理闭环。

- `z / zi / o / oi` 命中死链时立即提示并中止
- **网络路径（UNC）检测加 300ms 超时保护**，超时则跳过检测，不阻塞主路径
- 保留并增强 `check / gc`；Dashboard 增加 dead / stale / duplicate 视图

提示格式：

```
[xun] 错误：书签 'my-project' 指向的路径已不存在
  路径：C:\dev\projects\my-project
  建议：运行 'bm gc' 清理死链，或 'bm set my-project' 更新路径
```

---

### 7.8 shell init 模板化

**目标**：把自动学习、completion、命令别名、`zi / oi` 集成到统一 init 体验。

- `bm init powershell / bash / zsh / fish`
- 支持 `--cmd <prefix>` 参数（如 `bm init --cmd j` 输出 `j / ji / jo / joi` alias）
- 输出内容包含：`z / zi / o / oi` wrapper 函数 + 自动学习 hook + completion + `bm` alias
- 在 init 模板中提供注释形式可选 alias：`# alias cd='bm z'`
- `bm` 根命令 completion 只补全 bookmark 子命令；`z / zi / o / oi` completion 才补全候选路径
- PowerShell 自动学习 hook 使用异步子进程，不使用会话内 `Start-Job`

**关键时机约定**：Phase 2 完成 PowerShell hook 模板化时，wrapper 必须同时为 `zi / oi` 预留 `__BM_CD__` 接口。即使 `zi / oi` 本身是 P1 功能，wrapper 也必须在 Phase 2 就位，避免 Phase 3 上线时回头修改 init 模板引入回归。

PowerShell init 核心片段：

```powershell
$__bm_exe = if ($env:XUN_EXE) { $env:XUN_EXE } else { "xun.exe" }
function z {
    $result = & $__bm_exe bookmark z @args
    if ($result -match '^__BM_CD__ (.+)$') { Set-Location $Matches[1] }
    elseif ($result) { Write-Output $result }
}
function zi {
    $result = & $__bm_exe bookmark zi @args
    if ($result -match '^__BM_CD__ (.+)$') { Set-Location $Matches[1] }
}
function o  { & $__bm_exe bookmark o  @args }
function oi { & $__bm_exe bookmark oi @args }
function bm { & $__bm_exe bookmark @args }
```

**验收**：Windows / PowerShell 成为优先打磨样本；hook / completion / 命令别名来自同一套模板化配置。

---

### 7.9 `--preview`（dry-run 模式）

**目标**：给 query core 改造提供低风险演练模式。

- `bm z --preview foo`：打印候选与各维度分值，不执行跳转/打开
- 用于调试新排序行为，无需借助临时脚本

---

## 8. P2：长期增强

### 8.1 备注 / 描述 / 视图增强

- `desc` 字段：`bm set --desc "主项目"`
- `desc` 在 `--list / zi / oi / dashboard` 中展示
- `recent` 支持 `--tag / --workspace / --since 7d` 过滤

### 8.2 SQLite / 索引化存储

- 触发条件：> 5000 条记录或 completion / zi 出现明显延迟
- 建立四路索引：`name / path basename / path segment / tag`
- 需完整 schema migration 方案，对齐 `_BM_MAXAGE` 风格 aging 策略

### 8.3 Top-K 精排与倒排索引

- 已实现：持久化倒排索引（JSON sidecar，大库启用），先缩候选集再精排
- 已实现：completion / zi / oi / list 共享候选集策略
- 未实现：持久化 segment / tag 索引（后续仅在 SQLite 路线中考虑）

### 8.4 undo / 变更历史

- 已实现：`bookmark undo --steps <n>` 与 `bookmark redo --steps <n>`
- 当前覆盖 `set / save / rename / delete / import / pin / unpin / tag / gc / dedup`

### 8.5 schema migration / data versioning

- vNext 当前版本直接采用新主存储 schema（`schema_version = 1`）
- 自 vNext 之后的后续版本再由 `schema_version` 驱动增量迁移

### 8.6 Benchmark 与回归验证

- 同机环境对比 zoxide v0.9+
- 数据规模 > 5000 条
- Windows 11 + PowerShell 7.x
- 覆盖：`z / zi / oi / completion`
- 输出可 CI 集成的回归基准脚本

### 8.7 历史命令清理

- `bm fuzzy` 已从正式方案删除
- 后续只保留文档级 release note 说明，不再新增兼容壳

### 8.8 `--cmd <prefix>` 参数

- `bm init --cmd j` 输出 `j / ji / jo / joi` alias（对 autojump 迁移用户友好）
- 仅影响 init 模板输出，不影响 `bm` 主命令本身

---

## 9. 建议实施顺序

### Phase 1a — 工程收口（无新用户功能）

**目标**：消灭裂缝，建立可信代码基线。

- [x] 命令面一致性收敛（parser / help / completion / init / dispatch 五处同步）
- [x] 路径标准化全链路接入
- [x] `schema_version` 字段引入

**验收**：help 与 parser 行为严格一致；同路径书签不重复入库。

---

### Phase 1b — 功能地基

**目标**：在干净基线上构建核心能力。

- [x] 统一 query core（`BookmarkQuerySpec` + `bookmark::query`）
- [x] `explicit / imported / learned / pinned` 数据模型 + 排分公式升级
- [x] 多 token + name/path/tag 混合匹配
- [x] `bm fuzzy` 删除，正式入口统一为 `bm z --list`

**验收**：`bm z client api` 稳定返回可解释结果；`bm z` 与 `bm o` 排序完全一致。

---

### Phase 2 — Hybrid 成立

**目标**：让自动学习能力真正可用。

- [x] 自动学习目录访问（PowerShell hook 优先）
- [x] 排除目录配置（`_BM_EXCLUDE_DIRS`）
- [x] shell history 预填充冷启动
- [x] 导入外部生态（autojump / zoxide / z / fasd）
- [x] PowerShell init 模板化（**含 `zi / oi` wrapper 预留接口**）

**验收**：进入目录后自动记录 learned；`bm import --from zoxide` 成功导入；PowerShell hook 不阻塞 prompt。

---

### Phase 3 — 体验闭环

**目标**：完成面向最终用户的完整交互体验。

- [x] `zi` 交互式跳转（当前为 `dialoguer::FuzzySelect` + 非交互回落）
- [x] `oi` 交互式打开
- [x] 显式范围搜索（`--child / --base / --workspace / --global`）
- [x] `--list / --score / --why`
- [x] 歧义提示
- [x] completion 与 query core 对齐
- [x] `--preview`
- [x] help 中移除 `fuzzy` 条目

**验收**：`zi / oi` 在 Windows Terminal 上流畅运行；`--why` 输出各维度得分；tab completion 与 `--list` 排序一致。

---

### Phase 4 — 治理闭环

**目标**：完善数据健康维护体验。

- [x] dead-link 即时提示（本地路径与 UNC/网络路径超时保护已接入）
- [ ] Dashboard dead / stale / duplicate 视图
- [x] `check / gc` 强化
- [x] `desc` 字段 + `recent` 过滤增强
- [x] bash / zsh / fish init 模板

---

### Phase 5 — 长期性能与迁移

**目标**：为大规模使用打好技术底座。

- [x] SQLite 评估
- [ ] SQLite 迁移
- [x] 倒排索引（持久化倒排索引 JSON sidecar 已落地；SQLite 索引化后端未做）
- [x] benchmark 套件（本地可执行，待 CI 化）
- [x] `undo / redo` 机制（已切换为 delta-based history，当前覆盖 `set / save / rename / delete / import / pin / unpin / tag / gc / dedup`）
- [x] schema migration 框架与 legacy map 迁移
- [x] `bm fuzzy` 彻底移除
- [x] `--cmd <prefix>` 参数

---

## 10. 一句话版本

> **先统一命令面（Phase 1a）与搜索内核（Phase 1b），再补 source/pin/自动学习/导入与路径标准化（Phase 2），随后完成 zi/oi/范围搜索/推荐解释/preview/dead-link 治理闭环（Phase 3-4），最后升级 SQLite 与索引化性能基建并彻底清理 fuzzy 历史包袱（Phase 5）。**
