// cstat/scanner.rs
//
// Line-classification state machine.
// Uses memchr SIMD for fast newline/marker scanning.

use memchr::{memchr, memmem};

use crate::cstat::lang::LangRules;

// ─── Public types ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) enum ScanState {
    #[default]
    Normal,
    BlockComment,
    BlockComment2,
}

#[derive(Default, Clone, Debug)]
pub(crate) struct FileStat {
    pub code: u32,
    pub comment: u32,
    pub blank: u32,
    pub bytes: u64,
}

impl FileStat {
    pub fn total_lines(&self) -> u32 {
        self.code + self.comment + self.blank
    }
}

// ─── Main scanner ────────────────────────────────────────────────────────────

/// Scan raw file bytes and return line statistics.
pub(crate) fn scan_bytes(content: &[u8], rules: &LangRules) -> FileStat {
    let bytes = content.len() as u64;
    let mut stat = if rules.name == "Vue" {
        scan_vue(content)
    } else {
        scan_generic(content, rules, &mut ScanState::Normal)
    };
    stat.bytes = bytes;
    stat
}

// ─── Generic scanner ─────────────────────────────────────────────────────────

fn scan_generic(content: &[u8], rules: &LangRules, state: &mut ScanState) -> FileStat {
    let mut stat = FileStat::default();
    let mut pos = 0;

    while pos < content.len() {
        let line_end = memchr(b'\n', &content[pos..])
            .map(|i| pos + i + 1)
            .unwrap_or(content.len());

        let line = &content[pos..line_end];
        classify_line(line, state, rules, &mut stat);
        pos = line_end;
    }

    stat
}

// ─── Vue SFC scanner ─────────────────────────────────────────────────────────

fn scan_vue(content: &[u8]) -> FileStat {
    let mut stat = FileStat::default();
    let mut state = ScanState::Normal;
    for (rules, bytes) in split_vue_sections(content) {
        let s = scan_generic(bytes, rules, &mut state);
        stat.code += s.code;
        stat.comment += s.comment;
        stat.blank += s.blank;
    }
    stat
}

fn split_vue_sections<'a>(content: &'a [u8]) -> Vec<(&'static LangRules, &'a [u8])> {
    use crate::cstat::lang::rules_for_ext;
    static HTML_R: std::sync::OnceLock<LangRules> = std::sync::OnceLock::new();
    static TS_R: std::sync::OnceLock<LangRules> = std::sync::OnceLock::new();
    static CSS_R: std::sync::OnceLock<LangRules> = std::sync::OnceLock::new();
    let html = HTML_R.get_or_init(|| rules_for_ext("html").unwrap());
    let ts = TS_R.get_or_init(|| rules_for_ext("ts").unwrap());
    let css = CSS_R.get_or_init(|| rules_for_ext("css").unwrap());
    let tags: &[(&[u8], &LangRules, &[u8])] = &[
        (b"<template", html, b"</template>"),
        (b"<script", ts, b"</script>"),
        (b"<style", css, b"</style>"),
    ];
    let mut out: Vec<(&'static LangRules, &'a [u8])> = Vec::new();
    let mut pos = 0usize;
    while pos < content.len() {
        let Some(lt) = memchr(b'<', &content[pos..]).map(|i| pos + i) else {
            out.push((html, &content[pos..]));
            break;
        };
        if lt > pos {
            out.push((html, &content[pos..lt]));
        }
        let rest = &content[lt..];
        let mut matched = false;
        for &(open, rules, close) in tags {
            if rest.starts_with(open)
                && let Some(gt) = memchr(b'>', rest)
            {
                let from = lt + gt + 1;
                if let Some(cp) = memmem::find(&content[from..], close) {
                    out.push((rules, &content[from..from + cp]));
                    pos = from + cp + close.len();
                } else {
                    out.push((rules, &content[from..]));
                    pos = content.len();
                }
                matched = true;
                break;
            }
        }
        if !matched {
            out.push((html, &content[lt..lt + 1]));
            pos = lt + 1;
        }
    }
    out
}

// ─── Line classifier ─────────────────────────────────────────────────────────

fn classify_line(line: &[u8], state: &mut ScanState, rules: &LangRules, stat: &mut FileStat) {
    let line = trim_line_ending(line);

    // Inside block comment (primary)
    if *state == ScanState::BlockComment {
        let end = rules.block_end.unwrap_or("*/").as_bytes();
        if let Some(pos) = memmem::find(line, end) {
            *state = ScanState::Normal;
            if has_code_chars(&line[pos + end.len()..]) {
                stat.code += 1;
            } else {
                stat.comment += 1;
            }
        } else {
            stat.comment += 1;
        }
        return;
    }

    // Inside block comment (secondary, e.g. <!-- -->)
    if *state == ScanState::BlockComment2 {
        let end = rules.block_end2.unwrap_or("-->").as_bytes();
        if let Some(pos) = memmem::find(line, end) {
            *state = ScanState::Normal;
            if has_code_chars(&line[pos + end.len()..]) {
                stat.code += 1;
            } else {
                stat.comment += 1;
            }
        } else {
            stat.comment += 1;
        }
        return;
    }

    let trimmed = trim_leading_ws(line);
    if trimmed.is_empty() {
        stat.blank += 1;
        return;
    }

    // Line comment
    if let Some(lc) = rules.line_comment
        && trimmed.starts_with(lc.as_bytes())
    {
        stat.comment += 1;
        return;
    }

    // Block comment start (primary)
    if let (Some(bs), Some(be)) = (rules.block_start, rules.block_end)
        && trimmed.starts_with(bs.as_bytes())
    {
        let after = &trimmed[bs.len()..];
        if memmem::find(after, be.as_bytes()).is_some() {
            stat.comment += 1;
        } else {
            *state = ScanState::BlockComment;
            stat.comment += 1;
        }
        return;
    }

    // Block comment start (secondary)
    if let (Some(bs2), Some(be2)) = (rules.block_start2, rules.block_end2)
        && trimmed.starts_with(bs2.as_bytes())
    {
        let after = &trimmed[bs2.len()..];
        if memmem::find(after, be2.as_bytes()).is_some() {
            stat.comment += 1;
        } else {
            *state = ScanState::BlockComment2;
            stat.comment += 1;
        }
        return;
    }

    // Mid-line block comment open
    if let (Some(bs), Some(be)) = (rules.block_start, rules.block_end)
        && let Some(op) = memmem::find(trimmed, bs.as_bytes())
        && memmem::find(&trimmed[op + bs.len()..], be.as_bytes()).is_none()
    {
        *state = ScanState::BlockComment;
    }

    stat.code += 1;
}

// ─── Byte helpers ────────────────────────────────────────────────────────────

#[inline]
fn trim_line_ending(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

#[inline]
fn trim_leading_ws(s: &[u8]) -> &[u8] {
    let start = s
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(s.len());
    &s[start..]
}

#[inline]
fn has_code_chars(s: &[u8]) -> bool {
    s.iter()
        .any(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
}
