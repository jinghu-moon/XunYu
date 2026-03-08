use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::{AppEntry, Source, auto_alias, cache, is_utility_exe};

pub(crate) fn scan_startmenu(no_cache: bool) -> Vec<AppEntry> {
    let fingerprint = startmenu_fingerprint();
    if !no_cache {
        if let Some(v) = cache::load_source("startmenu", 24 * 3600, Some(&fingerprint)) {
            return v;
        }
    }

    #[cfg(windows)]
    let list = scan_startmenu_windows();
    #[cfg(not(windows))]
    let list = Vec::new();

    if !no_cache {
        cache::store_source("startmenu", &list, Some(&fingerprint));
    }
    list
}

#[cfg(windows)]
fn scan_startmenu_windows() -> Vec<AppEntry> {
    let mut out = Vec::new();
    for dir in startmenu_dirs() {
        collect_lnk_recursive(&dir, &mut out);
    }
    out
}

#[cfg(windows)]
fn startmenu_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        dirs.push(
            PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    dirs.push(PathBuf::from(
        r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs",
    ));
    dirs
}

#[cfg(windows)]
fn startmenu_fingerprint() -> String {
    let mut out = String::new();
    for dir in startmenu_dirs() {
        out.push_str(&dir.to_string_lossy().to_ascii_lowercase());
        out.push('=');
        let secs = dir
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        out.push_str(&secs.to_string());
        out.push(';');
    }
    out
}

#[cfg(not(windows))]
fn startmenu_fingerprint() -> String {
    "non-windows".to_string()
}

#[cfg(windows)]
fn collect_lnk_recursive(dir: &Path, out: &mut Vec<AppEntry>) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for item in read.flatten() {
        let path = item.path();
        if path.is_dir() {
            collect_lnk_recursive(&path, out);
            continue;
        }
        let is_lnk = path
            .extension()
            .and_then(|v| v.to_str())
            .map(|v| v.eq_ignore_ascii_case("lnk"))
            .unwrap_or(false);
        if !is_lnk {
            continue;
        }
        if let Some(entry) = process_lnk(&path) {
            out.push(entry);
        }
    }
}

#[cfg(windows)]
fn process_lnk(lnk_path: &Path) -> Option<AppEntry> {
    let target = resolve_lnk(lnk_path)?;
    if !target.to_ascii_lowercase().ends_with(".exe") {
        return None;
    }
    if !Path::new(&target).exists() {
        return None;
    }
    let exe_name = Path::new(&target)
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    if is_utility_exe(exe_name) {
        return None;
    }
    let display_name = lnk_path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("Unknown")
        .to_string();
    Some(AppEntry {
        name: auto_alias(&display_name),
        display_name,
        exe_path: target,
        source: Source::StartMenu,
    })
}

#[cfg(windows)]
fn resolve_lnk(lnk_path: &Path) -> Option<String> {
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{Interface, PCWSTR};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let shell_link: IShellLinkW =
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist: IPersistFile = shell_link.cast().ok()?;

        let mut wide: Vec<u16> = lnk_path
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .collect();
        wide.push(0);

        persist.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;
        let mut buffer = vec![0u16; MAX_PATH as usize];
        shell_link
            .GetPath(&mut buffer, std::ptr::null_mut(), 0)
            .ok()?;
        let target = String::from_utf16_lossy(&buffer)
            .trim_end_matches('\0')
            .trim()
            .to_string();
        if target.is_empty() {
            None
        } else {
            Some(target)
        }
    }
}
