# xun find — 路径/元数据搜索设计

**版本记录**
- v0.3.4 (2026-02-23)：补充 regex 全匹配实现说明、Exact 规则大小写处理、tsv/json 元数据取值说明、时间范围解析优先级。
- v0.3.3 (2026-02-23)：移除多字符短选项残留、修正 metadata/file_type 说明、补充 out_println 语义、Exact 规则索引 key 策略、规则去重说明、filter-file 的 ! 行提示。
- v0.3.2 (2026-02-23)：对齐 argh 参数约束（仅单字符短选项）、补充 --dry-run/--test-path 语义、规则复用策略、ListFormat 复用、过滤组合语义、--count 优先级、base_dirs 默认值、路径匹配相对语义、性能与输出缓冲说明。
- v0.3.1 (2026-02-23)：细化语义（rule_idx 1-based、Exact 规则定义、`**` 行为、输出路径语义）。
- v0.3.0 (2026-02-23)：7 项改进（剪枝对齐 ffsearch1、rule_idx 编号、dry-run 目录标记、路径规范化、属性冲突校验、空目录性能警告、输出宏约定）。
- v0.2.0 (2026-02-23)：补充实现架构（零新增依赖、目录剪枝、模块结构、并行策略）。
- v0.1.0 (2026-02-23)：初版设计，对齐 `ffsearch1` 规则语法与优先级。

> 目标：提供极速的路径/元数据搜索（不做内容搜索），规则语法与 `ffsearch1` 完全对齐，输出稳定可脚本化。
> 约束：零新增依赖（仅复用已有 `regex` + `windows-sys`），核心逻辑手写移植自 ffsearch1。
> 补充（2026-02）：Dashboard Web UI 迭代不影响 find 语义与 CLI 协议。

---

## 1. 命令语法

```
xun find [path...]
```

| 参数/选项 | 说明 | 默认值 |
| --- | --- | --- |
| `[path...]` | 基础搜索目录（可多个） | 当前目录 |
| `-i, --include <pattern>` | Glob 规则：Include | - |
| `-e, --exclude <pattern>` | Glob 规则：Exclude | - |
| `--regex-include <pattern>` | Regex 规则：Include（RE2 语义） | - |
| `--regex-exclude <pattern>` | Regex 规则：Exclude（RE2 语义） | - |
| `--extension <list>` | 扩展名 Include（如 `cpp,h`） | - |
| `--not-extension <list>` | 扩展名 Exclude | - |
| `--name <list>` | 文件名 Include（等价于 glob include） | - |
| `-F, --filter-file <path>` | 从文件加载规则（glob，默认 Exclude） | - |
| `-s, --size <expr>` | 大小过滤 | - |
| `--fuzzy-size <expr>` | 模糊大小过滤 | - |
| `--mtime <expr>` | 修改时间过滤 | - |
| `--ctime <expr>` | 创建时间过滤 | - |
| `--atime <expr>` | 访问时间过滤 | - |
| `-d, --depth <expr>` | 深度过滤 | - |
| `--attribute <spec>` | 文件属性过滤 | - |
| `--empty-files` | 仅空文件 | - |
| `--not-empty-files` | 排除空文件 | - |
| `--empty-dirs` | 仅空目录 | - |
| `--not-empty-dirs` | 排除空目录 | - |
| `--case` | 大小写敏感 | false |
| `-c, --count` | 仅输出匹配数量 | false |
| `--dry-run` | 不扫描文件系统，仅用于规则测试/模拟 | false |
| `--test-path <path>` | 规则测试的虚拟路径（与 `--dry-run` 配合） | - |
| `-f, --format <auto|table|tsv|json>` | 输出格式 | auto |

补充约定：
- 结果数据 → stdout；提示/表格/错误 → stderr。
- `--format json|tsv` 字段稳定、可脚本化。
- 默认不要求管理员权限，MFT 快速路径作为后续可选优化。
- 选项的可重复性由字段类型控制（建议用 `Vec<String>` 解析可重复参数）。
- argh 的短选项仅使用单字符形式，因此本设计仅保留 `-i/-e/-s/-d/-c/-F` 等单字符短选项。
- find 为默认功能，不使用 feature gate。
- 当 `[path...]` 为空时，默认使用当前目录 `.`。

---

## 2. 规则优先级与默认行为（对齐 ffsearch1）

**优先级**：
1. **Exact 规则**（glob 且无 `* ? [ ]`，即使包含 `/` 仍视为 Exact）优先级最高  
2. **Fuzzy 规则**（含通配符或 regex）次之  
3. 同一类别中 **后规则覆盖前规则**

**默认状态**：
- 若存在任意 Include 规则 → 默认 **Exclude**
- 若不存在 Include 规则 → 默认 **Include**

**规则覆盖**：
规则文本以 `!` 开头表示 **强制 Include**（用于在 exclude 规则中反向包含）。

**目录规则**：
规则以 `/` 结尾 → 仅匹配目录（dir-only）。

**规则编译时机**：
所有规则在遍历前一次性编译为内部表示（`CompiledRule`），遍历时只做匹配，不做解析。
编译阶段同时完成 Exact/Fuzzy 分类与扩展名索引构建。

**规则合并顺序**（`--filter-file` 与命令行）：
```
filter-file 规则 → 命令行规则
```
命令行规则后出现，优先级更高（"后规则覆盖前规则"原则自然生效）。

---

## 3. Glob / Regex 语义（对齐 ffsearch1）

### 3.0 路径分隔符规范化

内部路径统一使用 `/` 作为分隔符（参考 ffsearch1 `scanner.cpp:229`）。
- 遍历时将 Windows `\` 替换为 `/`，再进行规则匹配。
- 用户输入的 glob/regex 中 `\` 仅作转义符，不作路径分隔符。
- 输出路径字段（tsv/json/路径列表）统一使用 `/`，与 ffsearch1 输出一致；表格展示可按平台转回原生分隔符。
- 规则匹配基于 `base_dir` 下的**相对路径**（不包含 `\\?\` 前缀，也不包含 `base_dir` 本身）。

### 3.1 Glob 语义

- `*` 匹配任意长度字符
- `?` 匹配单个字符
- `[abc]` / `[a-z]` 字符集合与范围
- `[!abc]` 或 `[^abc]` 取反
- `\` 用于转义特殊字符
- `**` 仅在 **锚定路径** 中作为"跨层目录"匹配（非锚定规则中等价于 `*`）

**实现方式**：手写移植自 ffsearch1（`match.cpp`），不引入 `globset` 等外部 crate。
核心两个函数：
- `glob_match_component(text, pattern, case_sensitive)` — 单段匹配（~80 行）
- `match_path_parts(path_parts, pattern_parts, case_sensitive)` — 多段匹配含 `**`（~20 行）

**锚定规则**：
glob 模式包含 `/` 视为 **锚定路径**，按完整相对路径逐段匹配；  
若以 `./` 或 `/` 开头，视为从根相对路径起匹配。

**非锚定规则**：
不含 `/` 的 glob 只匹配 **文件名（最后一个路径段）**。

### 3.2 Regex 语义

- 使用 RE2 语义（`regex` crate，已有依赖），执行 **全匹配**
  - regex 含 `/` → 匹配完整相对路径
  - 否则 → 仅匹配文件名
  - 实现时需对用户 regex 自动包裹 `^(?:...)$` 以实现全匹配语义

### 3.3 规则输入与列表

- `--extension` / `--not-extension`：逗号分隔，允许带点（`.rs`），会自动去点与空白  
  等价于规则 `*.ext` 的 include/exclude。
- `--name`：逗号分隔的文件名 glob（等价于 `--include`）。
- `--filter-file`：逐行读取 glob 规则  
  - 空行或 `#` 开头行跳过  
  - 默认视为 **Exclude**  
  - 行首 `!` 表示 **Include**
  - 注意：filter-file 中的 `!` 行（Include 规则）同样会触发“存在 Include 规则 → 默认 Exclude”的行为

注意：`--extension` 属于 Include 规则，会触发“存在 Include 规则 → 默认 Exclude”的行为。  
示例：仅使用 `--extension rs` 会排除非 `rs` 文件；若希望在默认 Include 下排除扩展名，请使用 `--not-extension`。

---

## 4. 过滤表达式语法（对齐 ffsearch1）

### 4.1 Size 过滤（`-s/--size`）

支持单位：`B`, `K`, `M`, `G`（1024 基），支持小数。

**比较操作**：
```
>10k   >=10k   <1M   <=1M   =512
```

**范围**：
```
5k-10k
[5k-10k]   (5k-10k)   [5k-10k)   (5k-10k]
```

> `[]` 闭区间，`()` 开区间。

**组合语义**：
- 多个 `--size` 之间为 **OR**（任一匹配即通过）。
- 多个时间过滤（`--mtime/--ctime/--atime`）之间为 **AND**（全部满足才通过）。

### 4.2 Fuzzy Size（`--fuzzy-size`）

仅接受“简单值”或“范围”，不允许 `><=[]()`。  
语义为“单位桶内的模糊范围”：
- `3M` → `(2M, 3M]`
- `5k-10k` → `(4k, 10k]`（按左侧单位向下扩展）

### 4.3 Time 过滤（`--mtime/--ctime/--atime`）

**时间点**：`YYYY.MM.DD` 或 `YYYY.MM.DD.HH.MM.SS`  
**相对时间**：`<N><unit>`，单位：`s|m|h|d|w`

**前缀语义**：
- `+`：早于（older / before）  
- `-`：晚于（newer / after）  
- `~`：附近（1 个单位窗口）

**示例**：
```
-7d       # 最近 7 天内
+30d      # 早于 30 天
~3d       # 约 3 天前（2-3 天窗口）
~2024.01.01   # 2024-01-01 当天
```

**范围表达式**：
```
10d-2d                  # 相对区间（无前缀）
2024.01.01-2024.01.31   # 绝对区间
2024.01.01-+7d          # 混合区间（相对部分必须带前缀）
```

**注意**：
- 单独的相对时间/绝对时间必须带前缀（`+`/`-`/`~`）。
- 相对区间 `10d-2d` 不允许带前缀。

**解析优先级**：
1) 若首字符是 `+/-/~` → 按单值时间点解析  
2) 否则尝试按区间解析（按分隔符 `-` 拆分）  
3) 仍失败则报错

**与 redirect 的语法差异**：  
`find` 使用前缀时间语法（`-7d/+30d/~3d`），`redirect` 的 age 使用比较操作符语法（`>1d`/`<=2w`）。两者面向场景不同，暂不强行统一；需在文档中明确这一差异。

### 4.4 Depth 过滤（`-d/--depth`）

```
>=2   >2   <5   <=5   =3
2-5   [2-5]   (2-5)   [2-5)   (2-5]
```

### 4.5 属性过滤（`--attribute`）

格式：`+` 表示必须具备，`-` 表示必须不具备；可逗号/空格分隔。

可用属性：
- `h` hidden
- `r` readonly
- `s` system
- `l` reparse point（链接/联接）

示例：
```
--attribute +h,-l
```

**冲突校验**（参考 ffsearch1 `args.cpp:545`）：
同一属性同时出现 `+` 和 `-`（如 `+h,-h`）为逻辑矛盾，解析阶段直接报错：
```
Error: Attribute conflict: 'h' cannot be both required (+) and forbidden (-).
```

### 4.6 空文件/空目录

- `--empty-files` / `--not-empty-files`
- `--empty-dirs` / `--not-empty-dirs`

**性能注意**（参考 ffsearch1 `scanner.cpp:23-63`）：
`--empty-dirs` / `--not-empty-dirs` 需要对每个目录额外执行一次打开+枚举操作（`CreateFileW` + `GetFileInformationByHandleEx`）以判断是否为空。
在目录数量极大的场景下会产生可观的额外 I/O 开销。仅在用户显式指定时启用此检查。

---

## 5. 输出格式

**输出管道约定**：复用 xun 已有的输出宏，保持全项目一致性：
- `out_println!` → stdout（结果数据：tsv/json/count/路径列表）
- `ui_println!` → stderr（表格、提示、dry-run 决策过程、错误信息）

说明：`out_println!` 直通 stdout（不受 `--quiet` 控制），`ui_println!` 会受 `--quiet` 影响。stdout 的机器可读结果默认不应被 `--quiet` 抑制。

**输出路径语义**：
- 输出路径为 `base_dir + 相对路径`；`base_dir` 使用用户输入的原始字符串（不做 canonical 归一化），与 ffsearch1 行为一致。
- 因此当用户输入相对路径时，输出可能是相对路径。

### 5.1 `table`（stderr）

建议列：
`Path` | `Type` | `Size` | `Mtime` | `Rule` | `Source`

> `Rule`/`Source` 在 `--verbose` 时显示。

### 5.2 `tsv`（stdout）

稳定字段：
```
path    is_dir  size  mtime  depth  rule_type  rule_idx  decision_source
```
说明：`tsv/json` 输出包含 `size/mtime`，会强制读取元数据（即便未启用 size/time 过滤）。

### 5.3 `json`（stdout）

推荐结构：
```
{
  "query": { "paths": [...], "filters": {...}, "case_sensitive": false },
  "results": [
    { "path": "...", "is_dir": false, "size": 123, "mtime": "...", "depth": 2,
      "rule_type": "include", "rule_idx": 12, "decision_source": "explicit|inherited|secondary" }
  ]
}
```

### 5.4 `--count`

`--count` 优先级高于 `--format`，仅输出整数数量（stdout）。

### 5.5 `--dry-run` + `--test-path <path>` 规则测试

不扫描文件系统，仅对虚拟路径执行规则匹配并输出决策过程（stderr）。  
`--test-path` 存在时隐式启用 `--dry-run`。

**目录标记约定**：虚拟路径以 `/` 结尾视为目录（`is_dir=true`），否则视为文件。
这使得 dir-only 规则（以 `/` 结尾的 glob）能被正确测试。

```
path: "test/foo.rs"  (is_dir=false)
  → Rule #3 (include, glob, "*.rs") → INCLUDE
  → Rule #7 (exclude, glob, "test/") → SKIP (dir-only, path is file)
  → Decision: INCLUDE (source: explicit rule #3)

path: "test/"  (is_dir=true)
  → Rule #7 (exclude, glob, "test/") → EXCLUDE
  → Decision: EXCLUDE (source: explicit rule #7)
```

> 说明：`Rule #N` 为 1-based 编号，与 `rule_idx` 字段一致。

用途：在实际遍历前验证规则正确性，尤其适合调试复杂的 `--filter-file`。

---

## 6. 目录剪枝策略

遍历子目录时，需决定是否进入（参考 ffsearch1 `scanner.cpp:424`）：

**基本规则**：
被 Exclude 规则**显式匹配**的目录 → 跳过整个子树（不进入）。
未被显式匹配、或匹配结果为 Include 的目录 → 进入递归。

> **注意**：ffsearch1 不做 `!` 规则前瞻。即使存在 `!` 强制 Include 规则可能匹配被剪枝目录下的内容，
> 该目录仍会被跳过。这是有意的设计取舍——剪枝性能优先于 `!` 规则的完备性。
> 用户若需保留某个被 Exclude 的目录下的文件，应调整规则使该目录本身不被 Exclude。

**inherited_state 传递**：父目录的匹配结果作为子目录的默认状态传递。
子目录内的文件若无显式规则命中，继承父目录的 Include/Exclude 状态。

伪代码：
```
if is_dir && !(has_explicit_match && final_state == Exclude) {
    recurse_into(subdir, inherited_state = final_state)
}
```

---

## 7. 实现架构

### 7.1 模块结构

```
src/find/
  mod.rs              // pub mod + FindCmd 入口
  rules.rs            // 规则编译：Exact/Fuzzy 分类、扩展名索引构建
  matcher.rs           // 匹配引擎：glob_match_component、match_path_parts、determine_path_state
  walker.rs            // 目录遍历：初版 std::fs，后续 NT API
  filters/
    mod.rs
    size.rs            // Size + FuzzySize 解析与匹配
    time.rs            // mtime/ctime/atime 解析与匹配
    depth.rs           // Depth 解析与匹配
    attr.rs            // Windows 属性过滤
```

复用与收敛策略：
- `matcher.rs` 的 glob 匹配为 `util::glob_match` 的超集，实现完成后应迁移 `util::glob_match` 与 `matches_patterns` 以避免双引擎。
- 输出格式复用现有 `ListFormat`（`src/model.rs`）与输出辅助（`src/output.rs`）。

### 7.2 规则内部表示

```rust
enum CompiledRule {
    ExactPattern(String),                 // 不含通配符的完整模式（可含 `/`）
    Extension(String),                    // "*.rs" → 扩展名 HashMap O(1)
    GlobPattern { parts: Vec<String>, .. }, // 含通配符 → glob 匹配
    RegexPattern(regex::Regex),           // 用户 regex → 全匹配
}
```

Exact 规则索引建议：
- `exact_by_path: HashMap<String, Vec<RuleRef>>`（锚定规则，key=相对路径）
- `exact_by_name: HashMap<String, Vec<RuleRef>>`（非锚定规则，key=文件名）
匹配时先查 `exact_by_path`，再查 `exact_by_name`。
当 `case_sensitive = false` 时，索引 key 与查找 key 均转为小写。

分流逻辑（参考 ffsearch1 `args.cpp:568-573`）：
- 无 `* ? [ ]` 且非 regex → `exact_rules`（HashMap 查找）
- `*.ext` 形式 → `ext_rule_index: HashMap<ext, Vec<usize>>`（扩展名快速索引）
- 其余 → `fuzzy_rules`（顺序匹配，从后向前）

**rule_idx 编号规则**：
规则编号按分类后的存储顺序分配，**非输入顺序**（1-based，与 ffsearch1 一致）：
- Exact 规则：`1..=exact_rules.len()`
- Fuzzy 规则：`exact_rules.len()+1 ..= exact_rules.len() + fuzzy_rules.len()`

输出（tsv/json）中的 `rule_idx` 使用此编号，便于与内部数据结构对应。
`--dry-run` 输出中的 `Rule #N` 同样使用此编号。
实现时需显式将 1-based `rule_idx` 映射到 Rust 的 0-based 索引（参考 ffsearch1 `get_rule_by_index`）。

规则去重：初版不做规则去重；后续可参考 ffsearch1 的两阶段去重（文本去重 + 语义去重）。

### 7.3 匹配决策流程

对每个路径条目（参考 ffsearch1 `match_trace.cpp:99-217`）：

```
1. Exact 规则（从后向前）→ 命中则立即返回
2. Fuzzy 规则：
   a. 提取扩展名 → 查 ext_rule_index 获取候选索引
   b. 合并 non_ext_fuzzy_indices
   c. 按索引从大到小排序（后规则优先）
   d. 逐条匹配 → 命中则立即返回
3. 均未命中 → 返回 inherited_state（默认状态）
```

补充：当 `verbose >= 4` 或未构建扩展名索引时，Fuzzy 规则走全量遍历并打印每条规则的匹配状态（对齐 ffsearch1 `match_trace.cpp:142`）。

### 7.4 遍历引擎（分阶段）

| 阶段 | 方式 | 预期性能 |
|------|------|----------|
| 初版 | `std::fs::read_dir` 递归 | 基线 |
| 优化 1 | `windows-sys` 调用 `FindFirstFileExW` + `FIND_FIRST_EX_LARGE_FETCH` | ~2x |
| 优化 2 | `NtQueryDirectoryFile` + 64KB buffer（ffsearch1 方案） | ~3-5x |
| 优化 3 | MFT 直读（需管理员） | ~10-100x |

初版不写任何 Windows 特定代码。后续优化通过 `#[cfg(windows)]` 逐步引入，`windows-sys` 已有。

### 7.5 并行策略

初版单线程。后续并行参考 ffsearch1（`scanner.cpp:432`）：

```
浅层目录（depth < 2）→ 提交到线程池并行处理
中层目录（depth < 6 且线程池空闲）→ 并行
深层目录 → 内联递归（避免任务调度开销）
```

### 7.6 性能要点

- 路径分割：按 `/` 一次性分段为 `Vec<&str>`，避免重复 split
 - 输出：大量结果时建议在 `find` 内部使用 `BufWriter<Stdout>` 缓冲写；`out_println!` 目前每行会 `println!` 锁 stdout
- 每线程缓冲区复用：`filename_buf`、`path_buf`、`path_parts`（后续并行时）
- 扩展名索引：`HashMap<String, Vec<usize>>` 跳过无关 fuzzy 规则
- Windows 平台上 `DirEntry::file_type()` 通常无需额外系统调用（来自枚举结果）；`DirEntry::metadata()` 仍需额外打开句柄。初版应优先用 `file_type()` 判断目录/文件，仅在需要 size/time 过滤或输出包含 `size/mtime` 时调用 `metadata()`。

---

## 8. 依赖决策

| crate | 决策 | 理由 |
|-------|------|------|
| `regex` | ✅ 已有 | RE2 语义，等价 ffsearch1 的 re2 |
| `windows-sys` | ✅ 已有 | 后续 NT API 优化所需 |
| `globset` | ❌ 不引入 | ffsearch1 手写 glob ~100 行；globset 将 glob 编译为 regex，对 `*.ext` 等高频模式过重 |
| `ignore` | ❌ 不引入 | 过滤语义不匹配（gitignore vs ffsearch1 规则优先级）；目录剪枝控制粒度不够；依赖链重 |
| `walkdir` | ❌ 不引入 | 初版 `std::fs::read_dir` 足够；后续直接升级 NT API，中间层无价值 |

**原则**：零新增依赖。核心逻辑（glob 匹配、规则引擎、过滤器、遍历）全部手写，参考 ffsearch1 算法设计。

---

## 9. 非目标

- 不包含内容搜索（grep），留给下一阶段 `xun grep`。
- 不实现跨平台（Windows 优先）。
- 不引入持久索引（后续若需要再评估）。
- 不引入新的外部 crate（glob/遍历/过滤全部手写）。
