//! AST 符号级 diff（tree-sitter）
//!
//! 三阶段流水线：
//! 1. 符号提取（tree-sitter Query）
//! 2. 符号匹配（hash 精确 → 同名 → Added/Removed）
//! 3. 输出组装（Modified 符号内行级 diff）
//!
//! 性能特征：
//! - thread_local Parser 复用，避免重复创建
//! - SipHash（DefaultHasher）替代 SHA256
//! - 符号级匹配 O(n)，非 difftastic 的 O(n²)
//! - 降级条件：符号数 > 500 或行数 > 10000

use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};

use imara_diff::Algorithm;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, QueryCursor};

use super::lang;
use super::line;
use super::types::*;

// ── 常量 ──────────────────────────────────────────────────────────────────────

const MAX_SYMBOLS: usize = 500;
const MAX_LINES: usize = 10_000;

// ── 内部符号结构 ──────────────────────────────────────────────────────────────

struct Symbol {
    name: String,
    kind: String,
    start_line: usize,
    end_line: usize,
    hash: u64,
    source: String,
}

// ── SipHash ──────────────────────────────────────────────────────────────────

#[inline]
fn sip_hash(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

// ── thread_local Parser 复用 ─────────────────────────────────────────────────

thread_local! {
    static PARSER: RefCell<Parser> = RefCell::new(Parser::new());
}

// ── 符号提取 ──────────────────────────────────────────────────────────────────

fn extract_symbols(source: &str, ext: &str) -> Result<Vec<Symbol>, AstDiffError> {
    let language = lang::get_language(ext).ok_or(AstDiffError::UnsupportedLanguage)?;
    let query = lang::get_query(ext).ok_or(AstDiffError::UnsupportedLanguage)?;

    // 降级检查：行数
    let line_count = source.lines().count();
    if line_count > MAX_LINES {
        return Err(AstDiffError::TooManyLines(line_count));
    }

    // thread_local Parser 复用
    let tree = PARSER.with(|p| {
        let mut parser = p.borrow_mut();
        parser.set_language(&language).ok();
        parser.parse(source, None)
    });

    let tree = tree.ok_or(AstDiffError::ParseFailed)?;

    let source_lines: Vec<&str> = source.lines().collect();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());

    let mut symbols: Vec<Symbol> = Vec::new();

    while let Some(m) = matches.next() {
        let sym_cap = m
            .captures
            .iter()
            .find(|c| query.capture_names()[c.index as usize] == "symbol");
        let name_cap = m
            .captures
            .iter()
            .find(|c| query.capture_names()[c.index as usize] == "name");

        if let Some(sym) = sym_cap {
            let node = sym.node;
            let start_line = node.start_position().row + 1;
            let end_line = node.end_position().row + 1;

            // 符号名：优先 @name capture，否则用首行前 60 字符
            let name = if let Some(nc) = name_cap {
                nc.node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("<unknown>")
                    .to_string()
            } else {
                source_lines
                    .get(start_line - 1)
                    .unwrap_or(&"<unknown>")
                    .trim()
                    .chars()
                    .take(60)
                    .collect()
            };

            // 提取符号源码
            let sym_source: String = source_lines
                .get((start_line - 1)..end_line)
                .map(|ls: &[&str]| ls.join("\n"))
                .unwrap_or_default();

            let hash = sip_hash(&sym_source);

            symbols.push(Symbol {
                name,
                kind: node.kind().to_string(),
                start_line,
                end_line,
                hash,
                source: sym_source,
            });
        }
    }

    // 去重：tree-sitter query 有时对同一节点匹配多次
    symbols.dedup_by(|a, b| a.start_line == b.start_line && a.end_line == b.end_line);

    // 降级检查：符号数
    if symbols.len() > MAX_SYMBOLS {
        return Err(AstDiffError::TooManySymbols(symbols.len()));
    }

    Ok(symbols)
}

// ── 符号匹配（O(n) 贪心） ────────────────────────────────────────────────────

fn match_symbols(
    old_syms: &[Symbol],
    new_syms: &[Symbol],
    algo: Algorithm,
    context: usize,
    include_add_remove_lines: bool,
) -> (Vec<Hunk>, DiffStats) {
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut stats = DiffStats {
        added: 0,
        removed: 0,
        modified: 0,
        unchanged: 0,
        unit: StatsUnit::Symbol,
    };
    let mut new_consumed = vec![false; new_syms.len()];

    for old_sym in old_syms {
        // Pass 1：hash 精确匹配 → 视为未变更（短路，跳过行级 diff）
        let hash_match = new_syms
            .iter()
            .enumerate()
            .find(|(i, n)| !new_consumed[*i] && n.hash == old_sym.hash);

        if let Some((idx, _)) = hash_match {
            new_consumed[idx] = true;
            stats.unchanged += 1;
            continue;
        }

        // Pass 2：同名匹配 → Modified（对符号内部做行级 diff）
        let name_match = new_syms
            .iter()
            .enumerate()
            .find(|(i, n)| !new_consumed[*i] && n.name == old_sym.name);

        if let Some((idx, new_sym)) = name_match {
            new_consumed[idx] = true;
            stats.modified += 1;

            let inner_hunks = line::diff_lines(&old_sym.source, &new_sym.source, algo, context);
            for mut h in inner_hunks {
                h.symbol = Some(old_sym.name.clone());
                h.symbol_type = Some(symbol_type_label(&old_sym.kind));
                h.kind = HunkKind::Modified;
                // 偏移行号回原始文件
                h.old_start += (old_sym.start_line as u32) - 1;
                h.new_start += (new_sym.start_line as u32) - 1;
                hunks.push(h);
            }
            continue;
        }

        // 未匹配 → Removed
        stats.removed += 1;
        hunks.push(make_meta_hunk(
            old_sym,
            HunkKind::Removed,
            include_add_remove_lines,
        ));
    }

    // 未被匹配的 new 符号 → Added
    for (i, new_sym) in new_syms.iter().enumerate() {
        if !new_consumed[i] {
            stats.added += 1;
            hunks.push(make_meta_hunk(
                new_sym,
                HunkKind::Added,
                include_add_remove_lines,
            ));
        }
    }

    (hunks, stats)
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────────

/// Added/Removed 符号只输出元信息 hunk（不输出全部源码，减少输出体积）
///
/// - Removed: old_start/old_count 指向旧文件位置，new 为 0（该段在新文件不存在）
/// - Added:   new_start/new_count 指向新文件位置，old 为 0（该段在旧文件不存在）
fn make_meta_hunk(sym: &Symbol, kind: HunkKind, include_lines: bool) -> Hunk {
    let line_count = (sym.end_line - sym.start_line + 1) as u32;
    let (old_start, old_count, new_start, new_count) = match kind {
        HunkKind::Removed => (sym.start_line as u32, line_count, 0, 0),
        HunkKind::Added => (0, 0, sym.start_line as u32, line_count),
        _ => (
            sym.start_line as u32,
            line_count,
            sym.start_line as u32,
            line_count,
        ),
    };
    let lines = if include_lines {
        let tag = match kind {
            HunkKind::Removed => LineTag::Remove,
            HunkKind::Added => LineTag::Add,
            _ => LineTag::Context,
        };
        if tag == LineTag::Context {
            Vec::new()
        } else {
            sym.source
                .lines()
                .map(|l| DiffLine {
                    tag,
                    content: l.to_string(),
                })
                .collect()
        }
    } else {
        Vec::new()
    };
    Hunk {
        kind,
        symbol: Some(sym.name.clone()),
        symbol_type: Some(symbol_type_label(&sym.kind)),
        section: None,
        old_start,
        old_count,
        new_start,
        new_count,
        lines,
    }
}

/// tree-sitter node kind → 人类可读标签
fn symbol_type_label(kind: &str) -> String {
    match kind {
        "function_declaration" | "function_item" => "function",
        "class_declaration" => "class",
        "interface_declaration" => "interface",
        "type_alias_declaration" | "type_item" => "type",
        "lexical_declaration" | "variable_declaration" => "variable",
        "export_statement" => "export",
        "struct_item" => "struct",
        "enum_item" => "enum",
        "trait_item" => "trait",
        "impl_item" => "impl",
        "const_item" => "const",
        "rule_set" => "rule",
        "at_rule" => "at-rule",
        "element" => "element",
        other => other,
    }
    .to_string()
}

// ── 公开 API ──────────────────────────────────────────────────────────────────

/// AST 符号级 diff 入口
///
/// 返回 `Ok((hunks, stats))` 或 `Err` 触发 fallback 到行级 diff
pub fn diff_ast(
    old: &str,
    new: &str,
    ext: &str,
    algo: Algorithm,
    context: usize,
) -> Result<(Vec<Hunk>, DiffStats), AstDiffError> {
    diff_ast_internal(old, new, ext, algo, context, false)
}

/// AST 符号级 diff（Added/Removed 附带行内容，用于 Vue SFC 等场景）
pub fn diff_ast_with_add_remove_lines(
    old: &str,
    new: &str,
    ext: &str,
    algo: Algorithm,
    context: usize,
) -> Result<(Vec<Hunk>, DiffStats), AstDiffError> {
    diff_ast_internal(old, new, ext, algo, context, true)
}

fn diff_ast_internal(
    old: &str,
    new: &str,
    ext: &str,
    algo: Algorithm,
    context: usize,
    include_add_remove_lines: bool,
) -> Result<(Vec<Hunk>, DiffStats), AstDiffError> {
    let old_syms = extract_symbols(old, ext)?;
    let new_syms = extract_symbols(new, ext)?;

    let (hunks, stats) = match_symbols(
        &old_syms,
        &new_syms,
        algo,
        context,
        include_add_remove_lines,
    );
    Ok((hunks, stats))
}
