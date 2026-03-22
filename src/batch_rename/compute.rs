// batch_rename/compute.rs
//
// Pure computation for 6 rename modes.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use heck::{ToKebabCase, ToPascalCase, ToSnakeCase};
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

use crate::batch_rename::types::{CaseStyle, RenameOp};
use crate::output::{CliError, CliResult};

/// A single from→to literal replacement pair.
#[derive(Clone, Debug)]
pub struct ReplacePair {
    pub from: String,
    pub to: String,
}

/// Which rename mode is active.
pub enum RenameMode {
    Regex {
        pattern: String,
        replace: String,
    },
    Case(CaseStyle),
    Prefix(String),
    Suffix(String),
    StripPrefix(String),
    StripSuffix(String),
    Replace(Vec<ReplacePair>),
    RenameExt {
        from: String,
        to: String,
    },
    /// Apply case transformation to the extension only.
    ExtCase(CaseStyle),
    /// Insert a string at a given character position in the stem.
    InsertAt {
        pos: usize,
        insert: String,
    },
    /// Extended sequence: prefix position or stem-replace.
    SeqExt {
        start: usize,
        pad: usize,
        prefix: bool,
        only: bool,
    },
    /// Remove bracketed content from stem and trim whitespace.
    StripBrackets {
        round: bool,
        square: bool,
        curly: bool,
    },
    /// Trim whitespace or specific characters from stem ends.
    Trim {
        chars: Option<String>,
    },
    /// Slice the stem using Python-style indices (negative = from end).
    Slice {
        start: Option<i64>,
        end: Option<i64>,
    },
    /// Insert file date (mtime or ctime) into the stem.
    InsertDate {
        fmt: String,
        use_ctime: bool,
        prefix: bool,
    },
    /// Pad the last numeric group in the stem to a fixed width.
    NormalizeSeq {
        pad: usize,
    },
    /// Rename using a template string with variables like {stem}, {ext}, {n}, etc.
    Template {
        tpl: String,
        start: usize,
        pad: usize,
    },
    /// Remove all occurrences of specified characters from the stem.
    RemoveChars {
        chars: String,
    },
    /// Add an extension to files that have no extension.
    AddExt {
        ext: String,
    },
    /// Replace the last numeric group in each stem with a new sequential number.
    Renumber {
        start: usize,
        pad: usize,
    },
    /// Normalize Unicode form of the stem (nfc, nfd, nfkc, nfkd).
    NormalizeUnicode {
        form: String,
    },
}

/// Apply multiple rename steps in sequence. Each step's output becomes the next step's input.
/// Empty steps returns noop ops (from == to).
pub fn compute_ops_chain(files: &[PathBuf], steps: &[RenameMode]) -> CliResult<Vec<RenameOp>> {
    if steps.is_empty() {
        return Ok(files
            .iter()
            .map(|f| RenameOp {
                from: f.clone(),
                to: f.clone(),
            })
            .collect());
    }
    // First step uses original files
    let first_ops = compute_ops(files, &steps[0])?;
    // Track original `from` and accumulate final `to`
    let mut result: Vec<RenameOp> = first_ops;
    for step in &steps[1..] {
        // Move `to` out of result to avoid clone; compute_ops needs owned Vec<PathBuf>
        let current_tos: Vec<PathBuf> = result
            .iter_mut()
            .map(|o| std::mem::take(&mut o.to))
            .collect();
        let next_ops = compute_ops(&current_tos, step)?;
        // Restore: keep original `from`, take new `to`, restore intermediate as `to`
        for ((r, new_op), intermediate) in result.iter_mut().zip(next_ops).zip(current_tos) {
            r.to = if new_op.to != intermediate {
                new_op.to
            } else {
                intermediate
            };
        }
    }
    Ok(result)
}

/// Dispatch to the correct rename mode.
pub fn compute_ops(files: &[PathBuf], mode: &RenameMode) -> CliResult<Vec<RenameOp>> {
    match mode {
        RenameMode::Regex { pattern, replace } => mode_regex(files, pattern, replace),
        RenameMode::Case(style) => mode_case(files, style),
        RenameMode::Prefix(p) => mode_prefix(files, p),
        RenameMode::Suffix(s) => mode_suffix(files, s),
        RenameMode::StripPrefix(s) => mode_strip_prefix(files, s),
        RenameMode::StripSuffix(s) => mode_strip_suffix(files, s),
        RenameMode::SeqExt {
            start,
            pad,
            prefix,
            only,
        } => mode_seq_ext(files, *start, *pad, *prefix, *only),
        RenameMode::Replace(pairs) => mode_replace(files, pairs),
        RenameMode::RenameExt { from, to } => mode_rename_ext(files, from, to),
        RenameMode::ExtCase(style) => mode_ext_case(files, style),
        RenameMode::InsertAt { pos, insert } => mode_insert_at(files, *pos, insert),
        RenameMode::StripBrackets {
            round,
            square,
            curly,
        } => mode_strip_brackets(files, *round, *square, *curly),
        RenameMode::Trim { chars } => mode_trim(files, chars.as_deref()),
        RenameMode::Slice { start, end } => mode_slice(files, *start, *end),
        RenameMode::InsertDate {
            fmt,
            use_ctime,
            prefix,
        } => mode_insert_date(files, fmt, *use_ctime, *prefix),
        RenameMode::NormalizeSeq { pad } => mode_normalize_seq(files, *pad),
        RenameMode::Template { tpl, start, pad } => mode_template(files, tpl, *start, *pad),
        RenameMode::RemoveChars { chars } => mode_remove_chars(files, chars),
        RenameMode::AddExt { ext } => mode_add_ext(files, ext),
        RenameMode::Renumber { start, pad } => mode_renumber(files, *start, *pad),
        RenameMode::NormalizeUnicode { form } => mode_normalize_unicode(files, form),
    }
}

// ─── Rename modes ────────────────────────────────────────────────────────────

fn mode_regex(files: &[PathBuf], pattern: &str, replacement: &str) -> CliResult<Vec<RenameOp>> {
    let re = Regex::new(pattern)
        .map_err(|e| CliError::new(1, format!("Invalid regex '{}': {}", pattern, e)))?;
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = re.replace_all(&stem, replacement).into_owned();
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_case(files: &[PathBuf], style: &CaseStyle) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = match style {
                CaseStyle::Kebab => stem.to_kebab_case(),
                CaseStyle::Snake => stem.to_snake_case(),
                CaseStyle::Pascal => stem.to_pascal_case(),
                CaseStyle::Upper => stem.to_uppercase(),
                CaseStyle::Lower => stem.to_lowercase(),
                CaseStyle::Title => title_case(&stem),
            };
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_prefix(files: &[PathBuf], prefix: &str) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}{}", prefix, stem, ext)),
            }
        })
        .collect())
}

fn mode_suffix(files: &[PathBuf], suffix: &str) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}{}", stem, suffix, ext)),
            }
        })
        .collect())
}

fn mode_strip_prefix(files: &[PathBuf], strip: &str) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = stem.strip_prefix(strip).unwrap_or(&stem).to_string();
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub fn split_stem_ext(path: &Path) -> (String, String) {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_owned();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    (stem, ext)
}

fn sibling(original: &Path, new_name: &str) -> PathBuf {
    original
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(new_name)
}

fn mode_replace(files: &[PathBuf], pairs: &[ReplacePair]) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = pairs.iter().fold(stem, |acc, pair| {
                acc.replace(pair.from.as_str(), pair.to.as_str())
            });
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_strip_suffix(files: &[PathBuf], suffix: &str) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = if stem.ends_with(suffix) {
                stem[..stem.len() - suffix.len()].to_owned()
            } else {
                stem // no match → noop (from == to)
            };
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_rename_ext(files: &[PathBuf], from_ext: &str, to_ext: &str) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let current_ext = ext.trim_start_matches('.');
            let new_name = if current_ext.eq_ignore_ascii_case(from_ext) {
                format!("{}.{}", stem, to_ext)
            } else {
                format!("{}{}", stem, ext) // noop
            };
            RenameOp {
                from: f.clone(),
                to: sibling(f, &new_name),
            }
        })
        .collect())
}

fn mode_ext_case(files: &[PathBuf], style: &CaseStyle) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_ext = match style {
                CaseStyle::Lower => ext.to_lowercase(),
                CaseStyle::Upper => ext.to_uppercase(),
                _ => ext.clone(),
            };
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", stem, new_ext)),
            }
        })
        .collect())
}

fn mode_insert_at(files: &[PathBuf], pos: usize, insert: &str) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let clamped = pos.min(stem.len());
            let new_stem = format!("{}{}{}", &stem[..clamped], insert, &stem[clamped..]);
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_seq_ext(
    files: &[PathBuf],
    start: usize,
    pad: usize,
    prefix: bool,
    only: bool,
) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let (stem, ext) = split_stem_ext(f);
            let n = format!("{:0>width$}", start + i, width = pad);
            let new_stem = if only {
                n
            } else if prefix {
                format!("{}_{}", n, stem)
            } else {
                format!("{}_{}", stem, n)
            };
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_strip_brackets(
    files: &[PathBuf],
    round: bool,
    square: bool,
    curly: bool,
) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let mut s = stem;
            if round {
                // Remove (...) including surrounding whitespace, repeatedly
                loop {
                    let new = remove_bracket_pair(&s, '(', ')');
                    if new == s {
                        break;
                    }
                    s = new;
                }
            }
            if square {
                loop {
                    let new = remove_bracket_pair(&s, '[', ']');
                    if new == s {
                        break;
                    }
                    s = new;
                }
            }
            if curly {
                loop {
                    let new = remove_bracket_pair(&s, '{', '}');
                    if new == s {
                        break;
                    }
                    s = new;
                }
            }
            let new_stem = s.trim().to_owned();
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

/// Remove the first matching bracket pair and content, plus surrounding spaces.
fn remove_bracket_pair(s: &str, open: char, close: char) -> String {
    if let Some(start) = s.find(open)
        && let Some(end) = s[start..].find(close)
    {
        let end_abs = start + end + close.len_utf8();
        // Also consume leading/trailing spaces around the bracket
        let before = s[..start].trim_end();
        let after = s[end_abs..].trim_start();
        return format!("{} {}", before, after).trim().to_owned();
    }
    s.to_owned()
}

fn mode_trim(files: &[PathBuf], chars: Option<&str>) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = match chars {
                Some(c) => {
                    let chars_vec: Vec<char> = c.chars().collect();
                    stem.trim_matches(chars_vec.as_slice()).to_owned()
                }
                None => stem.trim().to_owned(),
            };
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_slice(files: &[PathBuf], start: Option<i64>, end: Option<i64>) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let chars: Vec<char> = stem.chars().collect();
            let len = chars.len() as i64;

            // Resolve negative indices (Python semantics)
            let resolve = |idx: i64| -> usize {
                if idx < 0 {
                    (len + idx).max(0) as usize
                } else {
                    idx.min(len) as usize
                }
            };

            let s = resolve(start.unwrap_or(0));
            let e = resolve(end.unwrap_or(len));
            let new_stem: String = if s >= e {
                String::new()
            } else {
                chars[s..e].iter().collect()
            };
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

/// Capitalise the first letter of each word, where words are delimited by
/// whitespace, hyphens, or underscores.  Other characters are left as-is.
fn title_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut cap_next = true;
    for ch in s.chars() {
        if ch == ' ' || ch == '-' || ch == '_' {
            cap_next = true;
            result.push(ch);
        } else if cap_next {
            result.extend(ch.to_uppercase());
            cap_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

fn mode_insert_date(
    files: &[PathBuf],
    fmt: &str,
    use_ctime: bool,
    prefix: bool,
) -> CliResult<Vec<RenameOp>> {
    files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            // Get file timestamp
            let meta = f.metadata().map_err(|e| {
                CliError::new(
                    1,
                    format!("Cannot read metadata for '{}': {}", f.display(), e),
                )
            })?;
            let sys_time = if use_ctime {
                // On Windows, created() is the creation time
                meta.created().or_else(|_| meta.modified())
            } else {
                meta.modified()
            }
            .map_err(|e| {
                CliError::new(
                    1,
                    format!("Cannot read file time for '{}': {}", f.display(), e),
                )
            })?;

            let dt: DateTime<Local> = sys_time.into();
            let date_str = dt.format(fmt).to_string();

            let new_stem = if prefix {
                format!("{}_{}", date_str, stem)
            } else {
                format!("{}_{}", stem, date_str)
            };
            Ok(RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            })
        })
        .collect()
}

/// Pad the last numeric group in the stem to `pad` digits (minimum width).
/// If the stem contains no digits, return a noop (from == to).
fn mode_normalize_seq(files: &[PathBuf], pad: usize) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = pad_last_number(&stem, pad);
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

/// Find the last contiguous digit run in `s` and pad it to `pad` width.
/// Returns the original string if no digits are found.
fn pad_last_number(s: &str, pad: usize) -> String {
    // Find the last digit group: scan from end
    let bytes = s.as_bytes();
    let mut end = bytes.len();
    // Walk backwards to find last digit
    while end > 0 && !bytes[end - 1].is_ascii_digit() {
        end -= 1;
    }
    if end == 0 {
        return s.to_owned(); // no digits
    }
    let mut start = end;
    while start > 0 && bytes[start - 1].is_ascii_digit() {
        start -= 1;
    }
    let num_str = &s[start..end];
    let n: u64 = num_str.parse().unwrap_or(0);
    let padded = format!("{:0>width$}", n, width = pad);
    format!("{}{}{}", &s[..start], padded, &s[end..])
}

/// Apply a template string to each file.
/// Supported variables: {stem}, {ext}, {n}, {upper}, {lower}, {parent}.
fn mode_template(
    files: &[PathBuf],
    tpl: &str,
    start: usize,
    pad: usize,
) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let (stem, ext) = split_stem_ext(f);
            let n = format!("{:0>width$}", start + i, width = pad);
            let parent = f
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let new_name = tpl
                .replace("{stem}", &stem)
                .replace("{ext}", &ext)
                .replace("{n}", &n)
                .replace("{upper}", &stem.to_uppercase())
                .replace("{lower}", &stem.to_lowercase())
                .replace("{parent}", parent);
            RenameOp {
                from: f.clone(),
                to: sibling(f, &new_name),
            }
        })
        .collect())
}

fn mode_remove_chars(files: &[PathBuf], chars: &str) -> CliResult<Vec<RenameOp>> {
    let char_set: std::collections::HashSet<char> = chars.chars().collect();
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem: String = stem.chars().filter(|c| !char_set.contains(c)).collect();
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_add_ext(files: &[PathBuf], ext: &str) -> CliResult<Vec<RenameOp>> {
    let ext_clean = ext.trim_start_matches('.');
    Ok(files
        .iter()
        .map(|f| {
            if f.extension().is_some() {
                RenameOp {
                    from: f.clone(),
                    to: f.clone(),
                }
            } else {
                let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
                RenameOp {
                    from: f.clone(),
                    to: sibling(f, &format!("{}.{}", name, ext_clean)),
                }
            }
        })
        .collect())
}

fn mode_renumber(files: &[PathBuf], start: usize, pad: usize) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let (stem, ext) = split_stem_ext(f);
            let new_num = format!("{:0>width$}", start + i, width = pad);
            let new_stem = replace_last_number(&stem, &new_num);
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

/// Replace the last contiguous digit run in `s` with `replacement`.
fn replace_last_number(s: &str, replacement: &str) -> String {
    let bytes = s.as_bytes();
    let mut end = bytes.len();
    while end > 0 && !bytes[end - 1].is_ascii_digit() {
        end -= 1;
    }
    if end == 0 {
        return s.to_owned();
    }
    let mut start = end;
    while start > 0 && bytes[start - 1].is_ascii_digit() {
        start -= 1;
    }
    format!("{}{}{}", &s[..start], replacement, &s[end..])
}

fn mode_normalize_unicode(files: &[PathBuf], form: &str) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = match form.to_ascii_lowercase().as_str() {
                "nfc" => stem.nfc().collect::<String>(),
                "nfd" => stem.nfd().collect::<String>(),
                "nfkc" => stem.nfkc().collect::<String>(),
                "nfkd" => stem.nfkd().collect::<String>(),
                _ => stem.clone(),
            };
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}
