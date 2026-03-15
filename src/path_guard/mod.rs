use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use indexmap::IndexSet;

pub(crate) mod parallel;
mod policy;
pub(crate) mod string_check;
pub(crate) mod winapi;

pub use policy::{PathInfo, PathIssue, PathIssueKind, PathKind, PathPolicy, PathValidationResult};

const SLASH: u16 = b'\\' as u16;
const FSLASH: u16 = b'/' as u16;
const COLON: u16 = b':' as u16;
const PERCENT: u16 = b'%' as u16;

thread_local! {
    static CHECK_BUF: RefCell<Vec<u16>> = RefCell::new(Vec::with_capacity(512));
}

pub fn validate_paths<I, P>(inputs: I, policy: &PathPolicy) -> PathValidationResult
where
    I: IntoIterator<Item = P>,
    P: AsRef<OsStr>,
{
    let raw_inputs: Vec<OsString> = inputs
        .into_iter()
        .map(|input| input.as_ref().to_os_string())
        .collect();
    parallel::validate_paths(raw_inputs, policy)
}

pub fn validate_paths_with_info<I, P>(inputs: I, policy: &PathPolicy) -> (Vec<PathInfo>, Vec<PathIssue>)
where
    I: IntoIterator<Item = P>,
    P: AsRef<OsStr>,
{
    let raw_inputs: Vec<OsString> = inputs
        .into_iter()
        .map(|input| input.as_ref().to_os_string())
        .collect();
    let (inputs, _) = dedupe_inputs(raw_inputs);

    let mut infos = Vec::new();
    let mut issues = Vec::new();
    let mut scratch = Vec::with_capacity(512);
    let cwd_snapshot = if policy.allow_relative {
        policy.cwd_snapshot.clone().or_else(current_dir_safe)
    } else {
        None
    };

    for raw in inputs {
        match validate_single_inner(raw.as_os_str(), policy, cwd_snapshot.as_ref(), &mut scratch, true) {
            Ok(info) => infos.push(info),
            Err(issue) => issues.push(issue),
        }
    }

    (infos, issues)
}

pub fn validate_single(
    raw: &OsStr,
    policy: &PathPolicy,
    scratch: &mut Vec<u16>,
) -> Result<PathInfo, PathIssue> {
    let cwd_snapshot = if policy.allow_relative {
        policy.cwd_snapshot.clone().or_else(current_dir_safe)
    } else {
        None
    };
    validate_single_inner(raw, policy, cwd_snapshot.as_ref(), scratch, true)
}

pub(crate) fn validate_paths_serial(
    raw_inputs: Vec<OsString>,
    policy: &PathPolicy,
) -> PathValidationResult {
    let (inputs, deduped) = dedupe_inputs(raw_inputs);
    let mut out = PathValidationResult::default();
    out.deduped = deduped;

    let cwd_snapshot = if policy.allow_relative {
        policy.cwd_snapshot.clone().or_else(current_dir_safe)
    } else {
        None
    };

    for raw in inputs {
        let result = with_check_buf(|scratch| {
            validate_string_stage(raw.as_os_str(), policy, cwd_snapshot.as_ref(), scratch)
        });

        let (path, _) = match result {
            Ok(value) => value,
            Err(kind) => {
                out.issues.push(build_issue(raw.as_os_str(), kind));
                continue;
            }
        };

        if policy.must_exist {
            match winapi::probe(&path) {
                Ok(attr) => {
                    if !policy.allow_reparse && winapi::is_reparse_point(attr) {
                        out.issues.push(build_issue(raw.as_os_str(), PathIssueKind::ReparsePoint));
                        continue;
                    }
                }
                Err(kind) => {
                    out.issues.push(build_issue(raw.as_os_str(), kind));
                    continue;
                }
            }
        }

        if policy.safety_check {
            if crate::windows::safety::ensure_safe_target(&path).is_err() {
                out.issues.push(build_issue(raw.as_os_str(), PathIssueKind::AccessDenied));
                continue;
            }
        }

        out.ok.push(path);
    }

    out
}

fn validate_single_inner(
    raw: &OsStr,
    policy: &PathPolicy,
    cwd_snapshot: Option<&PathBuf>,
    scratch: &mut Vec<u16>,
    probe_any: bool,
) -> Result<PathInfo, PathIssue> {
    let (path, kind) = validate_string_stage(raw, policy, cwd_snapshot, scratch)
        .map_err(|kind| build_issue(raw, kind))?;

    let mut info = PathInfo {
        path,
        kind,
        canonical: None,
        is_reparse_point: false,
        is_directory: None,
        existence_probe: None,
    };

    if probe_any {
        match winapi::probe_ex(&info.path) {
            Ok(data) => {
                let attr = data.dwFileAttributes;
                info.is_reparse_point = winapi::is_reparse_point(attr);
                info.is_directory = Some(winapi::is_directory(attr));
            }
            Err(kind) => {
                if policy.must_exist {
                    return Err(build_issue(raw, kind));
                }
                info.existence_probe = Some(kind);
            }
        }
    } else if policy.must_exist {
        match winapi::probe(&info.path) {
            Ok(attr) => {
                info.is_reparse_point = winapi::is_reparse_point(attr);
                info.is_directory = Some(winapi::is_directory(attr));
            }
            Err(kind) => return Err(build_issue(raw, kind)),
        }
    }

    if policy.must_exist && !policy.allow_reparse && info.is_reparse_point {
        return Err(build_issue(raw, PathIssueKind::ReparsePoint));
    }

    if policy.must_exist && policy.allow_reparse {
        if let Ok(handle) = winapi::open_path_with_policy(&info.path, policy) {
            if let Ok(final_path) = winapi::get_final_path(&handle) {
                info.canonical = Some(final_path);
            }
        }
    }

    if policy.safety_check {
        if crate::windows::safety::ensure_safe_target(&info.path).is_err() {
            return Err(build_issue(raw, PathIssueKind::AccessDenied));
        }
    }

    Ok(info)
}

pub(crate) fn validate_string_stage(
    raw: &OsStr,
    policy: &PathPolicy,
    cwd_snapshot: Option<&PathBuf>,
    scratch: &mut Vec<u16>,
) -> Result<(PathBuf, PathKind), PathIssueKind> {
    fill_wide(scratch, raw);
    let has_env = contains_unit(scratch, PERCENT);

    let mut check_policy = policy.clone();
    if policy.expand_env && has_env {
        check_policy.allow_relative = true;
    }

    if let Some(kind) = string_check::check_string(raw, scratch, &check_policy) {
        return Err(kind);
    }

    if !policy.expand_env && has_env {
        return Err(PathIssueKind::EnvVarNotAllowed);
    }

    let mut current = raw.to_os_string();
    if policy.expand_env && has_env {
        current = winapi::expand_env(raw)?;
        fill_wide(scratch, &current);
        if let Some(kind) = string_check::check_string(&current, scratch, policy) {
            return Err(kind);
        }
    }

    let mut kind = string_check::detect_kind(scratch);
    if matches!(kind, PathKind::Relative) {
        if !policy.allow_relative {
            return Err(PathIssueKind::RelativeNotAllowed);
        }
        let base = match cwd_snapshot {
            Some(value) => value,
            None => return Err(PathIssueKind::IoError),
        };
        let joined = base.join(&current);
        let full = winapi::get_full_path(&joined)?;
        current = full.into_os_string();
        fill_wide(scratch, &current);
        kind = string_check::detect_kind(scratch);
        if matches!(kind, PathKind::Relative) {
            return Err(PathIssueKind::RelativeNotAllowed);
        }
        if matches!(kind, PathKind::DriveRelative) {
            return Err(PathIssueKind::DriveRelativeNotAllowed);
        }
    }

    Ok((PathBuf::from(current), kind))
}

pub(crate) fn dedupe_inputs(raw_inputs: Vec<OsString>) -> (Vec<OsString>, usize) {
    let mut seen: IndexSet<Vec<u16>> = IndexSet::new();
    let mut out: Vec<OsString> = Vec::new();
    let mut deduped = 0usize;

    for raw in raw_inputs {
        let key = with_check_buf(|scratch| {
            fill_wide(scratch, &raw);
            normalize_for_dedupe(scratch)
        });
        if !seen.insert(key) {
            deduped += 1;
            continue;
        }
        out.push(raw);
    }

    (out, deduped)
}

fn normalize_for_dedupe(units: &[u16]) -> Vec<u16> {
    let mut out = Vec::with_capacity(units.len());
    for &unit in units {
        let unit = if unit == FSLASH { SLASH } else { unit };
        let unit = if (b'A' as u16..=b'Z' as u16).contains(&unit) {
            unit + 32
        } else {
            unit
        };
        out.push(unit);
    }
    trim_trailing_backslash(&mut out);
    out
}

fn trim_trailing_backslash(raw: &mut Vec<u16>) {
    while raw.last() == Some(&SLASH) && !is_drive_root(raw) && !is_unc_root(raw) {
        raw.pop();
    }
}

fn is_drive_root(raw: &[u16]) -> bool {
    raw.len() == 3 && raw[1] == COLON && raw[2] == SLASH && is_ascii_letter(raw[0])
}

fn is_unc_root(raw: &[u16]) -> bool {
    if raw.len() < 5 || raw[0] != SLASH || raw[1] != SLASH {
        return false;
    }
    let mut sep_count = 0usize;
    for (idx, &unit) in raw.iter().enumerate().skip(2) {
        if unit == SLASH {
            sep_count += 1;
            if sep_count == 2 {
                return idx + 1 == raw.len();
            }
        }
    }
    false
}

fn is_ascii_letter(unit: u16) -> bool {
    (b'a' as u16..=b'z' as u16).contains(&unit)
        || (b'A' as u16..=b'Z' as u16).contains(&unit)
}

pub(crate) fn current_dir_safe() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

pub(crate) fn build_issue(raw: &OsStr, kind: PathIssueKind) -> PathIssue {
    PathIssue {
        raw: raw.to_string_lossy().into_owned(),
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

fn contains_unit(units: &[u16], target: u16) -> bool {
    units.iter().any(|&unit| unit == target)
}

pub(crate) fn fill_wide(buf: &mut Vec<u16>, raw: &OsStr) {
    buf.clear();
    buf.extend(raw.encode_wide());
}

pub(crate) fn with_check_buf<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vec<u16>) -> R,
{
    CHECK_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        f(&mut buf)
    })
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
            let issue = build_issue(OsStr::new("x"), kind);
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
