//! Diff 模块公开 API + 路由调度
//!
//! 纯计算模块，零 IO 依赖（文件读取由调用方负责）。
//! CLI 和 Dashboard 共享同一套核心逻辑。

pub mod ast;
pub mod lang;
pub mod line;
pub mod types;
pub mod vue;

use std::borrow::Cow;

use types::*;

// ── 常量 ──────────────────────────────────────────────────────────────────────

/// 二进制检测：扫描前 8KB 是否含 NUL 字节
const BINARY_SCAN_LEN: usize = 8 * 1024;

// ── 二进制检测 ────────────────────────────────────────────────────────────────

/// 扫描前 8KB 是否含 NUL 字节（\0）
#[inline]
fn is_binary(content: &str) -> bool {
    let scan = &content.as_bytes()[..content.len().min(BINARY_SCAN_LEN)];
    scan.contains(&0)
}

// ── 空白预处理 ────────────────────────────────────────────────────────────────

/// 根据 WhitespaceOpt 对文本做规范化，返回 Cow（未修改时零拷贝）
fn preprocess_whitespace<'a>(text: &'a str, opts: &WhitespaceOpt) -> Cow<'a, str> {
    if !opts.strip_trailing_cr
        && !opts.ignore_all_space
        && !opts.ignore_space_change
        && !opts.ignore_blank_lines
    {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let ends_with_newline = text.ends_with('\n') || text.ends_with("\r\n");
    let mut first = true;

    // 用 split('\n') 而非 lines()，保留行尾 \r，使 strip_trailing_cr 有实际效果。
    // 先剥离末尾一个换行序列再 split，避免产生多余空尾元素（EOF 换行由 ends_with_newline 恢复）。
    let base = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);

    for raw_line in base.split('\n') {
        let mut l = Cow::Borrowed(raw_line);

        // 剥离行尾 CR
        if opts.strip_trailing_cr
            && let Some(stripped) = l.strip_suffix('\r')
        {
            l = Cow::Owned(stripped.to_string());
        }

        // 忽略所有空白（优先级高于 ignore_space_change）
        if opts.ignore_all_space {
            let no_ws: String = l.chars().filter(|c| !c.is_whitespace()).collect();
            l = Cow::Owned(no_ws);
        } else if opts.ignore_space_change {
            // 连续空白压缩为单个空格
            let compressed = compress_whitespace(&l);
            l = Cow::Owned(compressed);
        }

        // 忽略空行
        if opts.ignore_blank_lines && l.trim().is_empty() {
            continue;
        }

        // 用 \n 连接各行，但仅当原始文件末尾有换行时在最后追加 \n
        if !first {
            result.push('\n');
        }
        first = false;
        result.push_str(&l);
    }

    // 保留原始文件的 EOF 换行特征（仅当有实际内容时才恢复，
    // 避免 ignore_blank_lines 过滤全部空行后仍残留一个 \n）
    if ends_with_newline && !first {
        result.push('\n');
    }

    Cow::Owned(result)
}

/// 连续空白压缩为单个空格
fn compress_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                result.push(' ');
                prev_ws = true;
            }
        } else {
            result.push(c);
            prev_ws = false;
        }
    }
    result
}

// ── 公开 API ──────────────────────────────────────────────────────────────────

/// Diff 入口：路由调度 + 二进制检测 + 空白预处理
pub fn diff(req: DiffRequest<'_>) -> DiffResult {
    let (actual_algorithm, imara_algo) = line::map_algorithm(req.algorithm);

    // 1. 二进制检测
    if !req.force_text && (is_binary(req.old) || is_binary(req.new)) {
        return DiffResult {
            kind: DiffResultKind::Binary,
            stats: DiffStats::zero(StatsUnit::Line),
            hunks: vec![],
            actual_algorithm,
            identical_with_filters: false,
        };
    }

    // 2. 空白预处理
    let has_ws_filters = req.whitespace.strip_trailing_cr
        || req.whitespace.ignore_all_space
        || req.whitespace.ignore_space_change
        || req.whitespace.ignore_blank_lines;

    let old = preprocess_whitespace(req.old, &req.whitespace);
    let new = preprocess_whitespace(req.new, &req.whitespace);

    // 3. 严格相等检测
    if *old == *new {
        return DiffResult {
            kind: DiffResultKind::Identical,
            stats: DiffStats::zero(StatsUnit::Line),
            hunks: vec![],
            actual_algorithm,
            identical_with_filters: has_ws_filters,
        };
    }

    // 4. 模式分派
    match req.mode {
        DiffMode::Line => diff_as_line(&old, &new, imara_algo, actual_algorithm, req.context),
        DiffMode::Ast => diff_as_ast_or_fallback(
            &old,
            &new,
            req.ext,
            imara_algo,
            actual_algorithm,
            req.context,
        ),
        DiffMode::Auto => diff_auto(
            &old,
            &new,
            req.ext,
            imara_algo,
            actual_algorithm,
            req.context,
        ),
    }
}

/// AST diff，失败时 fallback 到行级
fn diff_as_ast_or_fallback(
    old: &str,
    new: &str,
    ext: &str,
    algo: imara_diff::Algorithm,
    actual: DiffAlgorithm,
    context: usize,
) -> DiffResult {
    match ast::diff_ast(old, new, ext, algo, context) {
        Ok((hunks, stats)) => DiffResult {
            kind: DiffResultKind::Ast,
            stats,
            hunks,
            actual_algorithm: actual,
            identical_with_filters: false,
        },
        Err(_) => diff_as_line(old, new, algo, actual, context),
    }
}

/// Auto 模式：按扩展名自动选择 diff 策略
fn diff_auto(
    old: &str,
    new: &str,
    ext: &str,
    algo: imara_diff::Algorithm,
    actual: DiffAlgorithm,
    context: usize,
) -> DiffResult {
    match ext {
        "vue" => {
            let hunks = vue::diff_vue(old, new, algo, context);
            let stats = line_stats_from_hunks(&hunks);
            DiffResult {
                kind: DiffResultKind::Line,
                stats,
                hunks,
                actual_algorithm: actual,
                identical_with_filters: false,
            }
        }
        ext if lang::has_ast_support(ext) => {
            diff_as_ast_or_fallback(old, new, ext, algo, actual, context)
        }
        _ => diff_as_line(old, new, algo, actual, context),
    }
}

// ── 内部分派 ──────────────────────────────────────────────────────────────────

/// 纯行级 diff
fn diff_as_line(
    old: &str,
    new: &str,
    algo: imara_diff::Algorithm,
    actual: DiffAlgorithm,
    context: usize,
) -> DiffResult {
    let hunks = line::diff_lines(old, new, algo, context);
    let stats = line_stats_from_hunks(&hunks);
    DiffResult {
        kind: DiffResultKind::Line,
        stats,
        hunks,
        actual_algorithm: actual,
        identical_with_filters: false,
    }
}

// ── 统计辅助 ──────────────────────────────────────────────────────────────────

/// 从 hunks 中统计行级增删数
fn line_stats_from_hunks(hunks: &[Hunk]) -> DiffStats {
    let mut added: u32 = 0;
    let mut removed: u32 = 0;

    for h in hunks {
        for line in &h.lines {
            match line.tag {
                LineTag::Add => added += 1,
                LineTag::Remove => removed += 1,
                LineTag::Context => {}
            }
        }
    }

    DiffStats {
        added,
        removed,
        modified: 0,
        unchanged: 0,
        unit: StatsUnit::Line,
    }
}

impl DiffStats {
    /// 全零统计（用于 Identical / Binary）
    pub fn zero(unit: StatsUnit) -> Self {
        Self {
            added: 0,
            removed: 0,
            modified: 0,
            unchanged: 0,
            unit,
        }
    }
}
