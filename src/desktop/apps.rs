#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct InstalledApp {
    pub name: String,
    pub path: String,
}

pub(crate) fn list_installed_apps() -> Vec<InstalledApp> {
    #[cfg(windows)]
    {
        list_installed_apps_windows()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn list_installed_apps_windows() -> Vec<InstalledApp> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

    let mut apps = Vec::new();

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    for dir in startmenu_dirs() {
        collect_shortcuts(&dir, &mut apps);
    }
    collect_registry_apps(&mut apps);
    collect_app_paths(&mut apps);

    apps.retain(|app| !is_excluded_app(app));
    apps.sort_by(|a, b| a.path.to_ascii_lowercase().cmp(&b.path.to_ascii_lowercase()));
    apps.dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));
    apps
}

#[cfg(windows)]
fn is_excluded_app(app: &InstalledApp) -> bool {
    if is_excluded_path(&app.path) {
        return true;
    }
    let name = app.name.to_ascii_lowercase();
    let exe = Path::new(&app.path)
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    is_excluded_label(&name) || is_excluded_label(&exe)
}

#[cfg(windows)]
fn is_excluded_path(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    if path.contains("\\windows\\installer\\") {
        return true;
    }
    if path.contains("\\package cache\\") {
        return true;
    }
    false
}

#[cfg(windows)]
fn is_excluded_label(label: &str) -> bool {
    const TOKENS: [&str; 15] = [
        "uninstall",
        "unins",
        "setup",
        "installer",
        "install",
        "update",
        "updater",
        "patch",
        "hotfix",
        "repair",
        "bootstrap",
        "vcredist",
        "redist",
        "redistributable",
        "msiexec",
    ];
    TOKENS.iter().any(|token| label.contains(token))
}

#[cfg(windows)]
fn startmenu_dirs() -> Vec<PathBuf> {
    use windows::Win32::UI::Shell::{CSIDL_COMMON_PROGRAMS, CSIDL_PROGRAMS};

    let mut dirs = Vec::new();
    if let Some(path) = get_special_folder_path(CSIDL_COMMON_PROGRAMS as i32) {
        dirs.push(path);
    }
    if let Some(path) = get_special_folder_path(CSIDL_PROGRAMS as i32) {
        dirs.push(path);
    }
    dirs
}

#[cfg(windows)]
fn get_special_folder_path(csidl: i32) -> Option<PathBuf> {
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::UI::Shell::{CSIDL_FLAG_CREATE, SHGetFolderPathW};

    unsafe {
        let mut buffer = [0u16; MAX_PATH as usize];
        if SHGetFolderPathW(
            None,
            csidl | CSIDL_FLAG_CREATE as i32,
            None,
            0,
            &mut buffer,
        )
        .is_ok()
        {
            let len = buffer
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(buffer.len());
            let path = String::from_utf16_lossy(&buffer[..len]).trim().to_string();
            if path.is_empty() {
                None
            } else {
                Some(PathBuf::from(path))
            }
        } else {
            None
        }
    }
}

#[cfg(windows)]
fn collect_shortcuts(dir: &Path, out: &mut Vec<InstalledApp>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for item in read.flatten() {
        let path = item.path();
        if path.is_dir() {
            collect_shortcuts(&path, out);
            continue;
        }
        let is_lnk = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("lnk"))
            .unwrap_or(false);
        if !is_lnk {
            continue;
        }
        let Some(target) = resolve_shortcut(&path) else {
            continue;
        };
        if !target.to_ascii_lowercase().ends_with(".exe") {
            continue;
        }
        if !Path::new(&target).exists() {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|v| v.to_str())
            .unwrap_or("Unknown")
            .to_string();
        out.push(InstalledApp { name, path: target });
    }
}

#[cfg(windows)]
fn resolve_shortcut(path: &Path) -> Option<String> {
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER, IPersistFile, STGM_READ};
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{Interface, PCWSTR};

    unsafe {
        let shell_link: IShellLinkW =
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist: IPersistFile = shell_link.cast().ok()?;

        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
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

#[cfg(windows)]
fn collect_registry_apps(out: &mut Vec<InstalledApp>) {
    use winreg::{RegKey, enums::*};

    const KEYS: &[&str] = &[
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    for (hive, is_hkcu) in [
        (RegKey::predef(HKEY_LOCAL_MACHINE), false),
        (RegKey::predef(HKEY_CURRENT_USER), true),
    ] {
        for key in KEYS {
            if is_hkcu && key.contains("WOW6432Node") {
                continue;
            }
            let Ok(root) = hive.open_subkey(key) else {
                continue;
            };
            for item in root.enum_keys().flatten() {
                let Ok(sub) = root.open_subkey(&item) else {
                    continue;
                };

                let system_component: u32 = sub.get_value("SystemComponent").unwrap_or(0);
                if system_component == 1 {
                    continue;
                }

                let display_name: String = sub.get_value("DisplayName").unwrap_or_default();
                if display_name.trim().is_empty() {
                    continue;
                }

                let exe_path = {
                    let display_icon: String = sub.get_value("DisplayIcon").unwrap_or_default();
                    extract_exe_from_string(&display_icon).or_else(|| {
                        let install_location: String =
                            sub.get_value("InstallLocation").unwrap_or_default();
                        guess_exe_from_install_location(&install_location, &display_name)
                    })
                };
                let Some(exe_path) = exe_path else {
                    continue;
                };
                if !Path::new(&exe_path).exists() {
                    continue;
                }

                out.push(InstalledApp {
                    name: display_name,
                    path: exe_path,
                });
            }
        }
    }
}

#[cfg(windows)]
fn collect_app_paths(out: &mut Vec<InstalledApp>) {
    use winreg::{RegKey, enums::*};

    const APP_PATHS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\App Paths";

    for hive in [RegKey::predef(HKEY_LOCAL_MACHINE), RegKey::predef(HKEY_CURRENT_USER)] {
        let Ok(root) = hive.open_subkey(APP_PATHS_KEY) else {
            continue;
        };
        for key_name in root.enum_keys().flatten() {
            let Ok(sub) = root.open_subkey(&key_name) else {
                continue;
            };
            let raw: String = sub.get_value("").unwrap_or_default();
            let Some(exe_path) = extract_exe_from_string(&raw) else {
                continue;
            };
            if !Path::new(&exe_path).exists() {
                continue;
            }
            let name = key_name.trim_end_matches(".exe").to_string();
            out.push(InstalledApp {
                name,
                path: exe_path,
            });
        }
    }
}

#[cfg(windows)]
fn extract_exe_from_string(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let first = if let Some(rest) = raw.strip_prefix('"') {
        let end = rest.find('"')?;
        &rest[..end]
    } else {
        raw.split(',').next().unwrap_or(raw).trim()
    };
    let p = Path::new(first);
    if p.extension()
        .and_then(|v| v.to_str())
        .map(|v| v.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
    {
        Some(first.to_string())
    } else {
        None
    }
}

#[cfg(windows)]
fn guess_exe_from_install_location(install_dir: &str, display_name: &str) -> Option<String> {
    let dir = Path::new(install_dir.trim());
    if !dir.is_dir() {
        return None;
    }
    let prefix = display_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(4)
        .collect::<String>()
        .to_ascii_lowercase();

    let mut best: Option<(i32, String)> = None;
    for file in std::fs::read_dir(dir).ok()?.flatten() {
        let path = file.path();
        if !path.is_file() {
            continue;
        }
        if path
            .extension()
            .and_then(|v| v.to_str())
            .map(|v| v.eq_ignore_ascii_case("exe"))
            != Some(true)
        {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let mut score = 1;
        if !prefix.is_empty() && name.starts_with(&prefix) {
            score = 10;
        }
        let candidate = path.to_string_lossy().to_string();
        match &best {
            None => best = Some((score, candidate)),
            Some((s, old)) => {
                if score > *s || (score == *s && candidate < *old) {
                    best = Some((score, candidate));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}
