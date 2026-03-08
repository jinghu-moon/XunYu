use std::path::Path;

use regex::Regex;

use crate::config::RedirectRule;
use crate::util::glob_match;

use super::ExplainDetail;
use super::path_norm::{file_ext_lower, normalize_ext, regex_is_match_cached};
use super::score::{age_secs, parse_age_expr, parse_size_expr, size_matches};

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

pub(super) fn explain_rule_pure(file_name: &str, rule: &RedirectRule) -> ExplainDetail {
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

pub(super) fn match_path<'a>(
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

pub(super) fn match_file<'a>(
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

pub(super) fn any_rule_matches_name_only(file_name: &str, rules: &[RedirectRule]) -> bool {
    rules.iter().any(|r| rule_matches_name_only(file_name, r))
}
