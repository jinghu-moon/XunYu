pub(super) fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(super) fn path_to_unc_wide(path: &str) -> Vec<u16> {
    to_wide(&format!("\\\\?\\{}", path))
}
