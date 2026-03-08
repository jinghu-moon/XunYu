use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, GetLastError};
use windows_sys::Win32::Storage::FileSystem::QueryDosDeviceW;

pub(super) fn collect_device_map() -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for drive in b'A'..=b'Z' {
        let letter = drive as char;
        let drive_name = format!("{letter}:");
        let Some(devices) = query_dos_devices(&drive_name) else {
            continue;
        };
        let normalized: Vec<String> = devices
            .into_iter()
            .map(|d| normalize_path_like(&d))
            .filter(|d| !d.is_empty())
            .collect();
        if !normalized.is_empty() {
            map.insert(drive_name, normalized);
        }
    }
    map
}

fn query_dos_devices(drive: &str) -> Option<Vec<String>> {
    let drive_wide = to_wide_null(drive);
    let mut cap = 512usize;
    loop {
        let mut buf = vec![0u16; cap];
        let len =
            unsafe { QueryDosDeviceW(drive_wide.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) };
        if len == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_INSUFFICIENT_BUFFER {
                cap = cap.saturating_mul(2);
                if cap > (1usize << 16) {
                    return None;
                }
                continue;
            }
            return None;
        }

        let mut out = Vec::new();
        let mut start = 0usize;
        let used = len as usize;
        for i in 0..used {
            if buf[i] == 0 {
                if i > start {
                    out.push(String::from_utf16_lossy(&buf[start..i]));
                }
                if i + 1 < used && buf[i + 1] == 0 {
                    break;
                }
                start = i + 1;
            }
        }
        return Some(out);
    }
}

pub(super) fn dos_to_nt_paths(
    dos_path: &str,
    device_map: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    if dos_path.len() < 3 || dos_path.as_bytes()[1] != b':' {
        return Vec::new();
    }

    let drive = dos_path[..2].to_ascii_uppercase();
    let rest = &dos_path[2..];
    let Some(prefixes) = device_map.get(&drive) else {
        return Vec::new();
    };
    prefixes
        .iter()
        .map(|prefix| format!("{prefix}{rest}"))
        .map(|p| normalize_path_like(&p))
        .collect()
}

pub(super) fn nt_to_dos_path(
    nt_path: &str,
    device_map: &HashMap<String, Vec<String>>,
) -> Option<String> {
    if let Some(rest) = strip_prefix_ascii_insensitive(nt_path, r"\??\") {
        return Some(normalize_path_like(rest));
    }
    if let Some(rest) = strip_prefix_ascii_insensitive(nt_path, r"\device\mup\") {
        return Some(normalize_path_like(&format!(r"\\{rest}")));
    }

    for (drive, prefixes) in device_map {
        for prefix in prefixes {
            if let Some(suffix) = strip_prefix_ascii_insensitive(nt_path, prefix) {
                return Some(normalize_path_like(&format!("{drive}{suffix}")));
            }
        }
    }
    None
}

pub(super) fn normalize_path_like(input: &str) -> String {
    let mut s = input.trim().replace('/', "\\");

    if let Some(rest) = strip_prefix_ascii_insensitive(&s, r"\\?\UNC\") {
        s = format!(r"\\{rest}");
    } else if let Some(rest) = strip_prefix_ascii_insensitive(&s, r"\\?\") {
        s = rest.to_string();
    } else if let Some(rest) = strip_prefix_ascii_insensitive(&s, r"\??\") {
        s = rest.to_string();
    }

    // Keep root forms like C:\ and \\server\share\ intact.
    while s.ends_with('\\') && s.len() > 3 && !looks_like_unc_root(&s) {
        s.pop();
    }

    if s.len() >= 2 && s.as_bytes()[1] == b':' {
        let drive = (s.as_bytes()[0] as char).to_ascii_uppercase();
        s.replace_range(0..1, &drive.to_string());
    }

    s
}

pub(super) fn looks_like_unc_root(path: &str) -> bool {
    if !path.starts_with(r"\\") {
        return false;
    }
    let parts: Vec<&str> = path.trim_start_matches('\\').split('\\').collect();
    parts.len() <= 2
}

pub(super) fn strip_prefix_ascii_insensitive<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    if !head.eq_ignore_ascii_case(prefix) {
        return None;
    }
    s.get(prefix.len()..)
}

fn to_wide_null(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
