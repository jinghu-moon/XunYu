use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    DriveAbsolute,
    DriveRelative,
    Relative,
    UNC,
    ExtendedLength,
    ExtendedUNC,
    DeviceNamespace,
    NTNamespace,
    VolumeGuid,
    ADS,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathIssueKind {
    Empty,
    InvalidChar,
    ReservedName,
    TrailingDotSpace,
    TooLong,
    RelativeNotAllowed,
    DriveRelativeNotAllowed,
    TraversalDetected,
    NotFound,
    AccessDenied,
    ReparsePoint,
    AdsNotAllowed,
    DeviceNamespaceNotAllowed,
    NtNamespaceNotAllowed,
    VolumeGuidNotAllowed,
    EnvVarNotAllowed,
    NetworkPathNotFound, // ERROR_BAD_NETPATH (53)
    SharingViolation,    // ERROR_SHARING_VIOLATION (32)
    SymlinkLoop,         // ERROR_CANT_RESOLVE_FILENAME (1921)
    IoError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathIssue {
    pub raw: String,
    pub kind: PathIssueKind,
    pub detail: &'static str,
}

#[derive(Debug, Default, Clone)]
pub struct PathValidationResult {
    pub ok: Vec<PathBuf>,
    pub issues: Vec<PathIssue>,
    pub deduped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathInfo {
    pub path: PathBuf,
    pub kind: PathKind,
    pub canonical: Option<PathBuf>,
    pub is_reparse_point: bool,
    pub is_directory: Option<bool>,
    pub existence_probe: Option<PathIssueKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPolicy {
    pub must_exist: bool,
    pub allow_relative: bool,
    pub expand_env: bool,
    pub allow_reparse: bool,
    pub allow_ads: bool,
    pub base: Option<PathBuf>,
    pub safety_check: bool,
    pub cwd_snapshot: Option<PathBuf>,
}

impl PathPolicy {
    pub fn for_write() -> Self {
        Self {
            must_exist: true,
            allow_relative: false,
            expand_env: false,
            allow_reparse: false,
            allow_ads: false,
            base: None,
            safety_check: true,
            cwd_snapshot: None,
        }
    }

    pub fn for_read() -> Self {
        Self {
            must_exist: true,
            allow_relative: true,
            expand_env: false,
            allow_reparse: true,
            allow_ads: false,
            base: None,
            safety_check: false,
            cwd_snapshot: None,
        }
    }

    pub fn for_output() -> Self {
        Self {
            must_exist: false,
            allow_relative: false,
            expand_env: false,
            allow_reparse: false,
            allow_ads: false,
            base: None,
            safety_check: false,
            cwd_snapshot: None,
        }
    }
}

impl Default for PathPolicy {
    fn default() -> Self {
        Self::for_read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_write_defaults() {
        let policy = PathPolicy::for_write();
        assert!(policy.must_exist);
        assert!(!policy.allow_relative);
        assert!(!policy.expand_env);
        assert!(!policy.allow_reparse);
        assert!(!policy.allow_ads);
        assert!(policy.base.is_none());
        assert!(policy.safety_check);
        assert!(policy.cwd_snapshot.is_none());
    }

    #[test]
    fn for_read_defaults() {
        let policy = PathPolicy::for_read();
        assert!(policy.must_exist);
        assert!(policy.allow_relative);
        assert!(!policy.expand_env);
        assert!(policy.allow_reparse);
        assert!(!policy.allow_ads);
        assert!(policy.base.is_none());
        assert!(!policy.safety_check);
        assert!(policy.cwd_snapshot.is_none());
    }

    #[test]
    fn for_output_defaults() {
        let policy = PathPolicy::for_output();
        assert!(!policy.must_exist);
        assert!(!policy.allow_relative);
        assert!(!policy.expand_env);
        assert!(!policy.allow_reparse);
        assert!(!policy.allow_ads);
        assert!(policy.base.is_none());
        assert!(!policy.safety_check);
        assert!(policy.cwd_snapshot.is_none());
    }

    #[test]
    fn default_matches_for_read() {
        assert_eq!(PathPolicy::default(), PathPolicy::for_read());
    }
}
