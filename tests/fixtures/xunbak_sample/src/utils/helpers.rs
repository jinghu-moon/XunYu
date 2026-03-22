/// Utility helpers for path normalization and string operations.
/// Tests: compressible text, moderate size, nested module path.

use std::path::{Path, PathBuf};

/// Normalize a Windows path to forward-slash separated form.
pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Strip a common prefix from a path.
pub fn strip_prefix<'a>(path: &'a Path, base: &Path) -> Option<&'a Path> {
    path.strip_prefix(base).ok()
}

/// Generate a deterministic temporary name for a given index.
pub fn temp_name(index: usize) -> String {
    format!("__xun_tmp_{index:04}__")
}

/// Case-insensitive path comparison for Windows semantics.
pub fn paths_equal_ci(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Truncate a string to a maximum byte length, ensuring valid UTF-8.
pub fn truncate_utf8(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_converts_backslashes() {
        let p = Path::new(r"C:\Users\test\file.txt");
        assert_eq!(normalize_path(p), "C:/Users/test/file.txt");
    }

    #[test]
    fn temp_name_format() {
        assert_eq!(temp_name(0), "__xun_tmp_0000__");
        assert_eq!(temp_name(42), "__xun_tmp_0042__");
    }

    #[test]
    fn case_insensitive_comparison() {
        assert!(paths_equal_ci("Foo/Bar.txt", "foo/bar.txt"));
        assert!(!paths_equal_ci("foo.txt", "bar.txt"));
    }

    #[test]
    fn truncate_respects_utf8_boundaries() {
        let s = "hello";
        assert_eq!(truncate_utf8(s, 3), "hel");
        assert_eq!(truncate_utf8(s, 100), "hello");
    }
}
