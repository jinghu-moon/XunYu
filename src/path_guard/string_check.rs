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

pub(crate) fn check_component(component: &[u16]) -> Option<PathIssueKind> {
    if component.is_empty() {
        return None;
    }
    if is_dot_component(component) {
        return None;
    }
    if matches!(component.last(), Some(&SPACE) | Some(&DOT)) {
        return Some(PathIssueKind::TrailingDotSpace);
    }

    let stem_end = component.iter().position(|&u| u == DOT).unwrap_or(component.len());
    if stem_end == 0 {
        return None;
    }
    let stem = &component[..stem_end];
    for name in reserved_names() {
        if eq_ignore_ascii_case_wide(stem, name) {
            return Some(PathIssueKind::ReservedName);
        }
    }
    None
}

pub(crate) fn check_traversal(base: &Path, joined: &Path) -> Option<PathIssueKind> {
    if joined.strip_prefix(base).is_err() {
        return Some(PathIssueKind::TraversalDetected);
    }
    None
}

pub(crate) fn check_string(raw: &OsStr, units: &[u16], policy: &PathPolicy) -> Option<PathIssueKind> {
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
        PathKind::Relative if !policy.allow_relative => {
            return Some(PathIssueKind::RelativeNotAllowed)
        }
        _ => {}
    }

    let mut start = 0usize;
    for (idx, &unit) in units.iter().enumerate() {
        if unit == SLASH || unit == FSLASH {
            if idx > start {
                if let Some(issue) = check_component(&units[start..idx]) {
                    return Some(issue);
                }
            }
            start = idx + 1;
        }
    }
    if start < units.len() {
        if let Some(issue) = check_component(&units[start..]) {
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

fn eq_ignore_ascii_case_wide(component: &[u16], name: &str) -> bool {
    let mut iter = name.encode_utf16();
    for &unit in component {
        let Some(expected) = iter.next() else {
            return false;
        };
        let unit = if (b'A' as u16..=b'Z' as u16).contains(&unit) {
            unit + 32
        } else {
            unit
        };
        let expected = if (b'A' as u16..=b'Z' as u16).contains(&expected) {
            expected + 32
        } else {
            expected
        };
        if unit != expected {
            return false;
        }
    }
    iter.next().is_none()
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
            check_string(raw, &wide, &policy),
            Some(PathIssueKind::RelativeNotAllowed)
        );

        let raw = OsStr::new(r"C:rel\file");
        let wide = to_wide(r"C:rel\file");
        assert_eq!(
            check_string(raw, &wide, &policy),
            Some(PathIssueKind::DriveRelativeNotAllowed)
        );

        let raw = OsStr::new(r"file.txt:stream");
        let wide = to_wide(r"file.txt:stream");
        assert_eq!(
            check_string(raw, &wide, &policy),
            Some(PathIssueKind::AdsNotAllowed)
        );

        let mut policy = PathPolicy::for_read();
        policy.allow_ads = true;
        let raw = OsStr::new("NUL.txt");
        let wide = to_wide("NUL.txt");
        assert_eq!(
            check_string(raw, &wide, &policy),
            Some(PathIssueKind::ReservedName)
        );

        let raw = OsStr::new(r"\\.\COM1");
        let wide = to_wide(r"\\.\COM1");
        assert_eq!(
            check_string(raw, &wide, &policy),
            Some(PathIssueKind::DeviceNamespaceNotAllowed)
        );

        let raw = OsStr::new(r"\Device\HarddiskVolume1\Windows");
        let wide = to_wide(r"\Device\HarddiskVolume1\Windows");
        assert_eq!(
            check_string(raw, &wide, &policy),
            Some(PathIssueKind::NtNamespaceNotAllowed)
        );

        let raw = OsStr::new(r"\\?\Volume{1234-5678}\file.txt");
        let wide = to_wide(r"\\?\Volume{1234-5678}\file.txt");
        assert_eq!(
            check_string(raw, &wide, &policy),
            Some(PathIssueKind::VolumeGuidNotAllowed)
        );

        let mut policy = PathPolicy::for_read();
        policy.base = Some(PathBuf::from(r"C:\base"));
        let raw = OsStr::new(r"C:\evil\file.txt");
        let wide = to_wide(r"C:\evil\file.txt");
        assert_eq!(
            check_string(raw, &wide, &policy),
            Some(PathIssueKind::TraversalDetected)
        );
    }
}
