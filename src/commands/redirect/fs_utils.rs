use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::io::{self, Read};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub(crate) fn sha256_file(path: &Path) -> io::Result<[u8; 32]> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 64];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..]);
    Ok(out)
}

pub(crate) fn unique_dest_path(base: &Path) -> PathBuf {
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = base.extension().and_then(|s| s.to_str()).unwrap_or("");

    for n in 1u32..=10_000 {
        let name = if ext.is_empty() {
            format!("{stem} ({n})")
        } else {
            format!("{stem} ({n}).{ext}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    base.to_path_buf()
}

pub(crate) fn unique_dest_path_with_timestamp(base: &Path) -> PathBuf {
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = base.extension().and_then(|s| s.to_str()).unwrap_or("");

    let ts = crate::store::now_secs();
    let name = if ext.is_empty() {
        format!("{stem} ({ts})")
    } else {
        format!("{stem} ({ts}).{ext}")
    };
    let candidate = parent.join(&name);
    if !candidate.exists() {
        return candidate;
    }

    for n in 1u32..=10_000 {
        let name = if ext.is_empty() {
            format!("{stem} ({ts}) ({n})")
        } else {
            format!("{stem} ({ts}) ({n}).{ext}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }

    unique_dest_path(base)
}

pub(crate) fn wide(path: &Path) -> Vec<u16> {
    let s = path.to_string_lossy().to_string();
    let s = normalize_extended_path(&s);
    OsStr::new(&s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub(crate) fn normalize_extended_path(raw: &str) -> String {
    if raw.starts_with(r"\\?\") {
        return raw.to_string();
    }
    if raw.starts_with(r"\\") {
        return format!(r"\\?\UNC\{}", raw.trim_start_matches(r"\\"));
    }
    if raw.len() >= 240 && raw.chars().nth(1) == Some(':') {
        return format!(r"\\?\{}", raw);
    }
    raw.to_string()
}
