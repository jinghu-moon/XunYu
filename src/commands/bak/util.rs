use std::fs;
use std::path::Path;

pub(crate) fn norm(p: &str) -> String {
    p.trim().replace('/', "\\").trim_matches('\\').to_string()
}

pub(crate) fn is_glob(s: &str) -> bool {
    s.contains('*') || s.contains('?')
}

pub(crate) fn fmt_size(bytes: i64) -> String {
    if bytes == 0 {
        return "    +0.0 B".into();
    }
    let abs = bytes.unsigned_abs();
    let (val, unit) = if abs >= 1_048_576 {
        (abs as f64 / 1_048_576.0, "MB")
    } else if abs >= 1_024 {
        (abs as f64 / 1_024.0, "KB")
    } else {
        (abs as f64, "B")
    };
    let sign = if bytes >= 0 { '+' } else { '-' };
    format!("{sign}{val:>7.1} {unit}")
}

pub(crate) fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = fs::read_dir(path) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else {
                total += fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}
