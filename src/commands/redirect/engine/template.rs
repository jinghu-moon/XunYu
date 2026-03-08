use super::super::matcher::{SizeOp, parse_age_expr};
use super::canonical_or_lexical;

use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::Time::FileTimeToSystemTime;

pub(crate) fn resolve_dest_dir(source: &Path, dest_raw: &str, src_path: &Path) -> PathBuf {
    let rendered = render_dest_template(dest_raw, src_path);
    let dest = PathBuf::from(rendered);
    if dest.is_absolute() {
        canonical_or_lexical(&dest)
    } else {
        canonical_or_lexical(&source.join(dest))
    }
}

fn render_dest_template(dest_raw: &str, src_path: &Path) -> String {
    let file_name = src_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let (cy, cm) = created_year_month(src_path).unwrap_or((0, 0));
    let mut out = dest_raw.to_string();
    out = out.replace("{name}", stem);
    out = out.replace("{ext}", ext);
    if cy > 0 {
        out = out.replace("{created.year}", &format!("{cy:04}"));
    }
    if cm > 0 {
        out = out.replace("{created.month}", &format!("{cm:02}"));
    }
    out
}

fn created_year_month(path: &Path) -> Option<(u16, u16)> {
    let meta = std::fs::metadata(path).ok()?;
    let created = meta.created().or_else(|_| meta.modified()).ok()?;
    let secs = created
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let ft = filetime_from_unix_secs(secs);
    let mut st = windows_sys::Win32::Foundation::SYSTEMTIME {
        wYear: 0,
        wMonth: 0,
        wDayOfWeek: 0,
        wDay: 0,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
    };
    let ok = unsafe { FileTimeToSystemTime(&ft as *const _, &mut st as *mut _) };
    if ok == 0 || st.wYear == 0 || st.wMonth == 0 {
        return None;
    }
    Some((st.wYear, st.wMonth))
}

fn filetime_from_unix_secs(secs: u64) -> FILETIME {
    const EPOCH_DIFF_SECS: u64 = 11_644_473_600;
    let intervals = (secs.saturating_add(EPOCH_DIFF_SECS)).saturating_mul(10_000_000);
    FILETIME {
        dwLowDateTime: (intervals & 0xFFFF_FFFF) as u32,
        dwHighDateTime: (intervals >> 32) as u32,
    }
}

pub(crate) fn age_matches(path: &Path, age_expr: &str) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return false;
    };
    let Ok(m) = modified.duration_since(std::time::UNIX_EPOCH) else {
        return false;
    };
    let secs = now.as_secs().saturating_sub(m.as_secs());
    let Ok((op, rhs)) = parse_age_expr(age_expr) else {
        return false;
    };
    match op {
        SizeOp::Lt => secs < rhs,
        SizeOp::Le => secs <= rhs,
        SizeOp::Gt => secs > rhs,
        SizeOp::Ge => secs >= rhs,
        SizeOp::Eq => secs == rhs,
    }
}
