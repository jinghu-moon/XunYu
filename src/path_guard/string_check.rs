use std::path::Path;

use memchr::memchr;

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

pub(crate) fn reserved_names() -> &'static [&'static str] {
    &RESERVED_NAMES
}

pub(crate) fn detect_kind(path: &str) -> PathKind {
    let bytes = path.as_bytes();

    if matches!(
        bytes,
        [b'\\', b'\\', b'?', b'\\', b'U', b'N', b'C', b'\\', ..]
    ) {
        return PathKind::ExtendedUNC;
    }
    if matches!(bytes, [b'\\', b'\\', b'?', b'\\', b'V', b'o', b'l', b'u', b'm', b'e', b'{', ..]) {
        return PathKind::VolumeGuid;
    }
    if matches!(bytes, [b'\\', b'\\', b'?', b'\\', ..]) {
        return PathKind::ExtendedLength;
    }
    if matches!(bytes, [b'\\', b'\\', b'.', b'\\', ..]) {
        return PathKind::DeviceNamespace;
    }
    if matches!(bytes, [b'\\', b'\\', ..]) {
        return PathKind::UNC;
    }
    if matches!(bytes, [b'\\', b'D', b'e', b'v', b'i', b'c', b'e', b'\\', ..])
        || matches!(bytes, [b'\\', b'?', b'?', b'\\', ..])
    {
        return PathKind::NTNamespace;
    }
    if bytes.len() >= 2 && is_ascii_letter(bytes[0]) && bytes[1] == b':' {
        if bytes.len() >= 3 && (bytes[2] == b'\\' || bytes[2] == b'/') {
            return PathKind::DriveAbsolute;
        }
        return PathKind::DriveRelative;
    }
    PathKind::Relative
}

pub(crate) fn is_ads(path: &str, kind: PathKind) -> bool {
    let bytes = path.as_bytes();
    let Some(pos) = memchr(b':', bytes) else {
        return false;
    };
    if matches!(kind, PathKind::DriveAbsolute | PathKind::DriveRelative) && pos == 1 {
        return false;
    }
    true
}

pub(crate) fn check_chars(path: &str) -> Option<PathIssueKind> {
    let bytes = path.as_bytes();
    if bytes.is_empty() {
        return Some(PathIssueKind::Empty);
    }
    let has_extended_prefix =
        matches!(bytes, [b'\\', b'\\', b'?', b'\\', ..]);
    if bytes.iter().any(|&b| b < 0x20) {
        return Some(PathIssueKind::InvalidChar);
    }
    if memchr(b'<', bytes).is_some()
        || memchr(b'>', bytes).is_some()
        || memchr(b'"', bytes).is_some()
        || memchr(b'|', bytes).is_some()
        || memchr(b'*', bytes).is_some()
    {
        return Some(PathIssueKind::InvalidChar);
    }
    if let Some(pos) = memchr(b'?', bytes) {
        let extra = memchr(b'?', &bytes[(pos + 1)..]).is_some();
        if !(has_extended_prefix && pos == 2 && !extra) {
            return Some(PathIssueKind::InvalidChar);
        }
    }
    None
}

pub(crate) fn check_component(component: &str) -> Option<PathIssueKind> {
    let bytes = component.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    if component == "." || component == ".." {
        return None;
    }
    if matches!(bytes.last(), Some(b' ' | b'.')) {
        return Some(PathIssueKind::TrailingDotSpace);
    }
    let stem_end = memchr(b'.', bytes).unwrap_or(bytes.len());
    if stem_end == 0 {
        return None;
    }
    let stem = &bytes[..stem_end];
    for name in reserved_names() {
        if eq_ignore_ascii_case_bytes(stem, name.as_bytes()) {
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

pub(crate) fn check_string(raw: &str, policy: &PathPolicy) -> Option<PathIssueKind> {
    if let Some(issue) = check_chars(raw) {
        return Some(issue);
    }

    let kind = detect_kind(raw);
    if is_ads(raw, kind) && !policy.allow_ads {
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

    let bytes = raw.as_bytes();
    let mut start = 0usize;
    for (idx, &b) in bytes.iter().enumerate() {
        if b == b'\\' || b == b'/' {
            if idx > start {
                if let Some(issue) = check_component(&raw[start..idx]) {
                    return Some(issue);
                }
            }
            start = idx + 1;
        }
    }
    if start < raw.len() {
        if let Some(issue) = check_component(&raw[start..]) {
            return Some(issue);
        }
    }

    if let Some(base) = &policy.base {
        let joined = base.join(raw);
        if let Some(issue) = check_traversal(base, &joined) {
            return Some(issue);
        }
    }

    None
}

fn is_ascii_letter(byte: u8) -> bool {
    (b'a'..=b'z').contains(&byte) || (b'A'..=b'Z').contains(&byte)
}

fn eq_ignore_ascii_case_bytes(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    for (&l, &r) in left.iter().zip(right.iter()) {
        let l = if (b'A'..=b'Z').contains(&l) {
            l + 32
        } else {
            l
        };
        let r = if (b'A'..=b'Z').contains(&r) {
            r + 32
        } else {
            r
        };
        if l != r {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detect_kind_examples() {
        assert_eq!(
            detect_kind(r"\\?\UNC\server\share\file.txt"),
            PathKind::ExtendedUNC
        );
        assert_eq!(
            detect_kind(r"\\?\Volume{1234-5678}\file.txt"),
            PathKind::VolumeGuid
        );
        assert_eq!(detect_kind(r"\\?\C:\Windows"), PathKind::ExtendedLength);
        assert_eq!(detect_kind(r"\\.\COM1"), PathKind::DeviceNamespace);
        assert_eq!(detect_kind(r"\\server\share\dir"), PathKind::UNC);
        assert_eq!(
            detect_kind(r"\Device\HarddiskVolume1\Windows"),
            PathKind::NTNamespace
        );
        assert_eq!(detect_kind(r"C:\Windows"), PathKind::DriveAbsolute);
        assert_eq!(detect_kind(r"C:/Windows"), PathKind::DriveAbsolute);
        assert_eq!(detect_kind(r"C:Windows"), PathKind::DriveRelative);
        assert_eq!(detect_kind(r"folder\file.txt"), PathKind::Relative);
    }

    #[test]
    fn check_chars_rejects_invalid() {
        assert_eq!(check_chars(""), Some(PathIssueKind::Empty));
        assert_eq!(check_chars("a\x1f"), Some(PathIssueKind::InvalidChar));
        assert_eq!(check_chars("a<b"), Some(PathIssueKind::InvalidChar));
        assert_eq!(check_chars("normal.txt"), None);
    }

    #[test]
    fn check_component_rejects_reserved_and_trailing() {
        assert_eq!(check_component("."), None);
        assert_eq!(check_component(".."), None);
        assert_eq!(
            check_component("file "),
            Some(PathIssueKind::TrailingDotSpace)
        );
        assert_eq!(
            check_component("file."),
            Some(PathIssueKind::TrailingDotSpace)
        );
        assert_eq!(check_component("CON"), Some(PathIssueKind::ReservedName));
        assert_eq!(
            check_component("NUL.txt"),
            Some(PathIssueKind::ReservedName)
        );
        assert_eq!(
            check_component("COM\u{b9}"),
            Some(PathIssueKind::ReservedName)
        );
        assert_eq!(
            check_component("LPT\u{b2}.log"),
            Some(PathIssueKind::ReservedName)
        );
        assert_eq!(check_component("normal.txt"), None);
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
        assert_eq!(
            check_string(r"rel\file", &policy),
            Some(PathIssueKind::RelativeNotAllowed)
        );
        assert_eq!(
            check_string(r"C:rel\file", &policy),
            Some(PathIssueKind::DriveRelativeNotAllowed)
        );
        assert_eq!(
            check_string(r"file.txt:stream", &policy),
            Some(PathIssueKind::AdsNotAllowed)
        );

        let mut policy = PathPolicy::for_read();
        policy.allow_ads = true;
        assert_eq!(check_string("NUL.txt", &policy), Some(PathIssueKind::ReservedName));
        assert_eq!(
            check_string(r"\\.\COM1", &policy),
            Some(PathIssueKind::DeviceNamespaceNotAllowed)
        );
        assert_eq!(
            check_string(r"\Device\HarddiskVolume1\Windows", &policy),
            Some(PathIssueKind::NtNamespaceNotAllowed)
        );
        assert_eq!(
            check_string(r"\\?\Volume{1234-5678}\file.txt", &policy),
            Some(PathIssueKind::VolumeGuidNotAllowed)
        );

        let mut policy = PathPolicy::for_read();
        policy.base = Some(PathBuf::from(r"C:\base"));
        assert_eq!(
            check_string(r"C:\evil\file.txt", &policy),
            Some(PathIssueKind::TraversalDetected)
        );
    }
}
