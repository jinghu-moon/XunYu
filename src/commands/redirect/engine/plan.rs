use crate::config::{RedirectProfile, RedirectUnmatched};
use crate::windows::safety::ensure_safe_target;

use super::super::matcher::{match_file, match_path};
use super::super::plan::{ConflictInfo, PlanItem, PlanKind, fingerprint_path};
use super::path::canonical_or_lexical;
use super::scan::{EngineIgnore, collect_paths_recursive, collect_paths_top};
use super::template::{age_matches, resolve_dest_dir};
use super::types::RedirectResult;

use std::path::Path;

pub(crate) struct PlanOutput {
    pub(crate) results: Vec<RedirectResult>,
    pub(crate) items: Vec<PlanItem>,
}

pub(crate) fn plan_redirect(source: &Path, profile: &RedirectProfile, copy: bool) -> PlanOutput {
    let mut results = Vec::new();
    let mut items = Vec::new();

    let source_abs = canonical_or_lexical(source);
    let ignore = EngineIgnore::new(&source_abs, profile);
    let paths = if profile.recursive {
        collect_paths_recursive(&source_abs, profile.max_depth.max(1) as usize, &ignore)
    } else {
        match collect_paths_top(&source_abs) {
            Ok(v) => v,
            Err(e) => {
                results.push(RedirectResult {
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
        plan_one_path(
            &source_abs,
            profile,
            copy,
            &src_path,
            &mut results,
            &mut items,
        );
    }

    PlanOutput { results, items }
}

fn plan_one_path(
    source_abs: &Path,
    profile: &RedirectProfile,
    copy: bool,
    src_path: &Path,
    results: &mut Vec<RedirectResult>,
    items: &mut Vec<PlanItem>,
) {
    let file_name = src_path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    if src_path.is_dir() {
        let Some(file_name) = file_name else {
            return;
        };
        if let Some(rule) = match_file(&file_name, &profile.rules) {
            let dest_dir = resolve_dest_dir(source_abs, &rule.dest, src_path);
            results.push(RedirectResult {
                action: "skip".to_string(),
                src: src_path.to_string_lossy().to_string(),
                dst: dest_dir.to_string_lossy().to_string(),
                rule: rule.name.clone(),
                result: "skipped".to_string(),
                reason: "directory_unsupported".to_string(),
            });
        }
        return;
    }
    if !src_path.is_file() {
        return;
    }

    let Some(file_name) = file_name else {
        return;
    };

    let (rule_name, dest_dir) = if let Some(rule) = match_path(src_path, &profile.rules) {
        (
            rule.name.clone(),
            resolve_dest_dir(source_abs, &rule.dest, src_path),
        )
    } else {
        match &profile.unmatched {
            RedirectUnmatched::Skip => {
                results.push(RedirectResult {
                    action: "skip".to_string(),
                    src: src_path.to_string_lossy().to_string(),
                    dst: "".to_string(),
                    rule: "".to_string(),
                    result: "skipped".to_string(),
                    reason: "unmatched".to_string(),
                });
                return;
            }
            RedirectUnmatched::Archive { age_expr, dest } => {
                if !age_matches(src_path, age_expr) {
                    results.push(RedirectResult {
                        action: "skip".to_string(),
                        src: src_path.to_string_lossy().to_string(),
                        dst: "".to_string(),
                        rule: "".to_string(),
                        result: "skipped".to_string(),
                        reason: "unmatched".to_string(),
                    });
                    return;
                }
                (
                    "(unmatched)".to_string(),
                    resolve_dest_dir(source_abs, dest, src_path),
                )
            }
        }
    };

    if let Err(msg) = ensure_safe_target(&dest_dir) {
        let reason = format!("unsafe_dest:{msg}");
        results.push(RedirectResult {
            action: "skip".to_string(),
            src: src_path.to_string_lossy().to_string(),
            dst: dest_dir.to_string_lossy().to_string(),
            rule: rule_name.clone(),
            result: "skipped".to_string(),
            reason,
        });
        return;
    }

    #[cfg(feature = "protect")]
    {
        let rules = &crate::config::load_config().protect.rules;
        if let Some(_rule) = crate::protect::is_protected(rules, src_path, "move") {
            results.push(RedirectResult {
                action: "skip".to_string(),
                src: src_path.to_string_lossy().to_string(),
                dst: dest_dir.to_string_lossy().to_string(),
                rule: rule_name.clone(),
                result: "skipped".to_string(),
                reason: "protected".to_string(),
            });
            return;
        }
    }

    let dst = dest_dir.join(&file_name);
    let conflict = if dst.exists() {
        match (fingerprint_path(&dst), fingerprint_path(src_path)) {
            (Some(existing), Some(incoming)) => Some(ConflictInfo { existing, incoming }),
            _ => None,
        }
    } else {
        None
    };

    let kind = if copy { PlanKind::Copy } else { PlanKind::Move };
    items.push(PlanItem {
        kind,
        src: src_path.to_string_lossy().to_string(),
        dst: dst.to_string_lossy().to_string(),
        rule: rule_name.clone(),
        src_fp: fingerprint_path(src_path),
        conflict,
        conflict_action: None,
    });

    results.push(RedirectResult {
        action: if copy { "copy" } else { "move" }.to_string(),
        src: src_path.to_string_lossy().to_string(),
        dst: dst.to_string_lossy().to_string(),
        rule: rule_name,
        result: "dry_run".to_string(),
        reason: "".to_string(),
    });
}
