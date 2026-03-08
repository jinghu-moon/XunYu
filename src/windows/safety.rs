use std::path::Path;

static BLACKLIST_DIRS: &[&str] = &[
    "c:\\windows",
    "c:\\program files",
    "c:\\program files (x86)",
    "c:\\programdata",
    "c:\\boot",
];

pub(crate) fn ensure_safe_target(path: &Path) -> Result<(), &'static str> {
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If it doesn't exist, we can't fully check, but we can check the string
            path.to_path_buf()
        }
    };

    let p_str = canonical.to_string_lossy().to_lowercase();
    // remove UNC prefix if any for easier matching
    let check_str = if p_str.starts_with(r"\\?\") {
        &p_str[4..]
    } else {
        &p_str
    };

    for &blocked in BLACKLIST_DIRS {
        if check_str.contains(blocked) || check_str == blocked {
            return Err("Target path is restricted for system safety.");
        }
    }

    Ok(())
}
