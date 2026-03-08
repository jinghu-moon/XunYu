use std::path::{Path, PathBuf};

pub(crate) fn dest_root_path(source: &Path, dest_raw: &str) -> PathBuf {
    let mut root = PathBuf::new();
    let p = PathBuf::from(dest_raw);
    for comp in p.components() {
        let s = comp.as_os_str().to_string_lossy();
        if s.contains('{') || s.contains('}') {
            break;
        }
        root.push(comp.as_os_str());
    }
    if root.as_os_str().is_empty() {
        root = p;
    }
    let abs = if root.is_absolute() {
        root
    } else {
        source.join(root)
    };
    abs.canonicalize().unwrap_or(abs)
}

pub(crate) fn is_network_share_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.starts_with(r"\\")
}
