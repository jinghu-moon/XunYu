use super::*;

pub(super) fn to_wide_null(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}

pub(super) fn wide_cstr_to_string(buf: &[u16]) -> Option<String> {
    let len = buf.iter().position(|&c| c == 0)?;
    let os = std::ffi::OsString::from_wide(&buf[..len]);
    Some(os.to_string_lossy().into_owned())
}

pub(super) fn filetime_to_secs(ft: FILETIME) -> Option<i64> {
    let ticks = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
    if ticks == 0 {
        return None;
    }
    const EPOCH_DIFF_100NS: u64 = 11644473600u64 * 10_000_000u64;
    if ticks < EPOCH_DIFF_100NS {
        return None;
    }
    Some(((ticks - EPOCH_DIFF_100NS) / 10_000_000u64) as i64)
}

pub(super) fn filetime_ticks_to_secs(ticks: i64) -> Option<i64> {
    if ticks <= 0 {
        return None;
    }
    let ticks = ticks as u64;
    const EPOCH_DIFF_100NS: u64 = 11644473600u64 * 10_000_000u64;
    if ticks < EPOCH_DIFF_100NS {
        return None;
    }
    Some(((ticks - EPOCH_DIFF_100NS) / 10_000_000u64) as i64)
}
