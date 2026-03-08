use super::*;
use crate::find::matcher::determine_path_state;

fn inherited_state(rules: &CompiledRules) -> RuleKind {
    if rules.default_include {
        RuleKind::Include
    } else {
        RuleKind::Exclude
    }
}

#[test]
fn exact_rule_case_insensitive_key() {
    let mut builder = RuleBuilder::new(false);
    builder
        .push_glob("Makefile", RuleKind::Include, None)
        .unwrap();
    let rules = builder.finish();

    let decision = determine_path_state(&rules, "makefile", false, inherited_state(&rules));
    assert_eq!(decision.final_state, RuleKind::Include);
    assert!(decision.explicit);
}

#[test]
fn exact_overrides_fuzzy() {
    let mut builder = RuleBuilder::new(true);
    builder.push_glob("*.rs", RuleKind::Include, None).unwrap();
    builder
        .push_glob("foo.rs", RuleKind::Exclude, None)
        .unwrap();
    let rules = builder.finish();

    let decision = determine_path_state(&rules, "foo.rs", false, inherited_state(&rules));
    assert_eq!(decision.final_state, RuleKind::Exclude);
    assert!(decision.explicit);
}

#[test]
fn fuzzy_last_rule_wins() {
    let mut builder = RuleBuilder::new(true);
    builder.push_glob("*.rs", RuleKind::Include, None).unwrap();
    builder.push_glob("*.rs", RuleKind::Exclude, None).unwrap();
    let rules = builder.finish();

    let decision = determine_path_state(&rules, "lib.rs", false, inherited_state(&rules));
    assert_eq!(decision.final_state, RuleKind::Exclude);
    assert!(decision.explicit);
}

#[test]
fn regex_full_match_and_path_match() {
    let mut builder = RuleBuilder::new(true);
    builder
        .push_regex(r"a\.txt", RuleKind::Include, None)
        .unwrap();
    builder
        .push_regex(r"dir/.+\.txt", RuleKind::Include, None)
        .unwrap();
    let rules = builder.finish();

    let decision = determine_path_state(&rules, "a.txt", false, inherited_state(&rules));
    assert_eq!(decision.final_state, RuleKind::Include);

    let decision = determine_path_state(&rules, "xa.txt", false, inherited_state(&rules));
    assert_ne!(decision.final_state, RuleKind::Include);

    let decision = determine_path_state(&rules, "dir/a.txt", false, inherited_state(&rules));
    assert_eq!(decision.final_state, RuleKind::Include);
}
