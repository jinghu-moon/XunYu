//! Diff 模块所有公开类型定义
//!
//! CLI 和 Dashboard 共用同一套类型，确保输出一致性。

use serde::{Deserialize, Serialize};

// ── 输入类型 ──────────────────────────────────────────────────────────────────

/// Diff 请求（CLI 和 Dashboard 共用）
pub struct DiffRequest<'a> {
    /// 旧文件内容（调用方负责读取）
    pub old: &'a str,
    /// 新文件内容
    pub new: &'a str,
    /// 文件扩展名（小写，无点号，如 "rs"、"vue"）
    pub ext: &'a str,
    /// diff 模式：auto | line | ast
    pub mode: DiffMode,
    /// 行级 diff 算法选择
    pub algorithm: DiffAlgorithm,
    /// 上下文行数，默认 3
    pub context: usize,
    /// 空白处理选项
    pub whitespace: WhitespaceOpt,
    /// 强制按文本处理二进制文件（类似 GNU diff --text / -a）
    pub force_text: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffMode {
    #[default]
    Auto,
    Line,
    Ast,
}

/// 行级 diff 算法（对齐 Git diff-algorithm 选项）
///
/// 注意：imara-diff 仅原生支持 Myers 和 Histogram。
/// Patience → Histogram（近似映射，histogram 是 patience 的超集变体）
/// Minimal → Myers（近似映射，Myers 已内置最小编辑距离语义）
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffAlgorithm {
    Myers,
    Minimal,
    Patience,
    #[default]
    Histogram,
}

/// 空白 / 行尾处理选项
/// 参考：https://www.gnu.org/s/diffutils/manual/html_node/White-Space.html
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WhitespaceOpt {
    /// 忽略行内空白量变化（类似 GNU diff -b）
    #[serde(default)]
    pub ignore_space_change: bool,
    /// 忽略所有空白（类似 GNU diff -w）
    #[serde(default)]
    pub ignore_all_space: bool,
    /// 忽略空行差异（类似 GNU diff -B）
    #[serde(default)]
    pub ignore_blank_lines: bool,
    /// 剥离行尾 CR，消除 CRLF/LF 噪声（Windows 环境高频需求）
    #[serde(default)]
    pub strip_trailing_cr: bool,
}

// ── 输出类型 ──────────────────────────────────────────────────────────────────

/// 统一 diff 输出
#[derive(Debug, Serialize)]
pub struct DiffResult {
    /// 结果类型
    pub kind: DiffResultKind,
    /// 统计信息
    pub stats: DiffStats,
    /// diff hunks
    pub hunks: Vec<Hunk>,
    /// 实际使用的算法（当近似映射时与请求不同）
    pub actual_algorithm: DiffAlgorithm,
    /// 是否在忽略规则下判定为相同（原始内容可能不同）
    pub identical_with_filters: bool,
}

#[derive(Debug, Serialize)]
pub struct DiffStats {
    pub added: u32,
    pub removed: u32,
    pub modified: u32,
    pub unchanged: u32,
    /// 统计粒度：Line（行级模式 / Vue SFC）| Symbol（AST 模式符号级）
    ///
    /// 不变量（前端可依赖）：
    ///   DiffResultKind::Ast       → StatsUnit::Symbol
    ///   DiffResultKind::Line      → StatsUnit::Line（含 Vue SFC 分段行级 diff）
    ///   DiffResultKind::Identical → StatsUnit::Line（stats 全零）
    ///   DiffResultKind::Binary    → StatsUnit::Line（stats 全零）
    pub unit: StatsUnit,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StatsUnit {
    Line,
    Symbol,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffResultKind {
    Identical,
    Line,
    Ast,
    Binary,
}

/// 行级 hunk
#[derive(Debug, Serialize)]
pub struct Hunk {
    /// hunk 类型
    pub kind: HunkKind,
    /// AST 模式下的符号名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// 符号类型：function / struct / ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_type: Option<String>,
    /// Vue SFC 段名：template / script / style
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// 旧文件起始行号（1-indexed）
    pub old_start: u32,
    pub old_count: u32,
    /// 新文件起始行号（1-indexed）
    pub new_start: u32,
    pub new_count: u32,
    /// 行内容
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Serialize)]
pub struct DiffLine {
    /// 行标记：context | add | remove
    pub tag: LineTag,
    /// 行内容（不含换行符）
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineTag {
    Context,
    Add,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HunkKind {
    Added,
    Removed,
    Modified,
}

// ── 错误类型 ──────────────────────────────────────────────────────────────────

/// AST diff 降级错误（由 mod.rs fallback 到行级 diff）
#[derive(Debug)]
pub enum AstDiffError {
    /// tree-sitter 解析失败
    ParseFailed,
    /// 符号数超过阈值（>500）
    TooManySymbols(usize),
    /// 行数超过阈值（>10000）
    TooManyLines(usize),
    /// 不支持的语言
    UnsupportedLanguage,
}

impl std::fmt::Display for AstDiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseFailed => write!(f, "tree-sitter parse failed"),
            Self::TooManySymbols(n) => write!(f, "too many symbols: {n} > 500"),
            Self::TooManyLines(n) => write!(f, "too many lines: {n} > 10000"),
            Self::UnsupportedLanguage => write!(f, "unsupported language for AST diff"),
        }
    }
}

impl std::error::Error for AstDiffError {}
