# XunYu 智能补全方案（Draft）

目标：在不破坏现有 `xun init` 工作流的前提下，引入**标准化动态补全入口**，并支持基于**前缀过滤 + frecency** 的排序与可选裁剪（默认不限制，仅在候选巨大场景按需裁剪），让补全“更懂用户”且保持低延迟。

补充（2026-02）：Dashboard Web UI 迭代不影响补全协议与 CLI 行为；相关 UI 变化详见 `Dashboard-Design.md`。

---

## 1. 现状与问题

**现状**
- `xun init <shell>` 输出的脚本内置补全（PowerShell/Bash/Zsh）。
- 已有部分动态补全：`redirect --profile`、`--undo/--tx`、书签名等。

**限制**
- 补全逻辑分散在脚本里，跨 shell 维护成本高。
- 缺少统一的动态补全入口，难以实现复杂排序、上下文联想与“Top N”裁剪。
- 无频率/历史加权，也无法利用当前路径的上下文提示。

---

## 2. 方案总览

### 2.1 核心思路
1. **新增统一动态补全入口**：`xun __complete`
2. **新增标准补全生成**：`xun completion <shell>`
3. **保留并增强现有 `xun init`**
   - `init` 继续提供 wrapper/alias/魔法输出处理
   - 同时调用 `completion` 装载统一补全逻辑

### 2.2 目标
- **统一**：所有 shell 都通过 `xun __complete` 获取候选。
- **智能**：前缀过滤 + 复用已有 frecency 排序。
- **可控**：默认不限制候选；仅在候选巨量时按需裁剪。
- **低延迟**：补全必须 < 50ms；超时回退静态补全。

### 2.3 非目标
- 不引入网络请求（补全必须离线）。
- 不引入大模型。
- 不替换当前 `xun init` 的 wrapper/alias 能力。

---

## 3. CLI 接口设计

### 3.1 新增命令
**生成补全脚本：**
```
xun completion <powershell|bash|zsh|fish>
```
输出补全脚本，供用户手动 `source` 或由 `xun init` 调用。

**动态补全入口（Cobra 风格，shell 预分词）：**
```
xun __complete <args...>
```
说明：
- shell 负责分词后传递参数
- 最后一个空字符串表示“在此位置按下 TAB”
- 可通过环境变量传递上下文（如 `XUN_COMPLETE_SHELL` / `XUN_COMPLETE_CWD`）
- Rust 端需处理 `--flag=partial` 形式（拆分等号为 `--flag` + `partial`）

### 3.2 候选数量控制
```
XUN_COMPLETE_LIMIT=0   # 默认：不限制（由硬上限兜底）
```
- 仅在候选巨大时启用（如书签名补全），推荐阈值 20-50。
- **硬上限**：无论 `LIMIT` 设置如何，单次输出不超过 200 条候选，防止极端场景卡死 shell 解析。

### 3.3 引号与空格处理
Shell 在调用 `__complete` 前已完成引号剥离。例如用户输入 `xun z "my pro<TAB>` 时，Rust 端收到的是 `my pro`（无引号）。
- 前缀匹配直接使用剥离后的文本。
- 返回含空格的候选时，**由 shell 补全脚本负责转义**（各 shell 转义规则不同），Rust 端原样输出。

---

## 4. 输出协议（__complete）

为跨 shell 简化解析，建议输出**行式文本**（不依赖 JSON）：

```
value<TAB>desc
value<TAB>desc
__XUN_COMPLETE__=ok<TAB>directive=5<TAB>v=1
```

说明：
- `value`：候选文本（不得包含 `\t` 和 `\n`，含这些字符的候选应跳过或替换为空格）
- `desc`：说明（可为空，同样禁止 `\t`/`\n`）
- `directive`：**bitmask 整数**，控制 shell 行为（参考 Cobra ShellCompDirective）

Directive 位定义：
| 位 | 值 | 名称 | 含义 |
| --- | --- | --- | --- |
| 0 | 1 | `NoFileComp` | 禁止回退文件补全 |
| 1 | 2 | `NoSpace` | 补全后不追加空格 |
| 2 | 4 | `FilterDirs` | 仅补全目录 |
| 3 | 8 | `FilterExt` | 限制文件扩展名（扩展名列表通过 `ext` 字段传递） |

示例：`directive=5` 表示 `NoFileComp | FilterDirs`（1+4）。

备注：
- 该 bitmask **参考但简化自** Cobra `ShellCompDirective`（值和集合不同），请勿直接复用 Cobra 常量。
- 建议在 Rust 端定义常量，避免魔数：
  ```
  const NO_FILE_COMP: u32 = 1;
  const NO_SPACE: u32 = 2;
  const FILTER_DIRS: u32 = 4;
  const FILTER_EXT: u32 = 8;
  ```

当 `FilterExt` 置位时，sentinel 行附加 `ext` 字段：
```
__XUN_COMPLETE__=ok<TAB>directive=8<TAB>ext=json|yaml<TAB>v=1
```
`ext` 使用 `|` 分隔，避免与其他字段分隔符产生歧义。

优势：shell 端用 bitwise AND 判断，比字符串 split + 匹配更快且无解析歧义。

**错误/回退：**
- 若计算失败或超时，输出 `__XUN_COMPLETE__=fallback`，脚本退回静态补全。

**零候选（正常无匹配）：**
- 仍输出 `__XUN_COMPLETE__=ok` + `directive`（例如 `directive=1`）
- 避免 shell 回退到文件补全（例如 `xun z <TAB>` 场景）

**退出码：**
- `0`：正常（包括零候选），shell 解析 stdout
- 非 `0`：异常，shell 忽略 stdout 并回退静态补全

**调试机制：**
设置环境变量启用补全调试日志：
```
XUN_COMP_DEBUG_FILE=C:\Temp\xun_comp.log
```
当此变量存在时，Rust 端将路由决策、候选数量、匹配耗时等信息追加写入该文件。
因为补全时 stdout 被 shell 捕获、stderr 常被丢弃，此机制是排查"按 TAB 无反应"问题的唯一可靠手段。

---

## 5. 智能排序设计

默认策略（保持简单且主流）：
1. **硬过滤**：前缀匹配（用户已输入部分）
2. **软排序**：复用已有 frecency（visit_count + last_visited）
3. **优先级**：subcommand > flag > value（静态优先级）

"上下文转移概率"等高级增强放到后续阶段；路径相关性（cwd 加分）已实现，见 §5.2。

### 5.1 Frecency 算法（zoxide-style）
采用 zoxide 的 4-bucket 时间衰减乘数，公式：`score = visit_count × multiplier`

| 时间桶 | 乘数 | 含义 |
| --- | --- | --- |
| < 1 小时 | 4.0 | 刚用过，强 boost |
| < 1 天 | 2.0 | 今天用过 |
| < 1 周 | 0.5 | 近期用过 |
| ≥ 1 周 | 0.25 | 较久未用 |

Aging 机制（防止 visit_count 无限增长）：
- 每次 `save_db` 时检查全局 `Σ visit_count`
- 超过阈值（默认 10000）时，所有 entry 的 `visit_count *= 0.9`
- `visit_count < 1` 的 entry 被移除
- 参考：zoxide `_ZO_MAXAGE` + 全局缩放策略
  
新用户/零历史：
- 若 frecency 数据为空或全部为 0，则按名称字典序排序（保证稳定、可预测）。

### 5.2 CWD 上下文加分（已实现）
当用户执行 `xun z` / `xun open` 时，书签路径与当前工作目录的关系会影响排序：

| 关系 | 乘数 | 场景 |
| --- | --- | --- |
| 完全相同 | ×2.0 | 用户就在书签目录中 |
| 书签是 cwd 祖先 | ×1.5 | 用户在书签的子目录里 |
| 书签是 cwd 后代 | ×1.3 | 书签在当前目录下层 |
| 无关 | ×1.0 | 不加分 |

- 最终评分：`fuzzy_score × (1 + frecency × weight) × cwd_boost`
- `list` 命令不启用 cwd 加分（传 `None`），仅导航命令启用。
- 补全场景（`__complete`）通过 `XUN_COMPLETE_CWD` 环境变量获取 cwd，复用同一函数。

### 5.3 排序稳定性（Tie-break）
当多个候选的最终评分相同时，按以下顺序 tie-break，保证结果确定性：
1. **静态优先级**：subcommand > flag > value（仅混合类型候选时生效）
2. **字典序**：同类型候选按名称升序（`a < b`）

实现要求：排序使用 **stable sort**（Rust `sort_by` 即稳定排序），确保相同评分的候选不会因排序算法而随机交换位置。

### 5.5 补全路由逻辑
`__complete` 收到 args 后，按以下顺序判断补全位置：
1. args 为空 / 仅一个不完整词 → 补全子命令
2. **全局 flag**：尚未识别子命令时，最后一个 arg 以 `--` 开头 → 补全全局 flag（`--verbose` 等）
3. **全局 flag 后续**：已消费全局 flag（如 `xun --verbose <TAB>`）→ 继续补全子命令
4. 若出现独立 `--`，其后全部视为位置参数（不再补全 flag）
5. 已识别子命令，最后一个 arg 以 `--` 开头 → 补全该子命令的 flag
6. 已识别子命令，前一个 arg 是需要值的 flag（如 `--profile`）→ 补全 flag value
7. 已识别子命令，位置参数位 → 补全位置参数（书签名 / 路径等）

候选源与 directive 示例：
| 位置 | 候选源 | directive | 含义 |
| --- | --- | --- | --- |
| 子命令 | 静态列表 | `1` | NoFileComp |
| flag 名 | 静态列表 | `3` | NoFileComp+NoSpace |
| `--profile` value | `config.redirect.profiles` | `1` | NoFileComp |
| `--format` value | `auto/table/tsv/json` | `1` | NoFileComp |
| `z/del/touch` 位置参数 | 书签名（frecency 排序） | `1` | NoFileComp |
| `set` 第二个位置参数 | 无候选 | `4` | FilterDirs |
| `redirect --undo` value | 最近 N 条 tx ID | `1` | NoFileComp |
| `redirect --tx` value | 最近 N 条 tx ID | `1` | NoFileComp |
| `config get/set` 第一个位置参数 | config key 列表（dot-path） | `1` | NoFileComp |
| `redirect` 位置参数（source dir） | 无候选 | `4` | FilterDirs |

说明：
- flag 名使用 `3`（NoFileComp+NoSpace），允许 `--flag=value` 与 `--flag value` 两种风格。
- 上表为代表性子集，完整路由随子命令扩展。

---

## 6. 数据与缓存

### 6.1 数据来源（复用现有）
- `~/.xun.json`：`visit_count` + `last_visited`（frecency）
- `~/.xun.visits.jsonl`：visit log（可扩展字段，但不新增补全专用文件）

### 6.2 缓存策略
- 进程内缓存 + 文件 mtime 校验
- 不新增持久化 cache 文件
- 需考虑并发补全（连续 TAB / 多 shell 进程同时请求）：缓存结构应使用 `OnceLock` / `RwLock` 等线程安全机制

---

## 7. Shell 集成策略

### 7.1 PowerShell
`Register-ArgumentCompleter` 调用 `xun __complete <args...>`，并将结果映射为 `CompletionResult`。

### 7.2 Bash/Zsh
`complete -F _xun_complete xun` 或 zsh compsys 中调用 `xun __complete <args...>`。

### 7.3 Fish
`complete -c xun -a "(xun __complete ...)"` 形式调用动态补全。

### 7.4 与 `xun init` 关系
- `init` 继续输出 wrapper/alias/魔法输出处理。
- 若检测到 `xun completion`，则 `init` 调用其输出补全脚本；否则使用旧脚本内置补全逻辑（兼容旧版本）。

---

## 8. 实施步骤（阶段任务清单）

**阶段 1：统一入口（低风险）**
- [x] 新增 `xun completion`（输出各 shell 补全脚本）
- [x] 新增 `xun __complete`（静态候选 + directive）
- [x] `init` 脚本优先加载 `completion`，失败回退旧逻辑
- [x] 支持 `__complete` 协议（sentinel + 退出码）

**阶段 2：动态数据源（中风险）**
- [x] `redirect --profile` / `--undo` / `--tx` 动态候选
- [x] `config get/set` dot-path key 列表
- [x] 书签名候选（前缀过滤）
- [x] 前缀过滤 + 静态优先级排序

**阶段 3：frecency 排序（低风险）**
- [x] 复用 `visit_count/last_visited` 排序
- [x] 候选上限策略（`XUN_COMPLETE_LIMIT` + 硬上限 200）

**阶段 4：上下文增强（可选）**
- [x] `XUN_COMPLETE_CWD` 路径加分
- [ ] 可配置权重与阈值

---

## 9. 兼容性与回滚

- 若 `xun __complete` 异常或超时，脚本自动回退静态补全。
- 可通过 `XUN_DISABLE_DYNAMIC_COMPLETE=1` 强制静态模式。

---

## 10. 性能约束

- 单次补全 < 50ms（目标）；>100ms 触发回退。
- 禁止网络请求；只读本地数据与缓存。
- 超时由 shell 脚本控制（例如 PowerShell 使用 Job + Timeout，Bash 使用 `timeout`），Rust 端不强制计时。
- 建议验收项：1 万条书签 + 连续 100 次补全，总耗时 < 500ms。
- 性能基准脚本与日志格式见：`./Completion-Benchmark.md`

---

## 11. 测试与验收

**功能测试**
- PowerShell/Bash/Zsh/Fish 基本补全可用。
- `redirect --profile` / `--undo` 动态候选正确。
- `XUN_COMPLETE_LIMIT` 仅在候选巨大时生效（默认 0）。

**性能测试**
- 10k 条历史日志下，补全耗时 < 50ms（可接受范围）。

**回退测试**
- 人为让 `__complete` 返回错误时，仍可静态补全。
- `v!=1` 或脚本检测异常时回退静态补全。
- `~/.xun.json` / `~/.xun.visits.jsonl` / 配置文件损坏或权限不足时安全降级为 fallback。

**边缘场景（需覆盖）**
- 路径含空格/引号/中文/emoji（尤其 PowerShell 转义场景）。
- `--flag=value` 与 `--` 终止 flag 解析。
- 零候选仍输出 `ok + directive`（避免回退文件补全）。
- 超长候选/大量候选（硬上限 200 生效）。
- 新用户零历史时的稳定排序。
- `XUN_COMPLETE_CWD` / `XUN_COMPLETE_SHELL` 缺失时的退化路径。

---

## 12. Open Questions

- `__complete` 输出需附带协议版本号（`v=1`）。脚本端可据此进行兼容性检查与降级提示。

---

## 13. Checklist

实施与验收清单见：`./Completion-Checklist.md`
