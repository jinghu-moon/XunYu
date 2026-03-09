use std::path::{Path, PathBuf};

use crate::config::RedirectProfile;
use crate::util::{IgnorePatterns, matches_patterns, normalize_glob_path, read_ignore_file};

use super::super::path_utils::dest_root_path;
use super::status::WATCH_STATUS_FILE;

pub(super) fn resolve_dest_dirs(source: &Path, profile: &RedirectProfile) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = profile
        .rules
        .iter()
        .map(|r| dest_root_path(source, &r.dest))
        .collect();

    if let crate::config::RedirectUnmatched::Archive { dest, .. } = &profile.unmatched {
        out.push(dest_root_path(source, dest));
    }

    out.sort();
    out.dedup();
    out
}

pub(super) fn build_ignore_set(source: &Path, profile: &RedirectProfile) -> IgnoreSet {
    let mut patterns = IgnorePatterns::default();
    let ignore_file = source.join(".xunignore");
    let ig = read_ignore_file(&ignore_file);
    patterns.exclude.extend(ig.exclude);
    patterns.include.extend(ig.include);

    let mut dest_abs: Vec<PathBuf> = resolve_dest_dirs(source, profile);
    dest_abs.sort();
    dest_abs.dedup();

    let protect_prefixes: Vec<PathBuf> = {
        #[cfg(feature = "protect")]
        {
            let cfg = crate::config::load_config();
            cfg.protect
                .rules
                .iter()
                .map(|r| PathBuf::from(&r.path))
                .collect()
        }
        #[cfg(not(feature = "protect"))]
        {
            Vec::new()
        }
    };

    IgnoreSet {
        patterns,
        protect_prefixes: protect_prefixes
            .into_iter()
            .map(|p| p.canonicalize().unwrap_or(p))
            .collect(),
    }
}

pub(super) struct IgnoreSet {
    patterns: IgnorePatterns,
    protect_prefixes: Vec<PathBuf>,
}

impl IgnoreSet {
    pub(super) fn protect_prefixes(&self) -> &[PathBuf] {
        &self.protect_prefixes
    }
}

pub(super) fn should_ignore(
    source: &Path,
    dest_dirs: &[PathBuf],
    ignore: &IgnoreSet,
    path: &Path,
) -> bool {
    if dest_dirs.iter().any(|d| path.starts_with(d)) {
        return true;
    }
    if ignore
        .protect_prefixes
        .iter()
        .any(|p| !p.as_os_str().is_empty() && path.starts_with(p))
    {
        return true;
    }

    let rel = match path.strip_prefix(source) {
        Ok(r) => r,
        Err(_) => return false,
    };
    if rel
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case(WATCH_STATUS_FILE))
        .unwrap_or(false)
    {
        return true;
    }
    let rel_norm = normalize_glob_path(&rel.to_string_lossy());
    let name_lower = rel
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let is_dir = path.is_dir();
    let excluded = matches_patterns(&rel_norm, &name_lower, &ignore.patterns.exclude, is_dir);
    if excluded {
        let included = matches_patterns(&rel_norm, &name_lower, &ignore.patterns.include, is_dir);
        return !included;
    }
    false
}
