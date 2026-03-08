use crate::config::{RedirectProfile, RedirectUnmatched};
use crate::util::{IgnorePatterns, matches_patterns, normalize_glob_path, read_ignore_file};

use super::super::path_utils::dest_root_path;
use super::canonical_or_lexical;

use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct EngineIgnore {
    patterns: IgnorePatterns,
    protect_prefixes: Vec<PathBuf>,
    dest_roots: Vec<PathBuf>,
    source: PathBuf,
}

impl EngineIgnore {
    pub(crate) fn new(source: &Path, profile: &RedirectProfile) -> Self {
        let ignore_file = source.join(".xunignore");
        let patterns = read_ignore_file(&ignore_file);
        let dest_roots = resolve_dest_roots(source, profile);
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
        Self {
            patterns,
            protect_prefixes: protect_prefixes
                .into_iter()
                .map(|p| p.canonicalize().unwrap_or(p))
                .collect(),
            dest_roots: dest_roots
                .into_iter()
                .map(|p| p.canonicalize().unwrap_or(p))
                .collect(),
            source: source.to_path_buf(),
        }
    }

    pub(crate) fn should_ignore(&self, path: &Path, is_dir: bool) -> bool {
        if self.dest_roots.iter().any(|d| path.starts_with(d)) {
            return true;
        }
        if self
            .protect_prefixes
            .iter()
            .any(|p| !p.as_os_str().is_empty() && path.starts_with(p))
        {
            return true;
        }
        let rel = path.strip_prefix(&self.source).ok();
        let rel = rel
            .map(|p| normalize_glob_path(&p.to_string_lossy()))
            .unwrap_or_else(|| normalize_glob_path(&path.to_string_lossy()));
        let name_lower = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let excluded = matches_patterns(&rel, &name_lower, &self.patterns.exclude, is_dir);
        if !excluded {
            return false;
        }
        let included = matches_patterns(&rel, &name_lower, &self.patterns.include, is_dir);
        !included
    }
}

pub(crate) fn collect_paths_top(source_abs: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(source_abs)?;
    Ok(entries
        .flatten()
        .map(|e| canonical_or_lexical(&e.path()))
        .collect())
}

pub(crate) fn collect_paths_recursive(
    source_abs: &Path,
    max_depth: usize,
    ignore: &EngineIgnore,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<(PathBuf, usize)> = vec![(source_abs.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        let rd = match std::fs::read_dir(&dir) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for ent in rd.flatten() {
            let p = canonical_or_lexical(&ent.path());
            let is_dir = p.is_dir();
            if ignore.should_ignore(&p, is_dir) {
                continue;
            }
            if is_dir {
                if depth + 1 <= max_depth {
                    stack.push((p, depth + 1));
                }
            } else {
                out.push(p);
            }
        }
    }
    out
}

pub(crate) fn resolve_dest_roots(source: &Path, profile: &RedirectProfile) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for r in &profile.rules {
        roots.push(dest_root_path(source, &r.dest));
    }
    if let RedirectUnmatched::Archive { dest, .. } = &profile.unmatched {
        roots.push(dest_root_path(source, dest));
    }
    roots.sort();
    roots.dedup();
    roots
}
