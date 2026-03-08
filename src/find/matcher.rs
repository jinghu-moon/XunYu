use super::glob::{glob_match_component, match_path_parts, split_path_parts};
use super::rules::{CompiledRules, PatternType, Rule, RuleKind};

pub(crate) struct MatchDecision {
    pub(crate) final_state: RuleKind,
    pub(crate) explicit: bool,
    pub(crate) rule_idx: Option<usize>,
}

pub(crate) fn determine_path_state(
    rules: &CompiledRules,
    path: &str,
    is_dir: bool,
    inherited: RuleKind,
) -> MatchDecision {
    let path = normalize_match_path(path);
    let path_parts = split_path_parts(path);
    let filename = path_parts.last().copied().unwrap_or("");

    let mut best_exact: Option<usize> = None;
    if !path.is_empty() {
        let path_key = normalize_key_case(path, rules.case_sensitive);
        if let Some(indices) = rules.exact_by_path.get(&path_key) {
            for idx in indices {
                let rule = &rules.rules[*idx];
                if rule.dir_only && !is_dir {
                    continue;
                }
                if best_exact.map_or(true, |b| *idx > b) {
                    best_exact = Some(*idx);
                }
            }
        }
    }
    if !filename.is_empty() {
        let name_key = normalize_key_case(filename, rules.case_sensitive);
        if let Some(indices) = rules.exact_by_name.get(&name_key) {
            for idx in indices {
                let rule = &rules.rules[*idx];
                if rule.dir_only && !is_dir {
                    continue;
                }
                if best_exact.map_or(true, |b| *idx > b) {
                    best_exact = Some(*idx);
                }
            }
        }
    }

    if let Some(idx) = best_exact {
        let rule = &rules.rules[idx];
        return MatchDecision {
            final_state: rule.kind,
            explicit: true,
            rule_idx: Some(rule.idx),
        };
    }

    if let Some(idx) = find_fuzzy_match(rules, &path_parts, filename, path, is_dir) {
        let rule = &rules.rules[idx];
        return MatchDecision {
            final_state: rule.kind,
            explicit: true,
            rule_idx: Some(rule.idx),
        };
    }

    MatchDecision {
        final_state: inherited,
        explicit: false,
        rule_idx: None,
    }
}

pub(crate) fn rule_matches(rule: &Rule, path: &str, is_dir: bool, case_sensitive: bool) -> bool {
    let path = normalize_match_path(path);
    let path_parts = split_path_parts(path);
    let filename = path_parts.last().copied().unwrap_or("");
    rule_matches_with_parts(rule, &path_parts, filename, path, is_dir, case_sensitive)
}

fn rule_matches_with_parts(
    rule: &Rule,
    path_parts: &[&str],
    filename: &str,
    path: &str,
    is_dir: bool,
    case_sensitive: bool,
) -> bool {
    if rule.dir_only && !is_dir {
        return false;
    }
    match rule.pattern_type {
        PatternType::Glob => {
            if rule.is_anchored {
                match_path_parts(path_parts, &rule.pattern_parts, case_sensitive)
            } else {
                !filename.is_empty()
                    && glob_match_component(filename, &rule.pattern, case_sensitive)
            }
        }
        PatternType::Regex => {
            let target = if rule.regex_on_path { path } else { filename };
            if target.is_empty() {
                false
            } else {
                rule.regex.as_ref().map_or(false, |re| re.is_match(target))
            }
        }
    }
}

fn normalize_match_path(path: &str) -> &str {
    let mut p = path.trim_end_matches('/');
    if p.starts_with("./") {
        p = p.trim_start_matches("./");
    }
    p.trim_start_matches('/')
}

fn normalize_key_case(key: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        key.to_string()
    } else {
        key.to_lowercase()
    }
}

fn find_fuzzy_match(
    rules: &CompiledRules,
    path_parts: &[&str],
    filename: &str,
    path: &str,
    is_dir: bool,
) -> Option<usize> {
    if rules.non_ext_fuzzy_indices.is_empty() && rules.ext_rule_index.is_empty() {
        return None;
    }

    let ext_key = extract_extension_key(filename, rules.case_sensitive);
    let ext_indices = ext_key
        .as_ref()
        .and_then(|key| rules.ext_rule_index.get(key));

    let mut i = rules.non_ext_fuzzy_indices.len();
    let mut j = ext_indices.map(|v| v.len()).unwrap_or(0);

    while i > 0 || j > 0 {
        let next_from_non_ext = if i > 0 {
            Some(rules.non_ext_fuzzy_indices[i - 1])
        } else {
            None
        };
        let next_from_ext = if j > 0 {
            ext_indices.map(|v| v[j - 1])
        } else {
            None
        };

        let rule_idx = match (next_from_non_ext, next_from_ext) {
            (Some(ne), Some(ex)) => {
                if ne >= ex {
                    i -= 1;
                    ne
                } else {
                    j -= 1;
                    ex
                }
            }
            (Some(ne), None) => {
                i -= 1;
                ne
            }
            (None, Some(ex)) => {
                j -= 1;
                ex
            }
            (None, None) => break,
        };

        let rule = &rules.rules[rule_idx];
        if rule_matches_with_parts(
            rule,
            path_parts,
            filename,
            path,
            is_dir,
            rules.case_sensitive,
        ) {
            return Some(rule_idx);
        }
    }

    None
}

fn extract_extension_key(filename: &str, case_sensitive: bool) -> Option<String> {
    let dot = filename.rfind('.')?;
    if dot + 1 >= filename.len() {
        return None;
    }
    let ext = &filename[dot + 1..];
    if ext.is_empty() {
        return None;
    }
    let key = if case_sensitive {
        ext.to_string()
    } else {
        ext.to_lowercase()
    };
    Some(key)
}
