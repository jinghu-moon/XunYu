//! Vue SFC 拆分 + 分段 diff
//!
//! 改进 cfx 参考实现：
//! - 正则匹配替代简陋字符串匹配
//! - 识别 `lang` 属性，正确分派语言（如 `<script lang="ts">`）
//! - 支持多个同名标签（如两个 `<style>`）

use std::sync::LazyLock;

use imara_diff::Algorithm;
use regex::Regex;

use super::ast;
use super::line;
use super::types::*;

// ── 正则（LazyLock，编译一次） ────────────────────────────────────────────────

/// 匹配 SFC 开标签起始行，捕获 tag 名和后续文本（可跨多行继续解析）
static RE_OPEN_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?m)^\s*<(template|script|style)\b(.*)"#).unwrap());

/// 从属性字符串中提取 lang="xxx"
static RE_LANG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"lang=["'](\w+)["']"#).unwrap());

// ── SFC 段结构 ────────────────────────────────────────────────────────────────

struct SfcSection {
    tag: String,       // template / script / style
    lang: String,      // html / js / ts / css / scss / less
    content: String,   // 标签内的内容
    start_line: usize, // 内容起始行号（1-indexed，相对原始 SFC）
}

// ── SFC 拆分 ──────────────────────────────────────────────────────────────────

/// 拆分 Vue SFC 为多个段，识别 lang 属性
fn split_sfc(source: &str) -> Vec<SfcSection> {
    let lines: Vec<&str> = source.lines().collect();
    let mut sections = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if let Some(caps) = RE_OPEN_START.captures(line) {
            let tag = caps.get(1).unwrap().as_str().to_string();
            let attrs_match = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let attrs_start = caps.get(2).map(|m| m.start()).unwrap_or(line.len());

            // 解析开标签，允许属性跨多行，直到遇到第一个 '>'
            let mut open_end_line = i;
            let mut open_end_col: Option<usize> = None;
            let mut attrs_buf = String::new();

            if let Some(pos) = attrs_match.find('>') {
                open_end_col = Some(attrs_start + pos);
                attrs_buf.push_str(&attrs_match[..pos]);
            } else {
                attrs_buf.push_str(attrs_match);
                while open_end_col.is_none() && open_end_line + 1 < lines.len() {
                    open_end_line += 1;
                    let l = lines[open_end_line];
                    if let Some(pos) = l.find('>') {
                        open_end_col = Some(pos);
                        attrs_buf.push('\n');
                        attrs_buf.push_str(&l[..pos]);
                    } else {
                        attrs_buf.push('\n');
                        attrs_buf.push_str(l);
                    }
                }
            }

            let Some(open_end_col) = open_end_col else {
                i += 1;
                continue;
            };

            // 提取 lang 属性
            let lang = if let Some(lc) = RE_LANG.captures(attrs_buf.as_str()) {
                lc.get(1).unwrap().as_str().to_string()
            } else {
                default_lang(&tag)
            };

            // 查找对应闭标签
            let close_tag = format!("</{}>", tag);
            let after_gt = if open_end_col < lines[open_end_line].len() {
                &lines[open_end_line][open_end_col + 1..]
            } else {
                ""
            };

            // 单行标签：开闭标签在同一行（如 <script>...</script>）
            if let Some(close_pos) = after_gt.find(&close_tag) {
                let content = after_gt[..close_pos].to_string();
                if !content.trim().is_empty() {
                    sections.push(SfcSection {
                        tag,
                        lang,
                        content,
                        start_line: open_end_line + 1, // 1-indexed，内容与开标签同行
                    });
                }
                i = open_end_line + 1;
                continue;
            }

            // 多行标签：闭标签在后续行或行内
            let mut content_lines: Vec<String> = Vec::new();
            let mut start_line_idx: Option<usize> = None;
            let mut end = open_end_line;
            let mut found_close = false;

            loop {
                if end >= lines.len() {
                    break;
                }
                let line_part = if end == open_end_line {
                    after_gt
                } else {
                    lines[end]
                };
                if end == open_end_line && line_part.trim().is_empty() {
                    end += 1;
                    continue;
                }
                if start_line_idx.is_none() {
                    start_line_idx = Some(end);
                }
                if let Some(pos) = line_part.find(&close_tag) {
                    let before = &line_part[..pos];
                    if !before.is_empty() {
                        content_lines.push(before.to_string());
                    }
                    found_close = true;
                    break;
                } else {
                    content_lines.push(line_part.to_string());
                    end += 1;
                }
            }

            if !found_close {
                i = open_end_line + 1;
                continue;
            }

            let content = content_lines.join("\n");
            if !content.trim().is_empty() {
                let start_line = start_line_idx.unwrap_or(open_end_line) + 1; // 1-indexed
                sections.push(SfcSection {
                    tag,
                    lang,
                    content,
                    start_line,
                });
            }

            i = end + 1;
        } else {
            i += 1;
        }
    }

    sections
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 标签默认语言
fn default_lang(tag: &str) -> String {
    match tag {
        "template" => "html",
        "script" => "js",
        "style" => "css",
        _ => "text",
    }
    .to_string()
}

/// 对单个 SFC 段执行 diff，根据 lang 选择 AST 或行级
fn diff_section(old: &SfcSection, new: &SfcSection, algo: Algorithm, context: usize) -> Vec<Hunk> {
    let ext = &old.lang;

    // scss / less 等无 grammar 的走行级
    let hunks =
        match ast::diff_ast_with_add_remove_lines(&old.content, &new.content, ext, algo, context) {
            Ok((h, _)) => h,
            Err(_) => line::diff_lines(&old.content, &new.content, algo, context),
        };

    // 偏移行号回原始 SFC + 标注 section
    hunks
        .into_iter()
        .map(|mut h| {
            h.old_start += (old.start_line as u32) - 1;
            h.new_start += (new.start_line as u32) - 1;
            h.section = Some(old.tag.clone());
            h
        })
        .collect()
}

// ── 公开 API ──────────────────────────────────────────────────────────────────

/// Vue SFC diff 入口
///
/// 拆分 old/new 为段，按 tag 名配对（consumed 标记防止重复匹配），逐段 diff。
/// 未配对的旧段输出为 Removed，未配对的新段输出为 Added。
pub fn diff_vue(old: &str, new: &str, algo: Algorithm, context: usize) -> Vec<Hunk> {
    let old_sections = split_sfc(old);
    let new_sections = split_sfc(new);
    let mut all_hunks = Vec::new();
    let mut new_consumed = vec![false; new_sections.len()];

    for old_sec in &old_sections {
        // 按 tag 名 + lang 配对，consumed 标记已消费的新段
        let paired = new_sections
            .iter()
            .enumerate()
            .find(|(i, n)| !new_consumed[*i] && n.tag == old_sec.tag && n.lang == old_sec.lang);

        if let Some((idx, new_sec)) = paired {
            new_consumed[idx] = true;
            if old_sec.content != new_sec.content {
                let hunks = diff_section(old_sec, new_sec, algo, context);
                all_hunks.extend(hunks);
            }
        } else {
            // old 段在 new 中不存在 → 整段删除
            all_hunks.push(make_removed_section_hunk(old_sec));
        }
    }

    // 未被匹配的 new 段 → 整段新增
    for (i, new_sec) in new_sections.iter().enumerate() {
        if !new_consumed[i] {
            all_hunks.push(make_added_section_hunk(new_sec));
        }
    }

    all_hunks
}

/// 构建整段删除的 hunk（Removed）
fn make_removed_section_hunk(sec: &SfcSection) -> Hunk {
    let line_count = sec.content.lines().count() as u32;
    let lines: Vec<DiffLine> = sec
        .content
        .lines()
        .map(|l| DiffLine {
            tag: LineTag::Remove,
            content: l.to_string(),
        })
        .collect();
    Hunk {
        kind: HunkKind::Removed,
        symbol: None,
        symbol_type: None,
        section: Some(sec.tag.clone()),
        old_start: sec.start_line as u32,
        old_count: line_count,
        new_start: 0,
        new_count: 0,
        lines,
    }
}

/// 构建整段新增的 hunk（Added）
fn make_added_section_hunk(sec: &SfcSection) -> Hunk {
    let line_count = sec.content.lines().count() as u32;
    let lines: Vec<DiffLine> = sec
        .content
        .lines()
        .map(|l| DiffLine {
            tag: LineTag::Add,
            content: l.to_string(),
        })
        .collect();
    Hunk {
        kind: HunkKind::Added,
        symbol: None,
        symbol_type: None,
        section: Some(sec.tag.clone()),
        old_start: 0,
        old_count: 0,
        new_start: sec.start_line as u32,
        new_count: line_count,
        lines,
    }
}
