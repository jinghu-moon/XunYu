use std::ffi::OsStr;
use std::path::Path;

use crate::path_guard::policy::{PathIssueKind, PathKind, PathPolicy};

const RESERVED_NAMES: [&str; 28] = [
    "con",
    "prn",
    "aux",
    "nul",
    "com1",
    "com2",
    "com3",
    "com4",
    "com5",
    "com6",
    "com7",
    "com8",
    "com9",
    "com\u{b9}",
    "com\u{b2}",
    "com\u{b3}",
    "lpt1",
    "lpt2",
    "lpt3",
    "lpt4",
    "lpt5",
    "lpt6",
    "lpt7",
    "lpt8",
    "lpt9",
    "lpt\u{b9}",
    "lpt\u{b2}",
    "lpt\u{b3}",
];

const SLASH: u16 = b'\\' as u16;
const FSLASH: u16 = b'/' as u16;
const DOT: u16 = b'.' as u16;
const SPACE: u16 = b' ' as u16;
const COLON: u16 = b':' as u16;
const QMARK: u16 = b'?' as u16;
const SUPERSCRIPT_ONE: u16 = 0x00B9;
const SUPERSCRIPT_TWO: u16 = 0x00B2;
const SUPERSCRIPT_THREE: u16 = 0x00B3;
const C_LOWER: u16 = b'c' as u16;
const O_LOWER: u16 = b'o' as u16;
const N_LOWER: u16 = b'n' as u16;
const P_LOWER: u16 = b'p' as u16;
const R_LOWER: u16 = b'r' as u16;
const A_LOWER: u16 = b'a' as u16;
const U_LOWER: u16 = b'u' as u16;
const X_LOWER: u16 = b'x' as u16;
const L_LOWER: u16 = b'l' as u16;
const T_LOWER: u16 = b't' as u16;
const M_LOWER: u16 = b'm' as u16;

pub(crate) fn reserved_names() -> &'static [&'static str] {
    &RESERVED_NAMES
}

pub(crate) fn detect_kind(units: &[u16]) -> PathKind {
    if matches_prefix(
        units,
        &[
            SLASH, SLASH, QMARK, SLASH, b'U' as u16, b'N' as u16, b'C' as u16, SLASH,
        ],
    ) {
        return PathKind::ExtendedUNC;
    }
    if matches_prefix(
        units,
        &[
            SLASH, SLASH, QMARK, SLASH, b'V' as u16, b'o' as u16, b'l' as u16, b'u' as u16,
            b'm' as u16, b'e' as u16, b'{' as u16,
        ],
    ) {
        return PathKind::VolumeGuid;
    }
    if matches_prefix(units, &[SLASH, SLASH, QMARK, SLASH]) {
        return PathKind::ExtendedLength;
    }
    if matches_prefix(units, &[SLASH, SLASH, b'.' as u16, SLASH]) {
        return PathKind::DeviceNamespace;
    }
    if matches_prefix(units, &[SLASH, SLASH]) {
        return PathKind::UNC;
    }
    if matches_prefix(
        units,
        &[SLASH, b'D' as u16, b'e' as u16, b'v' as u16, b'i' as u16, b'c' as u16, b'e' as u16, SLASH],
    ) || matches_prefix(units, &[SLASH, QMARK, QMARK, SLASH])
    {
        return PathKind::NTNamespace;
    }
    if units.len() >= 2 && is_ascii_letter(units[0]) && units[1] == COLON {
        if units.len() >= 3 && (units[2] == SLASH || units[2] == FSLASH) {
            return PathKind::DriveAbsolute;
        }
        return PathKind::DriveRelative;
    }
    PathKind::Relative
}

pub(crate) fn is_ads(units: &[u16], kind: PathKind) -> bool {
    let Some(pos) = units.iter().position(|&u| u == COLON) else {
        return false;
    };

    if matches!(kind, PathKind::DriveAbsolute | PathKind::DriveRelative) && pos == 1 {
        return false;
    }
    if matches!(kind, PathKind::ExtendedLength) && is_extended_drive_prefix(units) && pos == 5 {
        return false;
    }

    true
}

pub(crate) fn check_chars(units: &[u16]) -> Option<PathIssueKind> {
    if units.is_empty() {
        return Some(PathIssueKind::Empty);
    }

    let has_extended_prefix = matches_prefix(units, &[SLASH, SLASH, QMARK, SLASH]);
    let mut qmark_count = 0usize;

    for &unit in units {
        if unit < 0x20 {
            return Some(PathIssueKind::InvalidChar);
        }

        match unit {
            0x3C | 0x3E | 0x22 | 0x7C | 0x2A => return Some(PathIssueKind::InvalidChar),
            _ => {}
        }

        if unit == QMARK {
            qmark_count += 1;
        }
    }

    if qmark_count > 0 {
        if !has_extended_prefix || qmark_count != 1 || units.get(2) != Some(&QMARK) {
            return Some(PathIssueKind::InvalidChar);
        }
    }

    None
}

pub(crate) fn check_component(component: &[u16], allow_colon: bool) -> Option<PathIssueKind> {
    if component.is_empty() {
        return None;
    }
    if is_dot_component(component) {
        return None;
    }
    if matches!(component.last(), Some(&SPACE) | Some(&DOT)) {
        return Some(PathIssueKind::TrailingDotSpace);
    }
    // 路径组件内的冒号是非法字符（Windows 不允许文件名含冒号）
    // allow_colon=true 时跳过（ADS 已被上层允许）
    if !allow_colon && component.iter().any(|&u| u == COLON) {
        return Some(PathIssueKind::InvalidChar);
    }

    let stem_end = component.iter().position(|&u| u == DOT).unwrap_or(component.len());
    if stem_end == 0 {
        return None;
    }
    let stem = &component[..stem_end];
    if is_reserved_stem(stem) {
        return Some(PathIssueKind::ReservedName);
    }
    None
}

pub(crate) fn check_traversal(base: &Path, joined: &Path) -> Option<PathIssueKind> {
    if joined.strip_prefix(base).is_err() {
        return Some(PathIssueKind::TraversalDetected);
    }
    None
}

pub(crate) fn check_string(
    raw: &OsStr,
    units: &[u16],
    policy: &PathPolicy,
    allow_relative: bool,
) -> Option<PathIssueKind> {
    if let Some(issue) = check_chars(units) {
        return Some(issue);
    }

    let kind = detect_kind(units);
    if is_ads(units, kind) && !policy.allow_ads {
        return Some(PathIssueKind::AdsNotAllowed);
    }

    match kind {
        PathKind::DeviceNamespace => return Some(PathIssueKind::DeviceNamespaceNotAllowed),
        PathKind::NTNamespace => return Some(PathIssueKind::NtNamespaceNotAllowed),
        PathKind::VolumeGuid => return Some(PathIssueKind::VolumeGuidNotAllowed),
        PathKind::DriveRelative => return Some(PathIssueKind::DriveRelativeNotAllowed),
        PathKind::Relative if !allow_relative => {
            return Some(PathIssueKind::RelativeNotAllowed)
        }
        _ => {}
    }

    let mut start = 0usize;
    // 跳过路径中不属于文件名组件的盘符部分，避免冒号被误判为 InvalidChar
    // DriveAbsolute/DriveRelative: "X:" 占 index 0-1，从 index 2 开始
    // ExtendedLength + drive prefix (\\?\X:\): "\\?\" 占 index 0-3，"X:" 占 index 4-5，从 index 6 开始
    if matches!(kind, PathKind::DriveAbsolute | PathKind::DriveRelative) && units.len() >= 2 {
        start = 2; // 跳过 "X:"
    } else if matches!(kind, PathKind::ExtendedLength) && is_extended_drive_prefix(units) {
        start = 6; // 跳过 "\\?\X:"
    }
    for (idx, &unit) in units.iter().enumerate().skip(start) {
        if unit == SLASH || unit == FSLASH {
            if idx > start {
                if let Some(issue) = check_component(&units[start..idx], policy.allow_ads) {
                    return Some(issue);
                }
            }
            start = idx + 1;
        }
    }
    if start < units.len() {
        if let Some(issue) = check_component(&units[start..], policy.allow_ads) {
            return Some(issue);
        }
    }

    if let Some(base) = &policy.base {
        let joined = base.join(Path::new(raw));
        if let Some(issue) = check_traversal(base, &joined) {
            return Some(issue);
        }
    }

    None
}

fn matches_prefix(units: &[u16], prefix: &[u16]) -> bool {
    units.starts_with(prefix)
}

fn is_extended_drive_prefix(units: &[u16]) -> bool {
    units.len() >= 6
        && units[0] == SLASH
        && units[1] == SLASH
        && units[2] == QMARK
        && units[3] == SLASH
        && is_ascii_letter(units[4])
        && units[5] == COLON
}

fn is_dot_component(component: &[u16]) -> bool {
    (component.len() == 1 && component[0] == DOT)
        || (component.len() == 2 && component[0] == DOT && component[1] == DOT)
}

fn is_ascii_letter(unit: u16) -> bool {
    (b'a' as u16..=b'z' as u16).contains(&unit)
        || (b'A' as u16..=b'Z' as u16).contains(&unit)
}

fn is_reserved_stem(stem: &[u16]) -> bool {
    match stem.len() {
        3 => {
            let a = to_ascii_lower(stem[0]);
            let b = to_ascii_lower(stem[1]);
            let c = to_ascii_lower(stem[2]);
            matches!(
                (a, b, c),
                (C_LOWER, O_LOWER, N_LOWER)
                    | (P_LOWER, R_LOWER, N_LOWER)
                    | (A_LOWER, U_LOWER, X_LOWER)
                    | (N_LOWER, U_LOWER, L_LOWER)
            )
        }
        4 => {
            let a = to_ascii_lower(stem[0]);
            let b = to_ascii_lower(stem[1]);
            let c = to_ascii_lower(stem[2]);
            let d = stem[3];
            if matches!((a, b, c), (C_LOWER, O_LOWER, M_LOWER)) {
                return is_reserved_suffix(d);
            }
            if matches!((a, b, c), (L_LOWER, P_LOWER, T_LOWER)) {
                return is_reserved_suffix(d);
            }
            false
        }
        _ => false,
    }
}

fn is_reserved_suffix(unit: u16) -> bool {
    (b'1' as u16..=b'9' as u16).contains(&unit)
        || matches!(unit, SUPERSCRIPT_ONE | SUPERSCRIPT_TWO | SUPERSCRIPT_THREE)
}

fn to_ascii_lower(unit: u16) -> u16 {
    if (b'A' as u16..=b'Z' as u16).contains(&unit) {
        unit + 32
    } else {
        unit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::os::windows::ffi::OsStrExt;

    fn to_wide(raw: &str) -> Vec<u16> {
        OsStr::new(raw).encode_wide().collect()
    }

    #[test]
    fn detect_kind_examples() {
        assert_eq!(
            detect_kind(&to_wide(r"\\?\UNC\server\share\file.txt")),
            PathKind::ExtendedUNC
        );
        assert_eq!(
            detect_kind(&to_wide(r"\\?\Volume{1234-5678}\file.txt")),
            PathKind::VolumeGuid
        );
        assert_eq!(detect_kind(&to_wide(r"\\?\C:\Windows")), PathKind::ExtendedLength);
        assert_eq!(detect_kind(&to_wide(r"\\.\COM1")), PathKind::DeviceNamespace);
        assert_eq!(detect_kind(&to_wide(r"\\server\share\dir")), PathKind::UNC);
        assert_eq!(
            detect_kind(&to_wide(r"\Device\HarddiskVolume1\Windows")),
            PathKind::NTNamespace
        );
        assert_eq!(detect_kind(&to_wide(r"C:\Windows")), PathKind::DriveAbsolute);
        assert_eq!(detect_kind(&to_wide(r"C:/Windows")), PathKind::DriveAbsolute);
        assert_eq!(detect_kind(&to_wide(r"C:Windows")), PathKind::DriveRelative);
        assert_eq!(detect_kind(&to_wide(r"folder\file.txt")), PathKind::Relative);
    }

    #[test]
    fn check_chars_rejects_invalid() {
        assert_eq!(check_chars(&to_wide("")), Some(PathIssueKind::Empty));
        assert_eq!(
            check_chars(&to_wide("a\x1f")),
            Some(PathIssueKind::InvalidChar)
        );
        assert_eq!(
            check_chars(&to_wide("a<b")),
            Some(PathIssueKind::InvalidChar)
        );
        assert_eq!(check_chars(&to_wide("normal.txt")), None);
    }

    #[test]
    fn check_component_rejects_reserved_and_trailing() {
        assert_eq!(check_component(&to_wide(".")), None);
        assert_eq!(check_component(&to_wide("..")), None);
        assert_eq!(
            check_component(&to_wide("file ")),
            Some(PathIssueKind::TrailingDotSpace)
        );
        assert_eq!(
            check_component(&to_wide("file.")),
            Some(PathIssueKind::TrailingDotSpace)
        );
        assert_eq!(
            check_component(&to_wide("CON")),
            Some(PathIssueKind::ReservedName)
        );
        assert_eq!(
            check_component(&to_wide("NUL.txt")),
            Some(PathIssueKind::ReservedName)
        );
        assert_eq!(
            check_component(&to_wide("COM\u{b9}")),
            Some(PathIssueKind::ReservedName)
        );
        assert_eq!(
            check_component(&to_wide("LPT\u{b2}.log")),
            Some(PathIssueKind::ReservedName)
        );
        assert_eq!(check_component(&to_wide("normal.txt")), None);
    }

    #[test]
    fn check_traversal_detects_outside_base() {
        let base = PathBuf::from(r"C:\base");
        let inside = PathBuf::from(r"C:\base\child");
        let outside = PathBuf::from(r"C:\evil");
        assert_eq!(check_traversal(&base, &inside), None);
        assert_eq!(
            check_traversal(&base, &outside),
            Some(PathIssueKind::TraversalDetected)
        );
    }

    #[test]
    fn check_string_applies_policy() {
        let mut policy = PathPolicy::for_read();
        policy.allow_relative = false;
        let raw = OsStr::new(r"rel\file");
        let wide = to_wide(r"rel\file");
        assert_eq!(
            check_string(raw, &wide, &policy, policy.allow_relative),
            Some(PathIssueKind::RelativeNotAllowed)
        );

        let raw = OsStr::new(r"C:rel\file");
        let wide = to_wide(r"C:rel\file");
        assert_eq!(
            check_string(raw, &wide, &policy, policy.allow_relative),
            Some(PathIssueKind::DriveRelativeNotAllowed)
        );

        let raw = OsStr::new(r"file.txt:stream");
        let wide = to_wide(r"file.txt:stream");
        assert_eq!(
            check_string(raw, &wide, &policy, policy.allow_relative),
            Some(PathIssueKind::AdsNotAllowed)
        );

        let mut policy = PathPolicy::for_read();
        policy.allow_ads = true;
        let raw = OsStr::new("NUL.txt");
        let wide = to_wide("NUL.txt");
        assert_eq!(
            check_string(raw, &wide, &policy, policy.allow_relative),
            Some(PathIssueKind::ReservedName)
        );

        let raw = OsStr::new(r"\\.\COM1");
        let wide = to_wide(r"\\.\COM1");
        assert_eq!(
            check_string(raw, &wide, &policy, policy.allow_relative),
            Some(PathIssueKind::DeviceNamespaceNotAllowed)
        );

        let raw = OsStr::new(r"\Device\HarddiskVolume1\Windows");
        let wide = to_wide(r"\Device\HarddiskVolume1\Windows");
        assert_eq!(
            check_string(raw, &wide, &policy, policy.allow_relative),
            Some(PathIssueKind::NtNamespaceNotAllowed)
        );

        let raw = OsStr::new(r"\\?\Volume{1234-5678}\file.txt");
        let wide = to_wide(r"\\?\Volume{1234-5678}\file.txt");
        assert_eq!(
            check_string(raw, &wide, &policy, policy.allow_relative),
            Some(PathIssueKind::VolumeGuidNotAllowed)
        );

        let mut policy = PathPolicy::for_read();
        policy.base = Some(PathBuf::from(r"C:\base"));
        let raw = OsStr::new(r"C:\evil\file.txt");
        let wide = to_wide(r"C:\evil\file.txt");
        assert_eq!(
            check_string(raw, &wide, &policy, policy.allow_relative),
            Some(PathIssueKind::TraversalDetected)
        );
    }
}
