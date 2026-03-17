use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(crate) mod parallel;
mod policy;
pub(crate) mod string_check;
pub(crate) mod winapi;

pub use policy::{PathInfo, PathIssue, PathIssueKind, PathKind, PathPolicy, PathValidationResult};

const SLASH: u16 = b'\\' as u16;
const FSLASH: u16 = b'/' as u16;
const COLON: u16 = b':' as u16;
const PERCENT: u16 = b'%' as u16;
pub(crate) const BATCH_PROBE_MIN: usize = 10;

thread_local! {
    static CHECK_BUF: RefCell<Vec<u16>> = RefCell::new(Vec::with_capacity(512));
}

pub fn validate_paths<I, P>(inputs: I, policy: &PathPolicy) -> PathValidationResult
where
    I: IntoIterator<Item = P>,
    P: AsRef<OsStr>,
{
    let trace_start = trace_total_start();
    let trace_before = trace_snapshot_if_enabled();
    let iter = inputs.into_iter();
    let (lower, upper) = iter.size_hint();
    let mut raw_inputs: Vec<OsString> = Vec::with_capacity(upper.unwrap_or(lower));
    for input in iter {
        raw_inputs.push(input.as_ref().to_os_string());
    }
    let total_paths = raw_inputs.len();
    let result = parallel::validate_paths(raw_inputs, policy);
    trace_report(
        "validate_paths",
        total_paths,
        result.deduped,
        result.ok.len(),
        result.issues.len(),
        trace_start,
        trace_before,
    );
    result
}

pub fn validate_paths_owned<I, P>(inputs: I, policy: &PathPolicy) -> PathValidationResult
where
    I: IntoIterator<Item = P>,
    P: Into<OsString>,
{
    let trace_start = trace_total_start();
    let trace_before = trace_snapshot_if_enabled();
    let iter = inputs.into_iter();
    let (lower, upper) = iter.size_hint();
    let mut raw_inputs: Vec<OsString> = Vec::with_capacity(upper.unwrap_or(lower));
    for input in iter {
        raw_inputs.push(input.into());
    }
    let total_paths = raw_inputs.len();
    let result = parallel::validate_paths(raw_inputs, policy);
    trace_report(
        "validate_paths_owned",
        total_paths,
        result.deduped,
        result.ok.len(),
        result.issues.len(),
        trace_start,
        trace_before,
    );
    result
}

pub fn validate_paths_with_info<I, P>(inputs: I, policy: &PathPolicy) -> (Vec<PathInfo>, Vec<PathIssue>)
where
    I: IntoIterator<Item = P>,
    P: AsRef<OsStr>,
{
    let trace_start = trace_total_start();
    let trace_before = trace_snapshot_if_enabled();
    let iter = inputs.into_iter();
    let (lower, upper) = iter.size_hint();
    let mut raw_inputs: Vec<OsString> = Vec::with_capacity(upper.unwrap_or(lower));
    for input in iter {
        raw_inputs.push(input.as_ref().to_os_string());
    }
    let (inputs, deduped) = dedupe_inputs(raw_inputs);

    let mut infos = Vec::with_capacity(inputs.len());
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

    trace_report(
        "validate_paths_with_info",
        infos.len() + issues.len(),
        deduped,
        infos.len(),
        issues.len(),
        trace_start,
        trace_before,
    );
    (infos, issues)
}

pub fn validate_paths_with_info_owned<I, P>(
    inputs: I,
    policy: &PathPolicy,
) -> (Vec<PathInfo>, Vec<PathIssue>)
where
    I: IntoIterator<Item = P>,
    P: Into<OsString>,
{
    let trace_start = trace_total_start();
    let trace_before = trace_snapshot_if_enabled();
    let iter = inputs.into_iter();
    let (lower, upper) = iter.size_hint();
    let mut raw_inputs: Vec<OsString> = Vec::with_capacity(upper.unwrap_or(lower));
    for input in iter {
        raw_inputs.push(input.into());
    }
    let (inputs, deduped) = dedupe_inputs(raw_inputs);

    let mut infos = Vec::with_capacity(inputs.len());
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

    trace_report(
        "validate_paths_with_info_owned",
        infos.len() + issues.len(),
        deduped,
        infos.len(),
        issues.len(),
        trace_start,
        trace_before,
    );
    (infos, issues)
}

pub fn validate_single(
    raw: &OsStr,
    policy: &PathPolicy,
    scratch: &mut Vec<u16>,
) -> Result<PathInfo, PathIssue> {
    let trace_start = trace_total_start();
    let trace_before = trace_snapshot_if_enabled();
    let cwd_snapshot = if policy.allow_relative {
        policy.cwd_snapshot.clone().or_else(current_dir_safe)
    } else {
        None
    };
    let result = validate_single_inner(raw, policy, cwd_snapshot.as_ref(), scratch, true);
    trace_report(
        "validate_single",
        1,
        0,
        if result.is_ok() { 1 } else { 0 },
        if result.is_err() { 1 } else { 0 },
        trace_start,
        trace_before,
    );
    result
}

pub(crate) fn validate_paths_serial(
    raw_inputs: Vec<OsString>,
    policy: &PathPolicy,
) -> PathValidationResult {
    let trace_start = trace_total_start();
    let trace_before = trace_snapshot_if_enabled();
    let (inputs, deduped) = dedupe_inputs(raw_inputs);
    let total = inputs.len();
    let mut out = PathValidationResult::default();
    out.deduped = deduped;

    let cwd_snapshot = if policy.allow_relative {
        policy.cwd_snapshot.clone().or_else(current_dir_safe)
    } else {
        None
    };

    let mut checked: Vec<(usize, OsString, PathBuf)> = Vec::with_capacity(total);

    for (idx, raw) in inputs.into_iter().enumerate() {
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

        checked.push((idx, raw, path));
    }

    let probe_cache = if policy.must_exist && total >= BATCH_PROBE_MIN {
        build_probe_cache(
            checked.iter().map(|(idx, _, path)| (*idx, path)),
            total,
            BATCH_PROBE_MIN,
        )
    } else {
        vec![None; total]
    };

    for (idx, raw, path) in checked {
        if policy.must_exist {
            let probe_timer = trace_stage_start(TraceStage::Probe);
            match probe_cache
                .get(idx)
                .and_then(|cached| cached.as_ref())
            {
                Some(Ok(attr)) => {
                    if !policy.allow_reparse && winapi::is_reparse_point(*attr) {
                        trace_stage_end(TraceStage::Probe, probe_timer);
                        out.issues.push(build_issue(raw.as_os_str(), PathIssueKind::ReparsePoint));
                        continue;
                    }
                }
                Some(Err(kind)) => {
                    trace_stage_end(TraceStage::Probe, probe_timer);
                    out.issues.push(build_issue(raw.as_os_str(), *kind));
                    continue;
                }
                None => match winapi::probe(&path) {
                    Ok(attr) => {
                        if !policy.allow_reparse && winapi::is_reparse_point(attr) {
                            trace_stage_end(TraceStage::Probe, probe_timer);
                            out.issues.push(build_issue(raw.as_os_str(), PathIssueKind::ReparsePoint));
                            continue;
                        }
                    }
                    Err(kind) => {
                        trace_stage_end(TraceStage::Probe, probe_timer);
                        out.issues.push(build_issue(raw.as_os_str(), kind));
                        continue;
                    }
                },
            }
            trace_stage_end(TraceStage::Probe, probe_timer);
        }

        if policy.safety_check {
            let safety_timer = trace_stage_start(TraceStage::Safety);
            if crate::windows::safety::ensure_safe_target(&path).is_err() {
                trace_stage_end(TraceStage::Safety, safety_timer);
                out.issues.push(build_issue(raw.as_os_str(), PathIssueKind::AccessDenied));
                continue;
            }
            trace_stage_end(TraceStage::Safety, safety_timer);
        }

        out.ok.push(path);
    }

    trace_report(
        "validate_paths_serial",
        out.ok.len() + out.issues.len(),
        out.deduped,
        out.ok.len(),
        out.issues.len(),
        trace_start,
        trace_before,
    );
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

    let mut handle_for_canonical = None;
    let mut attr_from_handle = None;

    if probe_any || policy.must_exist {
        let probe_timer = trace_stage_start(TraceStage::Probe);
        if policy.must_exist && policy.allow_reparse {
            if let Ok(handle) = winapi::open_path_with_policy(&info.path, policy) {
                if let Ok(tag_info) = winapi::get_attribute_tag_info(&handle) {
                    attr_from_handle = Some(tag_info.FileAttributes);
                }
                handle_for_canonical = Some(handle);
            }
        }

        if probe_any {
            if let Some(attr) = attr_from_handle {
                info.is_reparse_point = winapi::is_reparse_point(attr);
                info.is_directory = Some(winapi::is_directory(attr));
            } else {
                match winapi::probe(&info.path) {
                    Ok(attr) => {
                        info.is_reparse_point = winapi::is_reparse_point(attr);
                        info.is_directory = Some(winapi::is_directory(attr));
                    }
                    Err(kind) => {
                        trace_stage_end(TraceStage::Probe, probe_timer);
                        if policy.must_exist {
                            return Err(build_issue(raw, kind));
                        }
                        info.existence_probe = Some(kind);
                    }
                }
            }
        } else if policy.must_exist {
            if let Some(attr) = attr_from_handle {
                info.is_reparse_point = winapi::is_reparse_point(attr);
                info.is_directory = Some(winapi::is_directory(attr));
            } else {
                match winapi::probe(&info.path) {
                    Ok(attr) => {
                        info.is_reparse_point = winapi::is_reparse_point(attr);
                        info.is_directory = Some(winapi::is_directory(attr));
                    }
                    Err(kind) => {
                        trace_stage_end(TraceStage::Probe, probe_timer);
                        return Err(build_issue(raw, kind));
                    }
                }
            }
        }
        trace_stage_end(TraceStage::Probe, probe_timer);
    }

    if policy.must_exist && !policy.allow_reparse && info.is_reparse_point {
        return Err(build_issue(raw, PathIssueKind::ReparsePoint));
    }

    if policy.must_exist && policy.allow_reparse {
        let canonical_timer = trace_stage_start(TraceStage::Canonical);
        if let Some(handle) = handle_for_canonical {
            if let Ok(final_path) = winapi::get_final_path(&handle) {
                info.canonical = Some(final_path);
            }
        }
        trace_stage_end(TraceStage::Canonical, canonical_timer);
    }

    if policy.safety_check {
        let safety_timer = trace_stage_start(TraceStage::Safety);
        if crate::windows::safety::ensure_safe_target(&info.path).is_err() {
            trace_stage_end(TraceStage::Safety, safety_timer);
            return Err(build_issue(raw, PathIssueKind::AccessDenied));
        }
        trace_stage_end(TraceStage::Safety, safety_timer);
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

    let string_timer = trace_stage_start(TraceStage::StringCheck);
    if let Some(kind) = string_check::check_string(raw, scratch, &check_policy) {
        trace_stage_end(TraceStage::StringCheck, string_timer);
        return Err(kind);
    }
    trace_stage_end(TraceStage::StringCheck, string_timer);

    if !policy.expand_env && has_env {
        return Err(PathIssueKind::EnvVarNotAllowed);
    }

    let mut current = raw.to_os_string();
    if policy.expand_env && has_env {
        let expand_timer = trace_stage_start(TraceStage::ExpandEnv);
        current = winapi::expand_env(raw)?;
        trace_stage_end(TraceStage::ExpandEnv, expand_timer);
        fill_wide(scratch, &current);
        let string_timer = trace_stage_start(TraceStage::StringCheck);
        if let Some(kind) = string_check::check_string(&current, scratch, policy) {
            trace_stage_end(TraceStage::StringCheck, string_timer);
            return Err(kind);
        }
        trace_stage_end(TraceStage::StringCheck, string_timer);
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
        let resolve_timer = trace_stage_start(TraceStage::RelativeResolve);
        let joined = base.join(&current);
        let full = winapi::get_full_path(&joined)?;
        trace_stage_end(TraceStage::RelativeResolve, resolve_timer);
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
    let dedupe_timer = trace_stage_start(TraceStage::Dedupe);
    let mut seen: HashMap<u64, usize> = HashMap::with_capacity(raw_inputs.len());
    let mut collisions: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut out: Vec<OsString> = Vec::new();
    let mut deduped = 0usize;

    'inputs: for raw in raw_inputs {
        let (hash, first_idx, current_norm) = with_check_buf(|scratch| {
            fill_wide(scratch, &raw);
            normalize_for_dedupe_in_place(scratch);
            let hash = hash_units(scratch);
            let first_idx = seen.get(&hash).copied();
            let current_norm = if first_idx.is_some() {
                Some(scratch.clone())
            } else {
                None
            };
            (hash, first_idx, current_norm)
        });

        if let Some(first_idx) = first_idx {
            let current_norm = current_norm.unwrap_or_else(|| {
                with_check_buf(|scratch| {
                    fill_wide(scratch, &raw);
                    normalize_for_dedupe_in_place(scratch);
                    scratch.clone()
                })
            });

            if normalized_equals_raw(&current_norm, &out[first_idx]) {
                deduped += 1;
                continue;
            }

            let entry = collisions.entry(hash).or_insert_with(|| vec![first_idx]);
            for &idx in entry.iter() {
                if idx == first_idx {
                    continue;
                }
                if normalized_equals_raw(&current_norm, &out[idx]) {
                    deduped += 1;
                    continue 'inputs;
                }
            }

            let new_idx = out.len();
            out.push(raw);
            entry.push(new_idx);
            continue;
        }

        seen.insert(hash, out.len());
        out.push(raw);
    }

    trace_stage_end(TraceStage::Dedupe, dedupe_timer);
    (out, deduped)
}

fn normalize_for_dedupe_in_place(units: &mut Vec<u16>) {
    for unit in units.iter_mut() {
        if *unit == FSLASH {
            *unit = SLASH;
        }
        if (b'A' as u16..=b'Z' as u16).contains(unit) {
            *unit = *unit + 32;
        }
    }
    trim_trailing_backslash(units);
}

fn hash_units(units: &[u16]) -> u64 {
    let mut hasher = DefaultHasher::new();
    units.hash(&mut hasher);
    hasher.finish()
}

fn hash_ascii_units(units: &[u16]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;
    let mut hash = FNV_OFFSET;
    for &unit in units {
        hash ^= unit as u8 as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn hash_ascii_folded(units: &[u16]) -> Option<u64> {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;
    let mut hash = FNV_OFFSET;
    for &unit in units {
        if unit > 0x7f {
            return None;
        }
        let lower = if (b'A' as u16..=b'Z' as u16).contains(&unit) {
            unit + 32
        } else {
            unit
        };
        hash ^= lower as u8 as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    Some(hash)
}

fn ascii_eq_folded(entry: &[u16], candidate_lower: &[u16]) -> bool {
    if entry.len() != candidate_lower.len() {
        return false;
    }
    for (idx, &unit) in entry.iter().enumerate() {
        if unit > 0x7f {
            return false;
        }
        let lower = if (b'A' as u16..=b'Z' as u16).contains(&unit) {
            unit + 32
        } else {
            unit
        };
        if lower != candidate_lower[idx] {
            return false;
        }
    }
    true
}

pub(crate) fn build_probe_cache<'a, I>(
    items: I,
    total: usize,
    min_batch: usize,
) -> Vec<Option<Result<u32, PathIssueKind>>>
where
    I: IntoIterator<Item = (usize, &'a PathBuf)>,
{
    let mut cache: Vec<Option<Result<u32, PathIssueKind>>> = vec![None; total];
    let mut grouped: HashMap<PathBuf, Vec<(usize, Vec<u16>)>> = HashMap::new();

    for (idx, path) in items {
        let parent = match path.parent() {
            Some(value) => value,
            None => continue,
        };
        let Some(name) = path.file_name() else {
            continue;
        };
        let Some(norm_name) = ascii_lower_os(name) else {
            continue;
        };
        grouped
            .entry(parent.to_path_buf())
            .or_default()
            .push((idx, norm_name));
    }

    for (dir, entries) in grouped {
        if entries.len() < min_batch {
            continue;
        }

        let mut targets: HashMap<u64, Vec<(Vec<u16>, usize)>> = HashMap::new();
        for (idx, name) in &entries {
            let hash = hash_ascii_units(name);
            targets.entry(hash).or_default().push((name.clone(), *idx));
        }

        let result = winapi::probe_dir_entries(&dir, |name, attr| {
            let Some(hash) = hash_ascii_folded(name) else {
                return;
            };
            let Some(candidates) = targets.get(&hash) else {
                return;
            };
            for (candidate, idx) in candidates {
                if ascii_eq_folded(name, candidate) {
                    cache[*idx] = Some(Ok(attr));
                }
            }
        });

        match result {
            Ok(()) => {
                for (idx, _) in entries {
                    if cache[idx].is_none() {
                        cache[idx] = Some(Err(PathIssueKind::NotFound));
                    }
                }
            }
            Err(kind) => {
                for (idx, _) in entries {
                    cache[idx] = Some(Err(kind));
                }
            }
        }
    }

    cache
}

fn normalized_equals_raw(normalized: &[u16], raw: &OsStr) -> bool {
    with_check_buf(|scratch| {
        fill_wide(scratch, raw);
        normalize_for_dedupe_in_place(scratch);
        scratch.as_slice() == normalized
    })
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

fn ascii_lower_os(value: &OsStr) -> Option<Vec<u16>> {
    let mut out: Vec<u16> = Vec::new();
    for unit in value.encode_wide() {
        if unit > 0x7f {
            return None;
        }
        let lower = if (b'A' as u16..=b'Z' as u16).contains(&unit) {
            unit + 32
        } else {
            unit
        };
        out.push(lower);
    }
    Some(out)
}

pub(crate) fn fill_wide(buf: &mut Vec<u16>, raw: &OsStr) {
    buf.clear();
    buf.extend(raw.encode_wide());
}

#[derive(Clone, Copy)]
pub(crate) enum TraceStage {
    Dedupe,
    StringCheck,
    ExpandEnv,
    RelativeResolve,
    Probe,
    Safety,
    Canonical,
}

#[derive(Clone, Copy)]
struct TraceSnapshot {
    dedupe_ns: u64,
    dedupe_count: u64,
    string_ns: u64,
    string_count: u64,
    expand_ns: u64,
    expand_count: u64,
    relative_ns: u64,
    relative_count: u64,
    probe_ns: u64,
    probe_count: u64,
    safety_ns: u64,
    safety_count: u64,
    canonical_ns: u64,
    canonical_count: u64,
}

static TRACE_DEDUPE_NS: AtomicU64 = AtomicU64::new(0);
static TRACE_DEDUPE_COUNT: AtomicU64 = AtomicU64::new(0);
static TRACE_STRING_NS: AtomicU64 = AtomicU64::new(0);
static TRACE_STRING_COUNT: AtomicU64 = AtomicU64::new(0);
static TRACE_EXPAND_NS: AtomicU64 = AtomicU64::new(0);
static TRACE_EXPAND_COUNT: AtomicU64 = AtomicU64::new(0);
static TRACE_RELATIVE_NS: AtomicU64 = AtomicU64::new(0);
static TRACE_RELATIVE_COUNT: AtomicU64 = AtomicU64::new(0);
static TRACE_PROBE_NS: AtomicU64 = AtomicU64::new(0);
static TRACE_PROBE_COUNT: AtomicU64 = AtomicU64::new(0);
static TRACE_SAFETY_NS: AtomicU64 = AtomicU64::new(0);
static TRACE_SAFETY_COUNT: AtomicU64 = AtomicU64::new(0);
static TRACE_CANONICAL_NS: AtomicU64 = AtomicU64::new(0);
static TRACE_CANONICAL_COUNT: AtomicU64 = AtomicU64::new(0);
static TRACE_ENABLED: OnceLock<bool> = OnceLock::new();

fn trace_enabled() -> bool {
    *TRACE_ENABLED.get_or_init(|| {
        std::env::var("XUN_PG_TRACE")
            .ok()
            .map(|value| {
                let value = value.trim().to_ascii_lowercase();
                matches!(value.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(false)
    })
}

fn trace_snapshot_if_enabled() -> Option<TraceSnapshot> {
    if !trace_enabled() {
        return None;
    }
    Some(TraceSnapshot {
        dedupe_ns: TRACE_DEDUPE_NS.load(Ordering::Relaxed),
        dedupe_count: TRACE_DEDUPE_COUNT.load(Ordering::Relaxed),
        string_ns: TRACE_STRING_NS.load(Ordering::Relaxed),
        string_count: TRACE_STRING_COUNT.load(Ordering::Relaxed),
        expand_ns: TRACE_EXPAND_NS.load(Ordering::Relaxed),
        expand_count: TRACE_EXPAND_COUNT.load(Ordering::Relaxed),
        relative_ns: TRACE_RELATIVE_NS.load(Ordering::Relaxed),
        relative_count: TRACE_RELATIVE_COUNT.load(Ordering::Relaxed),
        probe_ns: TRACE_PROBE_NS.load(Ordering::Relaxed),
        probe_count: TRACE_PROBE_COUNT.load(Ordering::Relaxed),
        safety_ns: TRACE_SAFETY_NS.load(Ordering::Relaxed),
        safety_count: TRACE_SAFETY_COUNT.load(Ordering::Relaxed),
        canonical_ns: TRACE_CANONICAL_NS.load(Ordering::Relaxed),
        canonical_count: TRACE_CANONICAL_COUNT.load(Ordering::Relaxed),
    })
}

fn trace_total_start() -> Option<Instant> {
    if trace_enabled() {
        Some(Instant::now())
    } else {
        None
    }
}

pub(crate) fn trace_stage_start(_stage: TraceStage) -> Option<Instant> {
    if trace_enabled() {
        Some(Instant::now())
    } else {
        None
    }
}

pub(crate) fn trace_stage_end(stage: TraceStage, start: Option<Instant>) {
    let Some(start) = start else {
        return;
    };
    let ns = start
        .elapsed()
        .as_nanos()
        .min(u64::MAX as u128) as u64;
    match stage {
        TraceStage::Dedupe => {
            TRACE_DEDUPE_NS.fetch_add(ns, Ordering::Relaxed);
            TRACE_DEDUPE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        TraceStage::StringCheck => {
            TRACE_STRING_NS.fetch_add(ns, Ordering::Relaxed);
            TRACE_STRING_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        TraceStage::ExpandEnv => {
            TRACE_EXPAND_NS.fetch_add(ns, Ordering::Relaxed);
            TRACE_EXPAND_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        TraceStage::RelativeResolve => {
            TRACE_RELATIVE_NS.fetch_add(ns, Ordering::Relaxed);
            TRACE_RELATIVE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        TraceStage::Probe => {
            TRACE_PROBE_NS.fetch_add(ns, Ordering::Relaxed);
            TRACE_PROBE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        TraceStage::Safety => {
            TRACE_SAFETY_NS.fetch_add(ns, Ordering::Relaxed);
            TRACE_SAFETY_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        TraceStage::Canonical => {
            TRACE_CANONICAL_NS.fetch_add(ns, Ordering::Relaxed);
            TRACE_CANONICAL_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn trace_diff(after: TraceSnapshot, before: TraceSnapshot) -> TraceSnapshot {
    TraceSnapshot {
        dedupe_ns: after.dedupe_ns.saturating_sub(before.dedupe_ns),
        dedupe_count: after.dedupe_count.saturating_sub(before.dedupe_count),
        string_ns: after.string_ns.saturating_sub(before.string_ns),
        string_count: after.string_count.saturating_sub(before.string_count),
        expand_ns: after.expand_ns.saturating_sub(before.expand_ns),
        expand_count: after.expand_count.saturating_sub(before.expand_count),
        relative_ns: after.relative_ns.saturating_sub(before.relative_ns),
        relative_count: after.relative_count.saturating_sub(before.relative_count),
        probe_ns: after.probe_ns.saturating_sub(before.probe_ns),
        probe_count: after.probe_count.saturating_sub(before.probe_count),
        safety_ns: after.safety_ns.saturating_sub(before.safety_ns),
        safety_count: after.safety_count.saturating_sub(before.safety_count),
        canonical_ns: after.canonical_ns.saturating_sub(before.canonical_ns),
        canonical_count: after.canonical_count.saturating_sub(before.canonical_count),
    }
}

fn trace_report(
    label: &str,
    total_paths: usize,
    deduped: usize,
    ok: usize,
    issues: usize,
    total_start: Option<Instant>,
    before: Option<TraceSnapshot>,
) {
    if !trace_enabled() {
        return;
    }
    let Some(before) = before else {
        return;
    };
    let after = trace_snapshot_if_enabled().unwrap_or(before);
    let diff = trace_diff(after, before);
    let total_ms = total_start.map(|s| s.elapsed().as_millis()).unwrap_or(0);

    println!(
        "trace:path_guard label={} total_paths={} deduped={} ok={} issues={} total_ms={}",
        label, total_paths, deduped, ok, issues, total_ms
    );
    trace_report_stage("dedupe", diff.dedupe_ns, diff.dedupe_count);
    trace_report_stage("string_check", diff.string_ns, diff.string_count);
    trace_report_stage("expand_env", diff.expand_ns, diff.expand_count);
    trace_report_stage("relative_resolve", diff.relative_ns, diff.relative_count);
    trace_report_stage("probe", diff.probe_ns, diff.probe_count);
    trace_report_stage("safety_check", diff.safety_ns, diff.safety_count);
    trace_report_stage("canonical", diff.canonical_ns, diff.canonical_count);
}

fn trace_report_stage(name: &str, ns: u64, count: u64) {
    if count == 0 {
        return;
    }
    let total_us = ns / 1_000;
    let avg_us = (ns as f64) / (count as f64) / 1_000.0;
    println!(
        "trace:path_guard stage={} count={} total_us={} avg_us={:.2}",
        name, count, total_us, avg_us
    );
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
