use std::ffi::OsStr;
use std::path::PathBuf;

use indexmap::IndexSet;

pub(crate) mod parallel;
mod policy;
pub(crate) mod string_check;
pub(crate) mod winapi;

pub use policy::{PathIssue, PathIssueKind, PathKind, PathPolicy, PathValidationResult};

pub fn validate_paths<I, P>(inputs: I, policy: &PathPolicy) -> PathValidationResult
where
    I: IntoIterator<Item = P>,
    P: AsRef<OsStr>,
{
    let raw_inputs: Vec<String> = inputs
        .into_iter()
        .map(|input| input.as_ref().to_string_lossy().into_owned())
        .collect();
    parallel::validate_paths(raw_inputs, policy)
}

pub(crate) fn validate_paths_serial(raw_inputs: Vec<String>, policy: &PathPolicy) -> PathValidationResult {
    let (inputs, deduped) = dedupe_inputs(raw_inputs);
    let mut out = PathValidationResult::default();
    out.deduped = deduped;

    for raw in inputs {
        let mut check_policy = policy.clone();
        if policy.expand_env && raw.contains('%') {
            check_policy.allow_relative = true;
        }
        if let Some(kind) = string_check::check_string(&raw, &check_policy) {
            out.issues.push(build_issue(&raw, kind));
            continue;
        }

        if !policy.expand_env && raw.contains('%') {
            out.issues
                .push(build_issue(&raw, PathIssueKind::EnvVarNotAllowed));
            continue;
        }

        let mut current = raw.clone();
        if policy.expand_env && raw.contains('%') {
            match winapi::expand_env(&raw) {
                Ok(expanded) => {
                    current = expanded;
                    if let Some(kind) = string_check::check_string(&current, policy) {
                        out.issues.push(build_issue(&raw, kind));
                        continue;
                    }
                }
                Err(kind) => {
                    out.issues.push(build_issue(&raw, kind));
                    continue;
                }
            }
        }

        let mut kind = string_check::detect_kind(&current);
        if matches!(kind, PathKind::Relative) {
            if !policy.allow_relative {
                out.issues
                    .push(build_issue(&raw, PathIssueKind::RelativeNotAllowed));
                continue;
            }
            let base = match policy.cwd_snapshot.clone().or_else(current_dir_safe) {
                Some(value) => value,
                None => {
                    out.issues.push(build_issue(&raw, PathIssueKind::IoError));
                    continue;
                }
            };
            let joined = base.join(&current);
            let full = match winapi::get_full_path(&joined) {
                Ok(path) => path,
                Err(kind) => {
                    out.issues.push(build_issue(&raw, kind));
                    continue;
                }
            };
            current = full.to_string_lossy().to_string();
            kind = string_check::detect_kind(&current);
            if matches!(kind, PathKind::Relative) {
                out.issues
                    .push(build_issue(&raw, PathIssueKind::RelativeNotAllowed));
                continue;
            }
            if matches!(kind, PathKind::DriveRelative) {
                out.issues
                    .push(build_issue(&raw, PathIssueKind::DriveRelativeNotAllowed));
                continue;
            }
        }

        let path = PathBuf::from(&current);
        if policy.must_exist {
            match winapi::probe(&path) {
                Ok(attr) => {
                    if !policy.allow_reparse && winapi::is_reparse_point(attr) {
                        out.issues.push(build_issue(&raw, PathIssueKind::ReparsePoint));
                        continue;
                    }
                }
                Err(kind) => {
                    out.issues.push(build_issue(&raw, kind));
                    continue;
                }
            }
        }

        if policy.safety_check {
            if crate::windows::safety::ensure_safe_target(&path).is_err() {
                out.issues.push(build_issue(&raw, PathIssueKind::AccessDenied));
                continue;
            }
        }

        out.ok.push(path);
    }

    out
}

pub(crate) fn dedupe_inputs(raw_inputs: Vec<String>) -> (Vec<String>, usize) {
    let mut seen: IndexSet<String> = IndexSet::new();
    let mut out: Vec<String> = Vec::new();
    let mut deduped = 0usize;

    for raw in raw_inputs {
        let key = normalize_for_dedupe(&raw);
        if !seen.insert(key) {
            deduped += 1;
            continue;
        }
        out.push(raw);
    }

    (out, deduped)
}

fn normalize_for_dedupe(raw: &str) -> String {
    if raw.is_ascii() {
        let mut out = String::with_capacity(raw.len());
        for b in raw.bytes() {
            let b = if b == b'/' { b'\\' } else { b };
            out.push((b as char).to_ascii_lowercase());
        }
        trim_trailing_backslash(&mut out);
        return out;
    }

    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        let ch = if ch == '/' { '\\' } else { ch };
        if ch.is_ascii() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    trim_trailing_backslash(&mut out);
    out
}

fn trim_trailing_backslash(raw: &mut String) {
    while raw.ends_with('\\') && !is_drive_root(raw) && !is_unc_root(raw) {
        raw.pop();
    }
}

fn is_drive_root(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes.len() == 3
        && bytes[1] == b':'
        && bytes[2] == b'\\'
        && bytes[0].is_ascii_alphabetic()
}

fn is_unc_root(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    if bytes.len() < 5 || bytes[0] != b'\\' || bytes[1] != b'\\' {
        return false;
    }
    let mut sep_count = 0usize;
    for (idx, &b) in bytes.iter().enumerate().skip(2) {
        if b == b'\\' {
            sep_count += 1;
            if sep_count == 2 {
                return idx + 1 == bytes.len();
            }
        }
    }
    false
}

pub(crate) fn current_dir_safe() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

pub(crate) fn build_issue(raw: &str, kind: PathIssueKind) -> PathIssue {
    PathIssue {
        raw: raw.to_string(),
        kind,
        detail: detail_for(kind),
    }
}

fn detail_for(kind: PathIssueKind) -> &'static str {
    match kind {
        PathIssueKind::Empty => "Path is empty.",
        PathIssueKind::InvalidChar => "Path contains invalid characters.",
        PathIssueKind::ReservedName => "Path uses a reserved device name.",
        PathIssueKind::TrailingDotSpace => "Path component ends with a dot or space.",
        PathIssueKind::TooLong => "Path exceeds maximum length.",
        PathIssueKind::RelativeNotAllowed => "Relative paths are not allowed.",
        PathIssueKind::DriveRelativeNotAllowed => "Drive-relative paths are not allowed.",
        PathIssueKind::TraversalDetected => "Path traversal detected.",
        PathIssueKind::NotFound => "Path not found.",
        PathIssueKind::AccessDenied => "Access denied.",
        PathIssueKind::ReparsePoint => "Reparse points are not allowed.",
        PathIssueKind::AdsNotAllowed => "Alternate data streams are not allowed.",
        PathIssueKind::DeviceNamespaceNotAllowed => "Device namespace paths are not allowed.",
        PathIssueKind::NtNamespaceNotAllowed => "NT namespace paths are not allowed.",
        PathIssueKind::VolumeGuidNotAllowed => "Volume GUID paths are not allowed.",
        PathIssueKind::EnvVarNotAllowed => "Environment variables are not allowed.",
        PathIssueKind::NetworkPathNotFound => "Network path not found.",
        PathIssueKind::SharingViolation => "Sharing violation.",
        PathIssueKind::SymlinkLoop => "Symlink loop detected.",
        PathIssueKind::IoError => "I/O error.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_preserves_order() {
        let mut policy = PathPolicy::for_output();
        policy.allow_relative = true;
        let result = validate_paths(vec!["B", "a", "b", "A"], &policy);
        assert_eq!(result.ok.len(), 2);
        assert!(result.ok[0].ends_with("B"));
        assert!(result.ok[1].ends_with("a"));
        assert_eq!(result.deduped, 2);
    }

    #[test]
    fn relative_blocked_when_not_allowed() {
        let mut policy = PathPolicy::for_read();
        policy.allow_relative = false;
        let result = validate_paths(vec!["rel\\file"], &policy);
        assert_eq!(result.ok.len(), 0);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].kind, PathIssueKind::RelativeNotAllowed);
    }

    #[test]
    fn issues_and_ok_split() {
        let mut policy = PathPolicy::for_output();
        policy.allow_relative = true;
        let result = validate_paths(vec!["good.txt", "bad|name.txt"], &policy);
        assert_eq!(result.ok.len(), 1);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].kind, PathIssueKind::InvalidChar);
    }

    #[test]
    fn issue_kind_details_defined() {
        let kinds = [
            PathIssueKind::Empty,
            PathIssueKind::InvalidChar,
            PathIssueKind::ReservedName,
            PathIssueKind::TrailingDotSpace,
            PathIssueKind::TooLong,
            PathIssueKind::RelativeNotAllowed,
            PathIssueKind::DriveRelativeNotAllowed,
            PathIssueKind::TraversalDetected,
            PathIssueKind::NotFound,
            PathIssueKind::AccessDenied,
            PathIssueKind::ReparsePoint,
            PathIssueKind::AdsNotAllowed,
            PathIssueKind::DeviceNamespaceNotAllowed,
            PathIssueKind::NtNamespaceNotAllowed,
            PathIssueKind::VolumeGuidNotAllowed,
            PathIssueKind::EnvVarNotAllowed,
            PathIssueKind::NetworkPathNotFound,
            PathIssueKind::SharingViolation,
            PathIssueKind::SymlinkLoop,
            PathIssueKind::IoError,
        ];
        for kind in kinds {
            let issue = build_issue("x", kind);
            assert!(!issue.detail.is_empty(), "missing detail for {kind:?}");
        }
    }

    #[test]
    fn env_var_not_allowed_is_reported() {
        let mut policy = PathPolicy::for_output();
        policy.allow_relative = true;
        let result = validate_paths(vec!["%TEMP%\\file.txt"], &policy);
        assert_eq!(result.ok.len(), 0);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].kind, PathIssueKind::EnvVarNotAllowed);
    }
}
