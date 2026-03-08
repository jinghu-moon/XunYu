use std::path::Path;

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(super) fn path_wide(path: &Path) -> Vec<u16> {
    let canonical = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    to_wide_null(&canonical.to_string_lossy())
}
