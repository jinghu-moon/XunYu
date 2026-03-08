# Diff 组件实施方案

> 目标：高性能、低 CPU、低内存、速度快
> 生成时间：2026-02-26

## 一、架构总览

```
src/
├── diff/                     # 新增模块（feature = "diff"）
│   ├── mod.rs                # 公开 API + 路由调度
│   ├── types.rs              # 所有输出类型（DiffResult, Hunk, Stats...）
│   ├── line.rs               # 行级 diff（imara-diff 封装）
│   ├── ast.rs                # AST 符号级 diff（tree-sitter）
│   ├── lang.rs               # 语言配置 + tree-sitter Query 常量
│   └── vue.rs                # Vue SFC 拆分 + 分段 diff
├── commands/
│   ├── diff.rs               # CLI 子命令 `xun diff`（新增）
│   └── dashboard/
│       ├── mod.rs            # 追加 /api/diff 路由
│       └── handlers.rs       # 追加 diff_handler
```

设计原则：
- 单 crate + feature gate，与现有 `lock`/`crypt`/`dashboard` 模式一致
- `src/diff/` 是纯计算模块，零 IO 依赖（文件读取由调用方负责）
- CLI 和 Dashboard 共享同一套 diff 核心逻辑

## 二、依赖与 Feature Gate

```toml
# Cargo.toml 新增

[features]
diff = [
    "dep:imara-diff",
    "dep:tree-sitter",
    "dep:tree-sitter-javascript",
    "dep:tree-sitter-typescript",
    "dep:tree-sitter-css",
    "dep:tree-sitter-rust",
    "dep:tree-sitter-html",
]

[dependencies]
# 行级 diff —— 已验证最快（p50=17.3μs，比 diffy 快 24x）
imara-diff = { version = "0.1", optional = true }

# AST 解析
tree-sitter            = { version = "0.24", optional = true }
tree-sitter-javascript = { version = "0.23", optional = true }
tree-sitter-typescript = { version = "0.23", optional = true }
tree-sitter-css        = { version = "0.23", optional = true }
tree-sitter-rust       = { version = "0.23", optional = true }
tree-sitter-html       = { version = "0.23", optional = true }
```

**不引入的依赖（及理由）：**
- `similar` — imara-diff 完全替代，无需两个行级 diff 库
- `sha2` — 符号 hash 用 `std::hash::DefaultHasher`（SipHash），本地 diff 不需要密码学 hash
- `memmap2` — 512KB 上限下 `std::fs::read_to_string` 已足够快，mmap 在小文件上反而有 syscall 开销
  （来源：[Rust Forum: fast file IO](https://users.rust-lang.org/t/how-to-do-fast-file-io/8278) — mmap 对小文件无优势）
- `tree-sitter-c` / `tree-sitter-cpp` — 控制二进制体积；`.c/.cpp/.h/.hpp` 先走行级 diff，后续按需引入（详见 lang.rs 语言配置表）
- `tree-sitter-scss` / `tree-sitter-less` — 社区 grammar 成熟度不足，`.scss/.less` 走行级 diff

## 三、性能策略（核心）

### 3.1 内存控制

| 策略 | 实现方式 | 依据 |
|------|---------|------|
| 文件大小上限 | 读取前 `fs::metadata().len()` 检查，超过 512KB 直接拒绝 | 避免大文件吃内存 |
| 零额外拷贝的 token 化 | imara-diff 的 `InternedInput` 内部做 token intern，不复制原始字符串 | [imara-diff README](https://lib.rs/crates/imara-diff)：pointer compression 减少内存 |
| tree-sitter Cursor 遍历 | 用 tree-sitter 内置的 `TreeCursor` API 而非递归收集节点，避免堆分配（不引入额外 crate） | tree-sitter 官方文档：`TreeCursor` 是栈上状态机，遍历过程零额外堆分配 |
| 符号源码切片引用 | `Symbol.source` 存 `&str`（生命周期绑定原始字符串），不 clone | 避免每个符号复制一份源码 |
| Parser 复用 | `thread_local!` 缓存 `Parser` 实例，避免重复创建 | tree-sitter Parser 创建有初始化开销 |

### 3.2 CPU 控制

| 策略 | 实现方式 | 依据 |
|------|---------|------|
| Histogram 算法（默认，可切换） | `imara_diff::Algorithm::Histogram`，支持 `--diff-algorithm` 切换为 Myers/Patience | [imara-diff](https://github.com/pascalkuthe/imara-diff)：Histogram 比 Myers 快 10-100%；[Git diff-algorithm](https://git-scm.com/docs/diff-algorithm-option.html)：多算法可选是主流设计 |
| SipHash 替代 SHA256 | `std::hash::DefaultHasher` 做符号 hash 精确匹配 | SipHash 是 CPU cache 友好的 64-bit hash，比 SHA256 快一个数量级 |
| 符号级而非节点级 diff | 提取顶层符号 → hash 匹配 → 名称匹配 → 符号内行级 diff | 对比 difftastic 的 Dijkstra 图搜索 O(n²)，符号级匹配是 O(n) |
| 短路求值 | hash 相同 → 直接标记 unchanged，跳过行级 diff | 大部分符号未变化，短路避免无意义计算 |
| Dashboard spawn_blocking | tree-sitter 解析放入 `tokio::task::spawn_blocking` | CPU 密集型不阻塞 async 事件循环（cfx 参考实现同策略） |

### 3.3 速度优化

| 策略 | 实现方式 | 依据 |
|------|---------|------|
| 严格相等检测 | diff 前 `if old == new` 全量比较，相同直接返回 Identical | 已读入字符串，O(n) 比较远快于 diff 计算且零误判 |
| imara-diff postprocess | `Diff::postprocess_lines()` 应用 gnu-diff 启发式优化输出可读性 | [imara-diff docs](https://docs.rs/imara-diff)：内置 heuristic |
| Query 编译缓存 | `lazy_static!` 或 `OnceLock` 缓存编译后的 `tree_sitter::Query` | Query 编译是一次性开销，后续复用 |
| 提前终止 | 符号数 > 500 或行数 > 10000 时降级为纯行级 diff | 防止 AST 提取在超大文件上耗时过长 |

## 四、核心类型定义

```rust
// src/diff/types.rs

use serde::{Deserialize, Serialize};

/// Diff 请求（CLI 和 Dashboard 共用）
pub struct DiffRequest<'a> {
    pub old: &'a str,              // 旧文件内容（调用方负责读取）
    pub new: &'a str,              // 新文件内容
    pub ext: &'a str,              // 文件扩展名（小写，无点号）
    pub mode: DiffMode,            // auto | line | ast
    pub algorithm: DiffAlgorithm,  // 行级 diff 算法选择
    pub context: usize,            // 上下文行数，默认 3
    pub whitespace: WhitespaceOpt, // 空白处理选项
}

#[derive(Default, Clone, Copy)]
pub enum DiffMode {
    #[default]
    Auto,   // 按扩展名自动选择
    Line,   // 强制行级
    Ast,    // 强制 AST
}

/// 行级 diff 算法（对齐 Git diff-algorithm 选项）
/// 参考：https://git-scm.com/docs/diff-algorithm-option.html
#[derive(Default, Clone, Copy)]
pub enum DiffAlgorithm {
    Myers,      // 经典 Myers，最小编辑距离
    Minimal,    // Myers + 最小化启发式（更慢但更紧凑）
    Patience,   // 基于 LCS 的 patience diff（可读性好）
    #[default]
    Histogram,  // Git histogram diff（默认，性能最优）
}

/// 空白 / 行尾处理选项
/// 参考：https://www.gnu.org/s/diffutils/manual/html_node/White-Space.html
#[derive(Default, Clone)]
pub struct WhitespaceOpt {
    /// 忽略行内空白量变化（类似 GNU diff -b）
    pub ignore_space_change: bool,
    /// 忽略所有空白（类似 GNU diff -w）
    pub ignore_all_space: bool,
    /// 忽略空行差异（类似 GNU diff -B）
    pub ignore_blank_lines: bool,
    /// 剥离行尾 CR，消除 CRLF/LF 噪声（Windows 环境高频需求）
    pub strip_trailing_cr: bool,
}

/// 统一输出
#[derive(Serialize)]
pub struct DiffResult {
    pub kind: DiffResultKind,  // "identical" | "line" | "ast" | "binary"
    pub stats: DiffStats,
    pub hunks: Vec<Hunk>,
}

#[derive(Serialize)]
pub struct DiffStats {
    pub added: u32,
    pub removed: u32,
    pub modified: u32,
    pub unchanged: u32,
    /// 统计粒度："line"（行级/行级模式）| "symbol"（AST 模式符号级）
    /// 前端据此决定展示文案（如 "3 symbols modified" vs "12 lines added"）
    ///
    /// 组合规则（不变量，前端可依赖）：
    ///   DiffResultKind::Ast       → StatsUnit::Symbol
    ///   DiffResultKind::Line      → StatsUnit::Line
    ///   DiffResultKind::Identical → StatsUnit::Line（stats 全零，unit 无实际意义）
    ///   DiffResultKind::Binary    → StatsUnit::Line（stats 全零，unit 无实际意义）
    pub unit: StatsUnit,
}

#[derive(Serialize)]
pub enum StatsUnit { Line, Symbol }

/// 行级 hunk
#[derive(Serialize)]
pub struct Hunk {
    pub kind: HunkKind,
    pub symbol: Option<String>,       // AST 模式下的符号名
    pub symbol_type: Option<String>,  // function / struct / ...
    pub section: Option<String>,      // Vue SFC: template / script / style
    pub old_start: u32,               // 1-indexed
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Serialize)]
pub struct DiffLine {
    pub tag: LineTag,          // context | add | remove
    pub content: String,
}

#[derive(Serialize)]
pub enum LineTag { Context, Add, Remove }

#[derive(Serialize)]
pub enum HunkKind { Added, Removed, Modified, Unchanged }

#[derive(Serialize)]
pub enum DiffResultKind { Identical, Line, Ast, Binary }
```

## 五、模块实现规格

### 5.1 mod.rs — 入口路由

```
pub fn diff(req: DiffRequest) -> DiffResult
```

路由逻辑：
1. **二进制检测**：扫描前 8KB 是否含 NUL 字节（`\0`）
   - 检测到 NUL 且未指定 `--text` → 返回 `DiffResult { kind: Binary, .. }`
   - 指定 `--text` → 强制按文本处理（对齐 GNU diff `--text` / `-a`）
   - 参考：https://www.gnu.org/s/diffutils/manual/html_node/diff-Options.html
2. **空白预处理**：根据 `WhitespaceOpt` 在 diff 前对 old/new 做规范化
   - `strip_trailing_cr` → 逐行去除 `\r`（Windows 高频场景）
   - `ignore_all_space` → 逐行去除所有空白后比较
   - `ignore_space_change` → 连续空白压缩为单个空格后比较
   - `ignore_blank_lines` → 过滤空行后比较
   - 预处理产生临时 `Cow<str>`，不修改原始输入
3. **严格相等检测** `if old == new`（预处理后）→ 返回 Identical
   - ⚠️ **语义澄清**：当开启空白忽略选项时，Identical 表示"在当前忽略规则下相同"，
     原始文件可能仍有空白差异。
   - `DiffResult` 增加标记字段：
     ```rust
     /// 是否在忽略规则下判定为相同（原始内容可能不同）
     pub identical_with_filters: bool,
     ```
   - CLI 输出区分：
     - 无忽略选项 + Identical → `"Files are identical"`
     - 有忽略选项 + Identical → `"Files are identical (ignoring whitespace)"`
   - JSON 输出中 `identical_with_filters: true` 供前端展示提示
4. `DiffMode::Line` → 直接调用 `line::diff_lines()`
5. `DiffMode::Ast` → 调用 `ast::diff_ast()`，失败时 fallback 到 line
6. `DiffMode::Auto` →
   - `.vue` → `vue::diff_vue()`
   - `.js/.ts/.rs/.css/.html` 等有 tree-sitter 支持 → `ast::diff_ast()`
   - `.c/.cpp/.h/.hpp/.scss/.less` 等无 grammar → `line::diff_lines()`
   - 其他扩展名 → `line::diff_lines()`

### 5.2 line.rs — imara-diff 封装

```
pub fn diff_lines(old: &str, new: &str, algorithm: DiffAlgorithm, context: usize) -> Vec<Hunk>
```

实现要点：
1. `InternedInput::new(old, new)` — 按行 tokenize + intern
2. 算法映射（imara-diff 仅提供 Myers 和 Histogram 两种实现）：
   - `DiffAlgorithm::Histogram` → `imara_diff::Algorithm::Histogram`（直接映射）
   - `DiffAlgorithm::Myers` → `imara_diff::Algorithm::Myers`（直接映射）
   - `DiffAlgorithm::Patience` → `imara_diff::Algorithm::Histogram`
     ⚠️ **近似映射**：imara-diff 无独立 patience 实现，histogram 是 patience 的超集变体，
     行为最接近但不完全等价。CLI `--help` 和 JSON 输出须标注：
     `"patience → histogram (approximate)"`
   - `DiffAlgorithm::Minimal` → `imara_diff::Algorithm::Myers`
     ⚠️ **近似映射**：imara-diff 的 Myers 已内置最小编辑距离语义，
     但不保证与 Git `--minimal` 的启发式完全一致。CLI `--help` 须标注：
     `"minimal → myers (approximate)"`
   - **DiffResult 增加实际算法字段**：
     ```rust
     pub struct DiffResult {
         // ...existing fields...
         /// 实际使用的算法（当近似映射时与请求不同）
         pub actual_algorithm: DiffAlgorithm,
     }
     ```
     当 requested ≠ actual 时，CLI 输出提示行：
     `"Note: --diff-algorithm=patience mapped to histogram (imara-diff approximation)"`
3. `Diff::compute(algo, &input)` — 按选定算法计算
4. `diff.postprocess_lines(&input)` — 启发式优化
4. `diff.hunks()` 迭代 → 生成带 context lines 的 `Hunk`
5. 上下文行合并：相邻 hunk 间距 ≤ 2×context 时合并（与 unified diff 行为一致）

### 5.3 ast.rs — 符号级 AST diff

```
pub fn diff_ast(old: &str, new: &str, ext: &str, context: usize)
    -> Result<Vec<Hunk>, AstDiffError>
```

三阶段流水线：

**阶段 1：符号提取**
- `lang::get_language(ext)` → `(Language, &'static str query_src)`
- `thread_local! { static PARSER: RefCell<Parser> }` 复用 Parser
- `QueryCursor` 遍历 AST，提取顶层符号 `(name, kind, start_line, end_line, hash, source_slice)`
- hash 用 `std::hash::DefaultHasher`（SipHash-1-3）

**阶段 2：符号匹配**（O(n) 贪心）
- Pass 1：hash 精确匹配 → `Unchanged`（短路，不做行级 diff）
- Pass 2：同名匹配 → `Modified`（对符号内部调用 `line::diff_lines()`）
- 剩余 old → `Removed`，剩余 new → `Added`

**阶段 3：输出组装**
- Modified 符号的行级 diff 带 context lines
- Added/Removed 符号只输出元信息，不输出全部源码（减少输出体积）

**降级条件（返回 Err，由 mod.rs fallback 到 line）：**
- tree-sitter parse 返回 None
- 提取的符号数 > 500
- 源文件行数 > 10,000

### 5.4 lang.rs — 语言配置

支持的语言（5 种 AST + 行级回退覆盖）：

| 扩展名 | 策略 | tree-sitter grammar | Query 提取目标 |
|--------|------|-------------------|---------------|
| `.js` `.mjs` `.cjs` | AST | tree-sitter-javascript | function, class, variable, export |
| `.ts` `.mts` `.cts` | AST | tree-sitter-typescript | function, class, interface, type, variable, export |
| `.css` | AST | tree-sitter-css | rule_set, at_rule |
| `.rs` | AST | tree-sitter-rust | fn, struct, enum, trait, impl, const, type |
| `.html` | AST | tree-sitter-html | element（顶层） |
| `.c` `.cpp` `.cc` `.cxx` `.h` `.hpp` | 行级 | — | 不引入 grammar，行级 diff 覆盖 |
| `.scss` `.sass` `.less` | 行级 | — | tree-sitter-css 无法正确解析 SCSS/Less 语法，回退行级 |
| `.vue` | 分段 AST | 见 vue.rs | SFC 拆分后按 lang 属性分派 |
| 其他（`.toml` `.yaml` `.json` `.md` `.txt` 等） | 行级 | — | 通用行级 diff |

**设计决策说明：**
- `.c/.cpp/.h/.hpp`：不引入 `tree-sitter-c` / `tree-sitter-cpp` grammar 以控制二进制体积。
  行级 diff 对 C/C++ 文件已足够实用，后续可按需引入。
- `.scss/.less`：tree-sitter-css 的 grammar 仅覆盖标准 CSS，
  SCSS 的嵌套规则、`$variable`、`@mixin` 等语法会导致解析错误。
  明确回退行级，避免误解析产生错误 diff。

Query 常量直接复用 cfx 参考实现（`reference/diff/code.rs:191-238`），已验证可用。

### 5.5 vue.rs — Vue SFC 处理

```
pub fn diff_vue(old: &str, new: &str, context: usize) -> Vec<Hunk>
```

改进 cfx 的简陋字符串匹配，用正则提取并识别 `lang` 属性：

```rust
// 支持 <script setup lang="ts"> 等带属性标签，同时捕获 lang 值
static RE_OPEN: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r#"(?m)^<(template|script|style)(\s[^>]*)?\s*>"#).unwrap()
);
static RE_CLOSE: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r#"(?m)^</(template|script|style)>"#).unwrap()
);
// 从属性字符串中提取 lang="xxx"
static RE_LANG: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r#"lang=["'](\w+)["']"#).unwrap()
);
```

**lang 属性分派规则：**

| 标签 | lang 值 | 分派目标 |
|------|---------|---------|
| `<template>` | 无 / `html` | `ast::diff_ast(content, "html")` |
| `<script>` | 无 / `js` | `ast::diff_ast(content, "js")` |
| `<script lang="ts">` | `ts` | `ast::diff_ast(content, "ts")` |
| `<script setup lang="ts">` | `ts` | `ast::diff_ast(content, "ts")` |
| `<style>` | 无 / `css` | `ast::diff_ast(content, "css")` |
| `<style lang="scss">` | `scss` | `line::diff_lines()`（SCSS 无 grammar） |
| `<style lang="less">` | `less` | `line::diff_lines()`（Less 无 grammar） |

拆分后：
- 每段根据 tag + lang 属性选择 AST 或行级 diff
- 每段 hunk 的行号偏移回原始 SFC 行号
- 多个同名标签（如两个 `<style>`）均独立处理

## 六、CLI 集成

```rust
// src/commands/diff.rs

#[derive(FromArgs)]
#[argh(subcommand, name = "diff")]
/// 比较两个文件的差异
pub struct DiffCmd {
    /// 旧文件路径
    #[argh(positional)]
    pub old: String,

    /// 新文件路径
    #[argh(positional)]
    pub new: String,

    /// diff 模式：auto（默认）| line | ast
    #[argh(option, default = "\"auto\".to_string()")]
    pub mode: String,

    /// diff 算法：histogram（默认）| myers | minimal | patience
    /// 参考 git diff --diff-algorithm
    #[argh(option, default = "\"histogram\".to_string()")]
    pub diff_algorithm: String,

    /// 输出格式：text（默认）| json
    #[argh(option, default = "\"text\".to_string()")]
    pub format: String,

    /// 上下文行数（默认 3，对齐 GNU diff -U）
    #[argh(option, default = "3")]
    pub context: usize,

    /// 文件大小上限，如 512K、1M（默认 512K）
    #[argh(option, default = "\"512K\".to_string()")]
    pub max_size: String,

    // ── 空白处理选项（对齐 GNU diff） ──

    /// 忽略行内空白量变化（类似 -b）
    #[argh(switch)]
    pub ignore_space_change: bool,

    /// 忽略所有空白（类似 -w）
    #[argh(switch)]
    pub ignore_all_space: bool,

    /// 忽略空行差异（类似 -B）
    #[argh(switch)]
    pub ignore_blank_lines: bool,

    /// 剥离行尾 CR，消除 CRLF/LF 噪声
    #[argh(switch)]
    pub strip_trailing_cr: bool,

    // ── 二进制处理 ──

    /// 强制按文本处理二进制文件（类似 GNU diff --text / -a）
    #[argh(switch)]
    pub text: bool,
}
```

执行流程：
1. 解析 `--max-size` → 字节数
2. `fs::metadata()` 检查两个文件大小
3. **统一读取**：始终 `fs::read()` 获取 `Vec<u8>`，再 `String::from_utf8_lossy()` 转为 `Cow<str>`
   - CLI 层**不做**二进制判断，仅负责读取和传参
   - 二进制检测（NUL 扫描）、`--text` 强制逻辑**全部由核心 `diff::diff()` 处理**
   - 这确保 CLI 与 Dashboard 对同一文件产生完全一致的结果
4. 构建 `WhitespaceOpt` + `DiffAlgorithm` 从 CLI 参数
5. 从文件扩展名推断 `ext`
6. 调用 `diff::diff(DiffRequest { ... })`
   - 核心返回 `DiffResult { kind: Binary, .. }` 时，CLI 输出 "Binary files differ"
7. `--format text` → 彩色 unified diff 输出（复用 `console` crate）
8. `--format json` → `serde_json::to_string(&result)`

### CLI 注册

```rust
// src/cli.rs 追加
#[cfg(feature = "diff")]
mod diff;
#[cfg(feature = "diff")]
pub use diff::DiffCmd;

// SubCommand 枚举追加
#[cfg(feature = "diff")]
Diff(DiffCmd),

// src/commands/mod.rs dispatch 追加
#[cfg(feature = "diff")]
SubCommand::Diff(a) => diff::cmd_diff(a),
```

## 七、Dashboard API 集成

```rust
// dashboard/mod.rs build_router() 追加
#[cfg(feature = "diff")]
let r = r.route("/api/diff", post(handlers::diff_handler));
```

Handler：

```rust
// dashboard/handlers.rs 追加

#[derive(Deserialize)]
pub struct DiffApiRequest {
    pub old_path: String,              // 相对路径（正斜杠）
    pub new_path: String,
    pub mode: Option<String>,          // auto | line | ast
    pub algorithm: Option<String>,     // histogram | myers | minimal | patience
    pub context: Option<usize>,        // 上下文行数，默认 3
    pub ignore_space_change: Option<bool>,
    pub ignore_all_space: Option<bool>,
    pub ignore_blank_lines: Option<bool>,
    pub strip_trailing_cr: Option<bool>,
    pub force_text: Option<bool>,      // 强制按文本处理二进制
}

pub async fn diff_handler(
    Json(req): Json<DiffApiRequest>,
) -> Result<Json<DiffResult>, (StatusCode, String)> {
    // 1. 路径拼接 + 穿越检查（复用 cfx 模式）
    // 2. fs::metadata 大小检查（512KB）
    // 3. 读取文件，UTF-8 失败 + 未 force_text → 返回 Binary
    // 4. 构建 DiffRequest（含 algorithm + whitespace）
    // 5. spawn_blocking 执行 diff（CPU 密集型不阻塞事件循环）
    // 6. 返回 JSON
}
```

## 八、性能预算

基于 benchmark 数据和架构分析的预期性能：

| 场景 | 预期耗时 | 预期内存 | 依据 |
|------|---------|---------|------|
| 行级 diff，1KB 文件 | < 50μs | < 64KB | imara-diff p50=17.3μs（benchmark 实测） |
| 行级 diff，100KB 文件 | < 5ms | < 512KB | imara-diff p90=422μs（按线性外推） |
| 行级 diff，512KB 文件 | < 20ms | < 2MB | imara-diff avg=113μs@混合样本 |
| AST diff，1KB TS 文件 | < 10ms | < 256KB | tree-sitter parse ~1-5ms + 符号匹配 O(n) |
| AST diff，100KB TS 文件 | < 100ms | < 4MB | tree-sitter 1.6MB JSON ~1.2s，100KB 按比例 |
| Vue SFC，50KB | < 150ms | < 4MB | 3 段分别 AST diff |
| 相同文件（短路） | O(n) 但远快于 diff | ~0 额外 | `old == new` 全量比较，已在内存中，无误判 |

来源：
- imara-diff benchmark：用户提供的 `metrics_imara.jsonl` 实测数据
- tree-sitter 大文件：[GitHub Issue #1277](https://github.com/tree-sitter/tree-sitter/issues/1277) — 1.6MB JSON 解析 ~1.2s
- tree-sitter 内存：[Cosine blog](https://cosine.sh/blog/tree-sitter-memory-leak) — Parser 实例需要正确释放

## 九、二进制体积影响评估

当前 XunYu release profile：`opt-level = "z"` + `lto = "fat"` + `strip = "symbols"`

| 依赖 | 预估增量 | 说明 |
|------|---------|------|
| imara-diff | ~30-50KB | 纯 Rust，算法代码量小 |
| tree-sitter (runtime) | ~100-150KB | C 库，但 runtime 本身不大 |
| 每个语言 grammar | ~200KB-800KB | C 生成代码，这是主要体积来源 |
| 5 个语言 grammar 合计 | ~1.5-3MB | JS + TS + CSS + Rust + HTML |

**总增量预估：~2-3.5MB**（feature 关闭时为 0）

如果体积敏感，可进一步裁剪：
- 只保留 JS/TS + CSS（去掉 Rust/HTML grammar，省 ~800KB）
- 或将 diff feature 排除在默认 feature 之外

## 十、实施顺序

```
Phase 1: 基础设施（~2 个文件）
  ├── Cargo.toml 添加依赖和 feature gate
  └── src/diff/types.rs 定义所有类型

Phase 2: 行级引擎（~2 个文件）
  ├── src/diff/line.rs（imara-diff 封装）
  └── src/diff/mod.rs（入口 + Line 路由）

Phase 3: AST 引擎（~3 个文件）
  ├── src/diff/lang.rs（语言配置 + Query）
  ├── src/diff/ast.rs（符号提取 + 匹配）
  └── src/diff/vue.rs（SFC 拆分）

Phase 4: CLI 集成（~3 个文件修改）
  ├── src/commands/diff.rs（新增）
  ├── src/cli.rs（注册子命令）
  └── src/commands/mod.rs（dispatch）

Phase 5: Dashboard 集成（~2 个文件修改）
  ├── src/commands/dashboard/mod.rs（路由）
  └── src/commands/dashboard/handlers.rs（handler）

Phase 6: 测试
  └── tests/diff_*.rs
```

## 十一、风险与缓解

| 风险 | 概率 | 缓解 |
|------|------|------|
| tree-sitter grammar 编译慢（首次 5-10 分钟） | 确定 | CI 缓存 target/；文档说明 |
| tree-sitter 对畸形文件 panic/hang | 低 | `std::panic::catch_unwind` 包裹 parse，超时 fallback |
| imara-diff API 变更（0.x 版本） | 低 | `Cargo.toml` 锁定精确版本 |
| 二进制体积膨胀 | 确定 | feature gate 隔离；默认不启用 |
| Vue SFC 嵌套标签误匹配 | 中 | 正则匹配行首 `<tag`，非贪婪；测试覆盖边界情况 |
| 非 UTF-8 文件导致 read_to_string panic | 中 | 读取层捕获 `Err`，未指定 `--text` 时返回 Binary；指定时用 `from_utf8_lossy` |
| 空白预处理产生额外内存拷贝 | 低 | 使用 `Cow<str>`，仅在需要规范化时分配；512KB 上限控制最大开销 |
| SCSS/Less 文件误走 CSS AST diff | 已消除 | lang.rs 明确将 `.scss/.less` 路由到行级 diff |

## 十二、未来扩展（低优先级，不在本期实施范围）

以下能力参考主流 diff 工具设计，作为后续迭代方向记录。

### 12.1 目录对比与过滤器

参考 [WinMerge 过滤器设计](https://manual.winmerge.org/en/Filters.html)、
[Meld 目录比较](https://wiki.gnome.org/Apps/Meld)。

- `xun diff-dir <DIR_A> <DIR_B>` — 递归扫描两个目录，输出文件级增删改列表
- `--include` / `--exclude` glob 规则（文件/目录级过滤）
- 行级过滤器：正则匹配的行视为"噪声"，不计入 diff（如注释行、生成代码标记）
- Dashboard 侧：目录树对比视图，点击文件进入文件级 diff

### 12.2 Dashboard 行内/词级高亮

参考 [WinMerge 字符级差异高亮](https://manual.winmerge.org/Compare_files.html)、
[Beyond Compare 多视图](https://www.scootersoftware.com/home/multifaceted)。

- 在 Modified 行内，进一步标记词/字符级变更区间
- 实现方式：对 add/remove 行对做二次 diff（imara-diff 按字符 tokenize）
- 输出：`DiffLine` 增加 `inline_changes: Option<Vec<InlineSpan>>` 字段
- 前端用 `<mark>` 标签高亮变更区间

### 12.3 三路合并（3-way merge）

参考 [KDiff3 三路合并](https://kdiff3.sourceforge.net/)。

- `xun diff3 <BASE> <OURS> <THEIRS>` — 三路比较
- 冲突检测与标记
- 优先级低，仅在 XunYu 需要版本管理辅助时考虑

### 12.4 更多语言 grammar

按需引入，每个 grammar 约 200KB-800KB 体积增量：
- `tree-sitter-c` / `tree-sitter-cpp` — C/C++ AST diff
- `tree-sitter-python` — Python AST diff
- `tree-sitter-go` — Go AST diff
- 可考虑动态加载 grammar（`.so`/`.dll`）避免静态链接体积
