mod path_norm;
mod rule_match;
mod score;

use std::path::Path;

use crate::config::RedirectRule;

pub(crate) use score::SizeOp;

#[derive(Debug, Clone)]
pub(crate) struct ExplainDetail {
    pub(crate) matched: bool,
    pub(crate) summary: String,
}

pub(crate) fn parse_size_expr(raw: &str) -> Result<(SizeOp, u64), String> {
    score::parse_size_expr(raw)
}

pub(crate) fn parse_age_expr(raw: &str) -> Result<(SizeOp, u64), String> {
    score::parse_age_expr(raw)
}

pub(crate) fn explain_rule_pure(file_name: &str, rule: &RedirectRule) -> ExplainDetail {
    rule_match::explain_rule_pure(file_name, rule)
}

pub(crate) fn match_path<'a>(
    src_path: &Path,
    rules: &'a [RedirectRule],
) -> Option<&'a RedirectRule> {
    rule_match::match_path(src_path, rules)
}

pub(crate) fn match_file<'a>(
    file_name: &str,
    rules: &'a [RedirectRule],
) -> Option<&'a RedirectRule> {
    rule_match::match_file(file_name, rules)
}

pub(crate) fn any_rule_matches_name_only(file_name: &str, rules: &[RedirectRule]) -> bool {
    rule_match::any_rule_matches_name_only(file_name, rules)
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
