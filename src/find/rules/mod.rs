use std::collections::HashMap;
use std::fs;

use regex::{Regex, RegexBuilder};

use crate::cli::FindCmd;
use crate::output::{CliError, CliResult};

use super::glob::{has_glob_wildcard, split_pattern_parts, unescape_glob_literal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleKind {
    Include,
    Exclude,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatternType {
    Glob,
    Regex,
}

#[derive(Clone, Debug)]
pub(crate) struct Rule {
    pub(crate) idx: usize,
    pub(crate) kind: RuleKind,
    pub(crate) pattern_type: PatternType,
    pub(crate) pattern: String,
    pub(crate) pattern_parts: Vec<String>,
    pub(crate) is_anchored: bool,
    pub(crate) is_exact: bool,
    pub(crate) dir_only: bool,
    pub(crate) regex: Option<Regex>,
    pub(crate) regex_on_path: bool,
    pub(crate) source: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct CompiledRules {
    pub(crate) rules: Vec<Rule>,
    pub(crate) exact_by_path: HashMap<String, Vec<usize>>,
    pub(crate) exact_by_name: HashMap<String, Vec<usize>>,
    pub(crate) ext_rule_index: HashMap<String, Vec<usize>>,
    pub(crate) non_ext_fuzzy_indices: Vec<usize>,
    pub(crate) default_include: bool,
    pub(crate) case_sensitive: bool,
}

struct RuleBuilder {
    case_sensitive: bool,
    rules: Vec<Rule>,
    exact_by_path: HashMap<String, Vec<usize>>,
    exact_by_name: HashMap<String, Vec<usize>>,
    has_include: bool,
}

impl RuleBuilder {
    fn new(case_sensitive: bool) -> Self {
        Self {
            case_sensitive,
            rules: Vec::new(),
            exact_by_path: HashMap::new(),
            exact_by_name: HashMap::new(),
            has_include: false,
        }
    }

    fn push_glob(&mut self, raw: &str, kind: RuleKind, source: Option<String>) -> CliResult {
        let (kind, mut pattern) = normalize_rule_text(raw, kind, source.as_deref())?;
        let mut dir_only = false;
        if pattern.ends_with('/') {
            dir_only = true;
            pattern = pattern.trim_end_matches('/').to_string();
        }
        if pattern.is_empty() {
            return Err(rule_error("Empty glob pattern.", source.as_deref()));
        }
        let anchored =
            pattern.contains('/') || pattern.starts_with("./") || pattern.starts_with('/');
        if anchored {
            pattern = pattern
                .trim_start_matches("./")
                .trim_start_matches('/')
                .to_string();
        }
        if pattern.is_empty() {
            return Err(rule_error(
                "Empty glob pattern after normalization.",
                source.as_deref(),
            ));
        }

        let is_exact = !has_glob_wildcard(&pattern);
        let pattern_parts = if anchored {
            split_pattern_parts(&pattern)
        } else {
            Vec::new()
        };

        let idx = self.rules.len();
        let rule = Rule {
            idx,
            kind,
            pattern_type: PatternType::Glob,
            pattern: pattern.clone(),
            pattern_parts: pattern_parts.clone(),
            is_anchored: anchored,
            is_exact,
            dir_only,
            regex: None,
            regex_on_path: false,
            source,
        };
        self.rules.push(rule);
        if kind == RuleKind::Include {
            self.has_include = true;
        }
        if is_exact {
            let key = if anchored {
                let parts: Vec<String> = pattern_parts
                    .iter()
                    .map(|p| unescape_glob_literal(p))
                    .collect();
                parts.join("/")
            } else {
                unescape_glob_literal(&pattern)
            };
            let key = normalize_key_case(&key, self.case_sensitive);
            let map = if anchored {
                &mut self.exact_by_path
            } else {
                &mut self.exact_by_name
            };
            map.entry(key).or_default().push(idx);
        }
        Ok(())
    }

    fn push_regex(&mut self, raw: &str, kind: RuleKind, source: Option<String>) -> CliResult {
        let (kind, mut pattern) = normalize_rule_text(raw, kind, source.as_deref())?;
        let mut dir_only = false;
        if pattern.ends_with('/') {
            dir_only = true;
            pattern = pattern.trim_end_matches('/').to_string();
        }
        if pattern.is_empty() {
            return Err(rule_error("Empty regex pattern.", source.as_deref()));
        }
        let regex_on_path = pattern.contains('/');
        let full = format!("^(?:{})$", pattern);
        let regex = RegexBuilder::new(&full)
            .case_insensitive(!self.case_sensitive)
            .build()
            .map_err(|e| rule_error(format!("Invalid regex: {e}"), source.as_deref()))?;

        let idx = self.rules.len();
        let rule = Rule {
            idx,
            kind,
            pattern_type: PatternType::Regex,
            pattern,
            pattern_parts: Vec::new(),
            is_anchored: regex_on_path,
            is_exact: false,
            dir_only,
            regex: Some(regex),
            regex_on_path,
            source,
        };
        self.rules.push(rule);
        if kind == RuleKind::Include {
            self.has_include = true;
        }
        Ok(())
    }

    fn finish(self) -> CompiledRules {
        let default_include = !self.has_include;
        let rules = self.rules;
        let (ext_rule_index, non_ext_fuzzy_indices) =
            build_ext_rule_index(&rules, self.case_sensitive);
        CompiledRules {
            rules,
            exact_by_path: self.exact_by_path,
            exact_by_name: self.exact_by_name,
            ext_rule_index,
            non_ext_fuzzy_indices,
            default_include,
            case_sensitive: self.case_sensitive,
        }
    }
}

fn normalize_rule_text(
    raw: &str,
    kind: RuleKind,
    source: Option<&str>,
) -> CliResult<(RuleKind, String)> {
    let mut trimmed = raw.trim();
    let mut final_kind = kind;
    if trimmed.starts_with('!') {
        trimmed = trimmed[1..].trim_start();
        final_kind = RuleKind::Include;
    }
    if trimmed.is_empty() {
        return Err(rule_error("Empty rule pattern.", source));
    }
    Ok((final_kind, trimmed.to_string()))
}

fn normalize_key_case(key: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        key.to_string()
    } else {
        key.to_lowercase()
    }
}

fn rule_error(message: impl Into<String>, source: Option<&str>) -> CliError {
    let mut details = Vec::new();
    if let Some(src) = source {
        details.push(format!("Rule source: {src}"));
    }
    CliError {
        code: 2,
        message: message.into(),
        details,
    }
}

fn split_csv(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        for part in value.split(',') {
            let item = part.trim();
            if item.is_empty() {
                continue;
            }
            out.push(item.to_string());
        }
    }
    out
}

fn build_ext_rule_index(
    rules: &[Rule],
    case_sensitive: bool,
) -> (HashMap<String, Vec<usize>>, Vec<usize>) {
    let mut ext_rule_index: HashMap<String, Vec<usize>> = HashMap::new();
    let mut non_ext_fuzzy_indices = Vec::new();

    for rule in rules {
        if rule.is_exact {
            continue;
        }
        if let Some(ext) = try_extract_extension(rule, case_sensitive) {
            ext_rule_index.entry(ext).or_default().push(rule.idx);
        } else {
            non_ext_fuzzy_indices.push(rule.idx);
        }
    }

    (ext_rule_index, non_ext_fuzzy_indices)
}

fn try_extract_extension(rule: &Rule, case_sensitive: bool) -> Option<String> {
    if rule.pattern_type != PatternType::Glob || rule.is_anchored || rule.is_exact {
        return None;
    }
    let pattern = rule.pattern.as_bytes();
    if pattern.len() <= 2 || pattern[0] != b'*' || pattern[1] != b'.' {
        return None;
    }
    let ext_part = &rule.pattern[2..];
    if ext_part.is_empty() {
        return None;
    }
    if ext_part
        .as_bytes()
        .iter()
        .any(|b| matches!(b, b'*' | b'?' | b'[' | b'/' | b'\\' | b'.'))
    {
        return None;
    }
    let ext = if case_sensitive {
        ext_part.to_string()
    } else {
        ext_part.to_lowercase()
    };
    Some(ext)
}

fn normalize_extension(ext: &str) -> Option<String> {
    let trimmed = ext.trim();
    if trimmed.is_empty() {
        return None;
    }
    let trimmed = trimmed.trim_start_matches('.');
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

pub(crate) fn compile_rules(args: &FindCmd) -> CliResult<CompiledRules> {
    let mut builder = RuleBuilder::new(args.case);

    if let Some(path) = args.filter_file.as_deref() {
        let content = fs::read_to_string(path)
            .map_err(|e| CliError::new(2, format!("Failed to read filter file: {e}")))?;
        for (idx, raw_line) in content.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let source = Some(format!("filter-file:{}:{}", path, idx + 1));
            builder.push_glob(line, RuleKind::Exclude, source)?;
        }
    }

    for pat in split_csv(&args.include) {
        builder.push_glob(&pat, RuleKind::Include, Some("include".to_string()))?;
    }
    for pat in split_csv(&args.exclude) {
        builder.push_glob(&pat, RuleKind::Exclude, Some("exclude".to_string()))?;
    }
    for pat in &args.regex_include {
        builder.push_regex(pat, RuleKind::Include, Some("regex-include".to_string()))?;
    }
    for pat in &args.regex_exclude {
        builder.push_regex(pat, RuleKind::Exclude, Some("regex-exclude".to_string()))?;
    }
    for ext in split_csv(&args.extension) {
        if let Some(ext) = normalize_extension(&ext) {
            let pattern = format!("*.{}", ext);
            builder.push_glob(&pattern, RuleKind::Include, Some("extension".to_string()))?;
        }
    }
    for ext in split_csv(&args.not_extension) {
        if let Some(ext) = normalize_extension(&ext) {
            let pattern = format!("*.{}", ext);
            builder.push_glob(
                &pattern,
                RuleKind::Exclude,
                Some("not-extension".to_string()),
            )?;
        }
    }
    for name in split_csv(&args.name) {
        builder.push_glob(&name, RuleKind::Include, Some("name".to_string()))?;
    }

    Ok(builder.finish())
}

#[cfg(test)]
mod tests;
