use crate::config::RedirectProfile;
use crate::store::now_secs;

use super::path::canonical_or_lexical;
use super::process::process_one_path;
use super::scan::{EngineIgnore, collect_paths_recursive, collect_paths_top};
use super::types::{RedirectOptions, RedirectResult};

use std::path::{Path, PathBuf};

pub(crate) fn new_tx_id() -> String {
    format!("redirect_{}_{}", now_secs(), std::process::id())
}

pub(crate) fn run_redirect(
    tx: &str,
    source: &Path,
    profile: &RedirectProfile,
    opts: &RedirectOptions,
) -> Vec<RedirectResult> {
    let mut out = Vec::new();

    let source_abs = canonical_or_lexical(source);
    let ignore = EngineIgnore::new(&source_abs, profile);
    let paths = if profile.recursive {
        collect_paths_recursive(&source_abs, profile.max_depth.max(1) as usize, &ignore)
    } else {
        match collect_paths_top(&source_abs) {
            Ok(v) => v,
            Err(e) => {
                out.push(RedirectResult {
                    action: "skip".to_string(),
                    src: source_abs.to_string_lossy().to_string(),
                    dst: "".to_string(),
                    rule: "(source)".to_string(),
                    result: "failed".to_string(),
                    reason: format!("read_dir_failed:{:?}", e.kind()),
                });
                Vec::new()
            }
        }
    };
    for src_path in paths {
        process_one_path(tx, &source_abs, profile, opts, &src_path, &mut out);
    }

    out
}

pub(crate) fn run_redirect_on_paths(
    tx: &str,
    source: &Path,
    profile: &RedirectProfile,
    opts: &RedirectOptions,
    paths: &[PathBuf],
) -> Vec<RedirectResult> {
    let mut out = Vec::new();
    let source_abs = canonical_or_lexical(source);

    for p in paths {
        let src_path = canonical_or_lexical(p);
        if !src_path.starts_with(&source_abs) {
            continue;
        }
        process_one_path(tx, &source_abs, profile, opts, &src_path, &mut out);
    }

    out
}
