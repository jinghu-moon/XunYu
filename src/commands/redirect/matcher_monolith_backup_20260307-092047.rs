use crate::config::RedirectRule;
use crate::util::glob_match;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub(crate) struct ExplainDetail {
    pub(crate) matched: bool,
    pub(crate) summary: String,
}

#[derive(Clone, Copy)]
pub(crate) enum SizeOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

static SIZE_EXPR_CACHE: OnceLock<Mutex<HashMap<String, Result<(SizeOp, u64), String>>>> =
    OnceLock::new();
static AGE_EXPR_CACHE: OnceLock<Mutex<HashMap<String, Result<(SizeOp, u64), String>>>> =
    OnceLock::new();

fn parse_size_expr_uncached(s: &str) -> Result<(SizeOp, u64), String> {
    let (op, rest) = if let Some(r) = s.strip_prefix(">=") {
        (SizeOp::Ge, r)
    } else if let Some(r) = s.strip_prefix("<=") {
        (SizeOp::Le, r)
    } else if let Some(r) = s.strip_prefix('>') {
        (SizeOp::Gt, r)
    } else if let Some(r) = s.strip_prefix('<') {
        (SizeOp::Lt, r)
    } else if let Some(r) = s.strip_prefix('=') {
        (SizeOp::Eq, r)
    } else {
        return Err("need one of >,>=,<,<=,=".to_string());
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return Err("missing size value".to_string());
    }

    let mut num = String::new();
    let mut unit = String::new();
    for c in rest.chars() {
        if c.is_ascii_digit() || c == '.' {
            if unit.is_empty() {
                num.push(c);
            } else {
                return Err("invalid size format".to_string());
            }
        } else if !c.is_whitespace() {
            unit.push(c);
        }
    }

    let value: f64 = num.parse().map_err(|_| "invalid number".to_string())?;
    if !value.is_finite() || value < 0.0 {
        return Err("invalid number".to_string());
    }

    let unit = unit.to_ascii_lowercase();
    let mul: f64 = match unit.as_str() {
        "" | "b" => 1.0,
        "kb" | "k" => 1024.0,
        "mb" | "m" => 1024.0 * 1024.0,
        "gb" | "g" => 1024.0 * 1024.0 * 1024.0,
        _ => return Err(format!("unknown unit: {unit}")),
    };

    let bytes = (value * mul).round();
    if bytes > (u64::MAX as f64) {
        return Err("size too large".to_string());
    }
    Ok((op, bytes as u64))
}

pub(crate) fn parse_size_expr(raw: &str) -> Result<(SizeOp, u64), String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty".to_string());
    }

    let cache = SIZE_EXPR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let guard = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = guard.get(s) {
            return cached.clone();
        }
    }

    let parsed = parse_size_expr_uncached(s);
    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(s.to_string(), parsed.clone());
    parsed
}

fn parse_age_expr_uncached(s: &str) -> Result<(SizeOp, u64), String> {
    let (op, rest) = if let Some(r) = s.strip_prefix(">=") {
        (SizeOp::Ge, r)
    } else if let Some(r) = s.strip_prefix("<=") {
        (SizeOp::Le, r)
    } else if let Some(r) = s.strip_prefix('>') {
        (SizeOp::Gt, r)
    } else if let Some(r) = s.strip_prefix('<') {
        (SizeOp::Lt, r)
    } else if let Some(r) = s.strip_prefix('=') {
        (SizeOp::Eq, r)
    } else {
        return Err("need one of >,>=,<,<=,=".to_string());
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return Err("missing age value".to_string());
    }

    let mut num = String::new();
    let mut unit = String::new();
    for c in rest.chars() {
        if c.is_ascii_digit() || c == '.' {
            if unit.is_empty() {
                num.push(c);
            } else {
                return Err("invalid age format".to_string());
            }
        } else if !c.is_whitespace() {
            unit.push(c);
        }
    }
    let value: f64 = num.parse().map_err(|_| "invalid number".to_string())?;
    if !value.is_finite() || value < 0.0 {
        return Err("invalid number".to_string());
    }

    let unit = unit.to_ascii_lowercase();
    let mul: f64 = match unit.as_str() {
        "s" | "sec" | "secs" => 1.0,
        "m" | "min" | "mins" => 60.0,
        "h" | "hr" | "hrs" => 3600.0,
        "d" | "day" | "days" => 86400.0,
        "w" | "week" | "weeks" => 7.0 * 86400.0,
        _ => return Err(format!("unknown unit: {unit}")),
    };

    let secs = (value * mul).round();
    if secs > (u64::MAX as f64) {
        return Err("age too large".to_string());
    }
    Ok((op, secs as u64))
}

pub(crate) fn parse_age_expr(raw: &str) -> Result<(SizeOp, u64), String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty".to_string());
    }

    let cache = AGE_EXPR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let guard = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = guard.get(s) {
            return cached.clone();
        }
    }

    let parsed = parse_age_expr_uncached(s);
    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(s.to_string(), parsed.clone());
    parsed
}

fn normalize_ext(s: &str) -> String {
    s.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn file_ext_lower(file_name: &str) -> Option<String> {
    Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
}

static REGEX_CACHE: OnceLock<Mutex<HashMap<String, Result<Regex, ()>>>> = OnceLock::new();

fn regex_is_match_cached(pattern: &str, text: &str) -> bool {
    let cache = REGEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let guard = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = guard.get(pattern) {
            return cached.as_ref().map(|rx| rx.is_match(text)).unwrap_or(false);
        }
    }

    let compiled = Regex::new(pattern).map_err(|_| ());
    let ok = compiled
        .as_ref()
        .map(|rx| rx.is_match(text))
        .unwrap_or(false);

    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(pattern.to_string(), compiled);
    ok
}

fn size_matches(size_bytes: u64, op: SizeOp, rhs: u64) -> bool {
    match op {
        SizeOp::Lt => size_bytes < rhs,
        SizeOp::Le => size_bytes <= rhs,
        SizeOp::Gt => size_bytes > rhs,
        SizeOp::Ge => size_bytes >= rhs,
        SizeOp::Eq => size_bytes == rhs,
    }
}

fn age_secs(modified: SystemTime) -> Option<u64> {
    let now = SystemTime::now();
    let now_s = now.duration_since(UNIX_EPOCH).ok()?.as_secs();
    let m_s = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(now_s.saturating_sub(m_s))
}

fn rule_matches(src_path: &Path, file_name: &str, rule: &RedirectRule) -> bool {
    let ext_ok = if rule.match_cond.ext.is_empty() {
        true
    } else {
        let Some(ext) = file_ext_lower(file_name) else {
            return false;
        };
        rule.match_cond
            .ext
            .iter()
            .map(|e| normalize_ext(e))
            .any(|e| e == ext)
    };

    let glob_ok = match rule.match_cond.glob.as_deref() {
        Some(g) if !g.trim().is_empty() => {
            let pat = g.trim().to_ascii_lowercase();
            let name = file_name.to_ascii_lowercase();
            glob_match(&pat, &name)
        }
        _ => true,
    };

    let regex_ok = match rule.match_cond.regex.as_deref() {
        Some(re) if !re.trim().is_empty() => regex_is_match_cached(re.trim(), file_name),
        _ => true,
    };

    let size_ok = match rule.match_cond.size.as_deref() {
        Some(sz) if !sz.trim().is_empty() => {
            let Ok(meta) = std::fs::metadata(src_path) else {
                return false;
            };
            let (op, rhs) = match parse_size_expr(sz) {
                Ok(v) => v,
                Err(_) => return false,
            };
            size_matches(meta.len(), op, rhs)
        }
        _ => true,
    };

    let age_ok = match rule.match_cond.age.as_deref() {
        Some(age) if !age.trim().is_empty() => {
            let Ok(meta) = std::fs::metadata(src_path) else {
                return false;
            };
            let Ok(modified) = meta.modified() else {
                return false;
            };
            let Some(secs) = age_secs(modified) else {
                return false;
            };
            let (op, rhs) = match parse_age_expr(age) {
                Ok(v) => v,
                Err(_) => return false,
            };
            size_matches(secs, op, rhs)
        }
        _ => true,
    };

    ext_ok && glob_ok && regex_ok && size_ok && age_ok
}

pub(crate) fn explain_rule_pure(file_name: &str, rule: &RedirectRule) -> ExplainDetail {
    let mut parts: Vec<String> = Vec::new();

    let ext_ok = if rule.match_cond.ext.is_empty() {
        parts.push("ext=N/A".to_string());
        true
    } else {
        match file_ext_lower(file_name) {
            Some(ext) => {
                let ok = rule
                    .match_cond
                    .ext
                    .iter()
                    .map(|e| normalize_ext(e))
                    .any(|e| e == ext);
                parts.push(format!(
                    "ext={ext} {}",
                    if ok { "matched" } else { "no match" }
                ));
                ok
            }
            None => {
                parts.push("ext=missing".to_string());
                false
            }
        }
    };

    let glob_ok = match rule.match_cond.glob.as_deref() {
        Some(g) if !g.trim().is_empty() => {
            let pat = g.trim().to_ascii_lowercase();
            let name = file_name.to_ascii_lowercase();
            let ok = glob_match(&pat, &name);
            parts.push(format!(
                "glob=\"{g}\" {}",
                if ok { "matched" } else { "no match" }
            ));
            ok
        }
        _ => {
            parts.push("glob=N/A".to_string());
            true
        }
    };

    let regex_ok = match rule.match_cond.regex.as_deref() {
        Some(re) if !re.trim().is_empty() => match Regex::new(re) {
            Ok(rx) => {
                let ok = rx.is_match(file_name);
                parts.push(format!(
                    "regex=/{re}/ {}",
                    if ok { "matched" } else { "no match" }
                ));
                ok
            }
            Err(e) => {
                parts.push(format!("regex=/{re}/ invalid:{e}"));
                false
            }
        },
        _ => {
            parts.push("regex=N/A".to_string());
            true
        }
    };

    let size_ok = match rule.match_cond.size.as_deref() {
        Some(sz) if !sz.trim().is_empty() => {
            parts.push(format!("size=\"{sz}\" needs file metadata"));
            false
        }
        _ => {
            parts.push("size=N/A".to_string());
            true
        }
    };

    let age_ok = match rule.match_cond.age.as_deref() {
        Some(age) if !age.trim().is_empty() => {
            parts.push(format!("age=\"{age}\" needs file metadata"));
            false
        }
        _ => {
            parts.push("age=N/A".to_string());
            true
        }
    };

    let matched = ext_ok && glob_ok && regex_ok && size_ok && age_ok;
    ExplainDetail {
        matched,
        summary: parts.join(", "),
    }
}

pub(crate) fn match_path<'a>(
    src_path: &Path,
    rules: &'a [RedirectRule],
) -> Option<&'a RedirectRule> {
    let file_name = src_path.file_name()?.to_str()?;
    for r in rules {
        if rule_matches(src_path, file_name, r) {
            return Some(r);
        }
    }
    None
}

pub(crate) fn match_file<'a>(
    file_name: &str,
    rules: &'a [RedirectRule],
) -> Option<&'a RedirectRule> {
    for r in rules {
        if rule_matches(Path::new(file_name), file_name, r) {
            return Some(r);
        }
    }
    None
}

fn rule_matches_name_only(file_name: &str, rule: &RedirectRule) -> bool {
    let ext_ok = if rule.match_cond.ext.is_empty() {
        true
    } else {
        let Some(ext) = file_ext_lower(file_name) else {
            return false;
        };
        rule.match_cond
            .ext
            .iter()
            .map(|e| normalize_ext(e))
            .any(|e| e == ext)
    };

    let glob_ok = match rule.match_cond.glob.as_deref() {
        Some(g) if !g.trim().is_empty() => {
            let pat = g.trim().to_ascii_lowercase();
            let name = file_name.to_ascii_lowercase();
            glob_match(&pat, &name)
        }
        _ => true,
    };

    let regex_ok = match rule.match_cond.regex.as_deref() {
        Some(re) if !re.trim().is_empty() => regex_is_match_cached(re.trim(), file_name),
        _ => true,
    };

    ext_ok && glob_ok && regex_ok
}

pub(crate) fn any_rule_matches_name_only(file_name: &str, rules: &[RedirectRule]) -> bool {
    rules.iter().any(|r| rule_matches_name_only(file_name, r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MatchCondition;

    fn rule_with(cond: MatchCondition) -> RedirectRule {
        RedirectRule {
            name: "r".to_string(),
            match_cond: cond,
            dest: "./Dest".to_string(),
        }
    }

    #[test]
    fn any_rule_matches_name_only_ext_is_case_insensitive() {
        let rules = vec![rule_with(MatchCondition {
            ext: vec!["jpg".to_string()],
            ..Default::default()
        })];
        assert!(any_rule_matches_name_only("A.JPG", &rules));
        assert!(!any_rule_matches_name_only("A.PNG", &rules));
    }

    #[test]
    fn any_rule_matches_name_only_glob_is_case_insensitive() {
        let rules = vec![rule_with(MatchCondition {
            glob: Some("report_*.pdf".to_string()),
            ..Default::default()
        })];
        assert!(any_rule_matches_name_only("report_2026.pdf", &rules));
        assert!(any_rule_matches_name_only("REPORT_2026.PDF", &rules));
        assert!(!any_rule_matches_name_only("notes_2026.pdf", &rules));
    }

    #[test]
    fn any_rule_matches_name_only_regex_uses_cache_and_rejects_invalid() {
        let ok_rules = vec![rule_with(MatchCondition {
            regex: Some(r"^a\d+\.txt$".to_string()),
            ..Default::default()
        })];
        assert!(any_rule_matches_name_only("a12.txt", &ok_rules));
        assert!(!any_rule_matches_name_only("b12.txt", &ok_rules));

        let bad_rules = vec![rule_with(MatchCondition {
            regex: Some("(".to_string()),
            ..Default::default()
        })];
        assert!(!any_rule_matches_name_only("a12.txt", &bad_rules));
    }

    #[test]
    fn any_rule_matches_name_only_size_only_rule_is_considered_possible() {
        let rules = vec![rule_with(MatchCondition {
            size: Some(">1kb".to_string()),
            ..Default::default()
        })];
        assert!(any_rule_matches_name_only("anything.bin", &rules));
    }

    #[test]
    fn parse_size_expr_parses_ops_and_units() {
        let (op, bytes) = parse_size_expr(">= 1kb").unwrap();
        assert!(matches!(op, SizeOp::Ge));
        assert_eq!(bytes, 1024);

        let (op, bytes) = parse_size_expr("=1MB").unwrap();
        assert!(matches!(op, SizeOp::Eq));
        assert_eq!(bytes, 1024 * 1024);

        assert!(parse_size_expr("wat").is_err());
    }

    #[test]
    fn parse_age_expr_parses_ops_and_units() {
        let (op, secs) = parse_age_expr("> 1d").unwrap();
        assert!(matches!(op, SizeOp::Gt));
        assert_eq!(secs, 86400);

        let (op, secs) = parse_age_expr("<=2w").unwrap();
        assert!(matches!(op, SizeOp::Le));
        assert_eq!(secs, 2 * 7 * 86400);

        assert!(parse_age_expr(">= 1qq").is_err());
    }
}
