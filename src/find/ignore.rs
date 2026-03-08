use std::path::Path;

use crate::util::{IgnorePatterns, matches_patterns, read_ignore_file};

pub(crate) struct IgnoreSet {
    patterns: IgnorePatterns,
}

impl IgnoreSet {
    pub(crate) fn new(base: &Path) -> Self {
        let ignore_file = base.join(".xunignore");
        let patterns = read_ignore_file(&ignore_file);
        Self { patterns }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.patterns.exclude.is_empty() && self.patterns.include.is_empty()
    }

    pub(crate) fn should_ignore(&self, rel_norm: &str, name_lower: &str, is_dir: bool) -> bool {
        let excluded = matches_patterns(rel_norm, name_lower, &self.patterns.exclude, is_dir);
        if !excluded {
            return false;
        }
        let included = matches_patterns(rel_norm, name_lower, &self.patterns.include, is_dir);
        !included
    }
}
