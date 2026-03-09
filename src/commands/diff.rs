use std::fs;
use std::path::Path;

use argh::FromArgs;
use console::Style;

use crate::diff;
use crate::diff::types::*;
use crate::output::{CliError, CliResult};

// ── CLI 子命令定义 ──────────────────────────────────────────────────────────

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

// ── 参数解析辅助 ────────────────────────────────────────────────────────────

/// 解析人类可读的大小字符串（如 "512K"、"1M"、"2048"）为字节数
fn parse_max_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty size string".into());
    }

    let (num_part, multiplier) = if s.ends_with('K') || s.ends_with('k') {
        (&s[..s.len() - 1], 1024u64)
    } else if s.ends_with('M') || s.ends_with('m') {
        (&s[..s.len() - 1], 1024 * 1024)
    } else if s.ends_with('G') || s.ends_with('g') {
        (&s[..s.len() - 1], 1024 * 1024 * 1024)
    } else {
        (s, 1u64)
    };

    num_part
        .trim()
        .parse::<u64>()
        .map(|n| n * multiplier)
        .map_err(|e| format!("invalid size '{s}': {e}"))
}

fn parse_mode(s: &str) -> Result<DiffMode, String> {
    match s {
        "auto" => Ok(DiffMode::Auto),
        "line" => Ok(DiffMode::Line),
        "ast" => Ok(DiffMode::Ast),
        other => Err(format!(
            "invalid mode '{}', expected: auto | line | ast",
            other
        )),
    }
}

fn parse_algorithm(s: &str) -> Result<DiffAlgorithm, String> {
    match s {
        "histogram" => Ok(DiffAlgorithm::Histogram),
        "myers" => Ok(DiffAlgorithm::Myers),
        "minimal" => Ok(DiffAlgorithm::Minimal),
        "patience" => Ok(DiffAlgorithm::Patience),
        other => Err(format!(
            "invalid algorithm '{}', expected: histogram | myers | minimal | patience",
            other
        )),
    }
}

/// 从文件路径提取小写扩展名（无点号）
fn extract_ext(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

// ── 主入口 ──────────────────────────────────────────────────────────────────

pub(crate) fn cmd_diff(args: DiffCmd) -> CliResult {
    // 1. 解析参数
    let max_bytes = parse_max_size(&args.max_size)
        .map_err(|e| CliError::with_details(1, e, &["Fix: Use format like 512K, 1M, 2G"]))?;
    let mode = parse_mode(&args.mode)
        .map_err(|e| CliError::with_details(1, e, &["Fix: Use one of: auto | line | ast"]))?;
    let algorithm = parse_algorithm(&args.diff_algorithm).map_err(|e| {
        CliError::with_details(
            1,
            e,
            &["Fix: Use one of: histogram | myers | minimal | patience"],
        )
    })?;

    let old_path = Path::new(&args.old);
    let new_path = Path::new(&args.new);

    // 2. 文件存在性检查
    if !old_path.is_file() {
        return Err(CliError::new(
            1,
            format!("'{}' is not a file or does not exist.", args.old),
        ));
    }
    if !new_path.is_file() {
        return Err(CliError::new(
            1,
            format!("'{}' is not a file or does not exist.", args.new),
        ));
    }

    // 3. 文件大小检查
    check_file_size(old_path, max_bytes, &args.old)?;
    check_file_size(new_path, max_bytes, &args.new)?;

    // 4. 读取文件
    let old_bytes = fs::read(old_path)
        .map_err(|e| CliError::new(1, format!("failed to read '{}': {e}", args.old)))?;
    let new_bytes = fs::read(new_path)
        .map_err(|e| CliError::new(1, format!("failed to read '{}': {e}", args.new)))?;

    // 4.5 UTF-8 验证：非 --text 模式下，非 UTF-8 文件视为 Binary
    if !args.text
        && (std::str::from_utf8(&old_bytes).is_err() || std::str::from_utf8(&new_bytes).is_err())
    {
        let (actual_algorithm, _) = diff::line::map_algorithm(algorithm);
        let result = diff::types::DiffResult {
            kind: diff::types::DiffResultKind::Binary,
            stats: diff::types::DiffStats::zero(diff::types::StatsUnit::Line),
            hunks: vec![],
            actual_algorithm,
            identical_with_filters: false,
        };
        match args.format.as_str() {
            "json" => print_json(&result),
            _ => print_text(&result, &args.old, &args.new, algorithm),
        }
        return Ok(());
    }

    let old_text = String::from_utf8_lossy(&old_bytes);
    let new_text = String::from_utf8_lossy(&new_bytes);

    // 5. 推断扩展名（优先 new，fallback old）
    let ext = extract_ext(new_path);
    let ext = if ext.is_empty() {
        extract_ext(old_path)
    } else {
        ext
    };

    // 6. 构建请求并调用核心 diff
    let req = DiffRequest {
        old: &old_text,
        new: &new_text,
        ext: &ext,
        mode,
        algorithm,
        context: args.context,
        whitespace: WhitespaceOpt {
            ignore_space_change: args.ignore_space_change,
            ignore_all_space: args.ignore_all_space,
            ignore_blank_lines: args.ignore_blank_lines,
            strip_trailing_cr: args.strip_trailing_cr,
        },
        force_text: args.text,
    };

    let result = diff::diff(req);

    // 7. 输出
    match args.format.as_str() {
        "json" => print_json(&result),
        _ => print_text(&result, &args.old, &args.new, algorithm),
    }

    Ok(())
}

// ── 文件大小检查 ────────────────────────────────────────────────────────────

fn check_file_size(path: &Path, max_bytes: u64, display_path: &str) -> CliResult {
    let meta = fs::metadata(path)
        .map_err(|e| CliError::new(1, format!("cannot stat '{}': {e}", display_path)))?;
    let size = meta.len();
    if size > max_bytes {
        return Err(CliError::with_details(
            1,
            format!(
                "'{}' is too large ({} bytes, limit {} bytes).",
                display_path, size, max_bytes
            ),
            &["Fix: Use --max-size to increase the limit, e.g. --max-size 2M"],
        ));
    }
    Ok(())
}

// ── JSON 输出 ───────────────────────────────────────────────────────────────

fn print_json(result: &DiffResult) {
    match serde_json::to_string(result) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Error: failed to serialize diff result: {e}"),
    }
}

// ── Text 输出（彩色 unified diff） ─────────────────────────────────────────

fn print_text(result: &DiffResult, old_path: &str, new_path: &str, requested: DiffAlgorithm) {
    let red = Style::new().red();
    let green = Style::new().green();
    let cyan = Style::new().cyan();
    let dim = Style::new().dim();
    let bold = Style::new().bold();

    // 算法近似映射提示
    if result.actual_algorithm != requested {
        eprintln!(
            "{}",
            dim.apply_to(format!(
                "Note: --diff-algorithm={} mapped to {} (imara-diff approximation)",
                algo_name(requested),
                algo_name(result.actual_algorithm),
            ))
        );
    }

    match result.kind {
        DiffResultKind::Binary => {
            eprintln!("Binary files {} and {} differ", old_path, new_path);
            return;
        }
        DiffResultKind::Identical => {
            if result.identical_with_filters {
                eprintln!("Files are identical (ignoring whitespace)");
            } else {
                eprintln!("Files are identical");
            }
            return;
        }
        _ => {}
    }

    // 文件头
    eprintln!("{}", bold.apply_to(format!("--- {old_path}")));
    eprintln!("{}", bold.apply_to(format!("+++ {new_path}")));

    // 逐 hunk 输出
    for hunk in &result.hunks {
        print_hunk_header(hunk, &cyan, &dim);

        for line in &hunk.lines {
            match line.tag {
                LineTag::Remove => eprintln!("{}", red.apply_to(format!("-{}", line.content))),
                LineTag::Add => eprintln!("{}", green.apply_to(format!("+{}", line.content))),
                LineTag::Context => eprintln!(" {}", line.content),
            }
        }
    }

    // 统计摘要
    print_stats_summary(&result.stats, &dim);
}

/// 输出 hunk 头（@@ -old_start,old_count +new_start,new_count @@ [symbol]）
fn print_hunk_header(hunk: &Hunk, cyan: &Style, dim: &Style) {
    let range = format!(
        "@@ -{},{} +{},{} @@",
        hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count,
    );

    // 附加符号/段信息
    let suffix = match (&hunk.symbol, &hunk.section) {
        (Some(sym), Some(sec)) => format!(" {sec}::{sym}"),
        (Some(sym), None) => format!(" {sym}"),
        (None, Some(sec)) => format!(" [{sec}]"),
        (None, None) => String::new(),
    };

    if suffix.is_empty() {
        eprintln!("{}", cyan.apply_to(&range));
    } else {
        eprint!("{}", cyan.apply_to(&range));
        eprintln!("{}", dim.apply_to(&suffix));
    }
}

/// 输出统计摘要行
fn print_stats_summary(stats: &DiffStats, dim: &Style) {
    let unit = match stats.unit {
        StatsUnit::Line => "line",
        StatsUnit::Symbol => "symbol",
    };

    let mut parts = Vec::new();
    if stats.added > 0 {
        parts.push(format!("+{} {unit}s added", stats.added));
    }
    if stats.removed > 0 {
        parts.push(format!("-{} {unit}s removed", stats.removed));
    }
    if stats.modified > 0 {
        parts.push(format!("~{} {unit}s modified", stats.modified));
    }

    if !parts.is_empty() {
        eprintln!("{}", dim.apply_to(parts.join(", ")));
    }
}

/// DiffAlgorithm → 显示名
fn algo_name(algo: DiffAlgorithm) -> &'static str {
    match algo {
        DiffAlgorithm::Histogram => "histogram",
        DiffAlgorithm::Myers => "myers",
        DiffAlgorithm::Minimal => "minimal",
        DiffAlgorithm::Patience => "patience",
    }
}
