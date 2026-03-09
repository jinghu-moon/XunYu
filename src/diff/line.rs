//! 行级 diff 引擎（imara-diff 封装）
//!
//! 性能特征：
//! - Histogram 算法（默认），比 Myers 快 10-100%
//! - InternedInput 零额外拷贝 token 化，pointer compression 减少内存
//! - postprocess_lines 启发式优化输出可读性

use imara_diff::{Algorithm, Diff, InternedInput};

use super::types::{DiffAlgorithm, DiffLine, Hunk, HunkKind, LineTag};

// ── 算法映射 ──────────────────────────────────────────────────────────────────

/// 将 DiffAlgorithm 映射到 imara-diff Algorithm，返回 (实际算法枚举, imara Algorithm)
///
/// imara-diff 仅支持 Myers 和 Histogram：
/// - Patience → Histogram（近似：histogram 是 patience 的超集变体）
/// - Minimal → Myers（近似：Myers 已内置最小编辑距离语义）
pub fn map_algorithm(requested: DiffAlgorithm) -> (DiffAlgorithm, Algorithm) {
    match requested {
        DiffAlgorithm::Histogram => (DiffAlgorithm::Histogram, Algorithm::Histogram),
        DiffAlgorithm::Myers => (DiffAlgorithm::Myers, Algorithm::Myers),
        DiffAlgorithm::Patience => (DiffAlgorithm::Histogram, Algorithm::Histogram),
        DiffAlgorithm::Minimal => (DiffAlgorithm::Myers, Algorithm::Myers),
    }
}

// ── 核心行级 diff ─────────────────────────────────────────────────────────────

/// 计算两段文本的行级 diff，返回带上下文的 hunks
///
/// - `old` / `new`：已经过空白预处理的文本
/// - `algorithm`：imara-diff Algorithm（已映射）
/// - `context`：上下文行数（默认 3）
pub fn diff_lines(old: &str, new: &str, algorithm: Algorithm, context: usize) -> Vec<Hunk> {
    let input = InternedInput::new(old, new);
    let mut diff = Diff::compute(algorithm, &input);
    diff.postprocess_lines(&input);

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    // 收集原始 hunks（无上下文）
    let raw_hunks: Vec<_> = diff.hunks().collect();
    if raw_hunks.is_empty() {
        return vec![];
    }

    build_hunks_with_context(&raw_hunks, &old_lines, &new_lines, context)
}

// ── Hunk 构建（带上下文行合并） ───────────────────────────────────────────────

/// 将 imara-diff 原始 hunks 转换为带上下文行的输出 hunks
///
/// 合并规则：相邻 hunk 间距 ≤ 2×context 时合并（与 unified diff 行为一致）
fn build_hunks_with_context(
    raw_hunks: &[imara_diff::Hunk],
    old_lines: &[&str],
    new_lines: &[&str],
    context: usize,
) -> Vec<Hunk> {
    let old_len = old_lines.len() as u32;
    let new_len = new_lines.len() as u32;
    let ctx = context as u32;

    // 将原始 hunks 分组：间距 ≤ 2×context 的合并为一组
    let groups = group_hunks(raw_hunks, ctx, old_len);
    let mut result = Vec::with_capacity(groups.len());

    for group in &groups {
        let first = &group[0];
        let last = &group[group.len() - 1];

        // 计算本组 hunk 在 old/new 中的范围（含上下文）
        let old_start = first.before.start.saturating_sub(ctx);
        let old_end = (last.before.end + ctx).min(old_len);
        let new_start = first.after.start.saturating_sub(ctx);
        let new_end = (last.after.end + ctx).min(new_len);

        let mut lines = Vec::new();
        let mut old_pos = old_start;
        let mut new_pos = new_start;

        for h in group {
            // 上下文行（hunk 前）
            while old_pos < h.before.start {
                push_context_line(&mut lines, old_lines, new_lines, old_pos, new_pos);
                old_pos += 1;
                new_pos += 1;
            }
            // 删除行
            for i in h.before.clone() {
                lines.push(DiffLine {
                    tag: LineTag::Remove,
                    content: old_lines.get(i as usize).unwrap_or(&"").to_string(),
                });
            }
            old_pos = h.before.end;
            // 新增行
            for i in h.after.clone() {
                lines.push(DiffLine {
                    tag: LineTag::Add,
                    content: new_lines.get(i as usize).unwrap_or(&"").to_string(),
                });
            }
            new_pos = h.after.end;
        }

        // 尾部上下文行
        while old_pos < old_end {
            push_context_line(&mut lines, old_lines, new_lines, old_pos, new_pos);
            old_pos += 1;
            new_pos += 1;
        }

        result.push(Hunk {
            kind: HunkKind::Modified,
            symbol: None,
            symbol_type: None,
            section: None,
            old_start: old_start + 1, // 1-indexed
            old_count: old_end - old_start,
            new_start: new_start + 1,
            new_count: new_end - new_start,
            lines,
        });
    }

    result
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 将原始 hunks 按间距分组：相邻 hunk 在 old 序列中间距 ≤ 2×context 时合并
fn group_hunks(raw: &[imara_diff::Hunk], ctx: u32, _old_len: u32) -> Vec<Vec<imara_diff::Hunk>> {
    let mut groups: Vec<Vec<imara_diff::Hunk>> = Vec::new();
    let threshold = 2 * ctx;

    for h in raw {
        let should_merge = groups.last().is_some_and(|g| {
            let prev = g.last().unwrap();
            h.before.start.saturating_sub(prev.before.end) <= threshold
        });

        if should_merge {
            groups.last_mut().unwrap().push(h.clone());
        } else {
            groups.push(vec![h.clone()]);
        }
    }

    groups
}

/// 推入一行上下文
#[inline]
fn push_context_line(
    lines: &mut Vec<DiffLine>,
    old_lines: &[&str],
    new_lines: &[&str],
    old_pos: u32,
    new_pos: u32,
) {
    let old_line = old_lines.get(old_pos as usize);
    let new_line = new_lines.get(new_pos as usize);
    if let (Some(a), Some(b)) = (old_line, new_line) {
        debug_assert_eq!(
            a,
            b,
            "context line mismatch at old={}, new={}",
            old_pos + 1,
            new_pos + 1
        );
    }
    let content = old_line.or(new_line).unwrap_or(&"").to_string();
    lines.push(DiffLine {
        tag: LineTag::Context,
        content,
    });
}
