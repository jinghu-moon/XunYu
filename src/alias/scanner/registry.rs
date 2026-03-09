use std::path::Path;

use super::{AppEntry, Source, auto_alias, cache, is_utility_exe};

pub(crate) fn scan_registry(no_cache: bool) -> Vec<AppEntry> {
    let fingerprint = registry_fingerprint();
    if !no_cache && let Some(v) = cache::load_source("registry", 24 * 3600, Some(&fingerprint)) {
        return v;
    }

    #[cfg(windows)]
    let list = scan_registry_windows();
    #[cfg(not(windows))]
    let list = Vec::new();

    if !no_cache {
        cache::store_source("registry", &list, Some(&fingerprint));
    }
    list
}

#[cfg(windows)]
fn scan_registry_windows() -> Vec<AppEntry> {
    use winreg::{RegKey, enums::*};

    const KEYS: &[&str] = &[
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    let mut list = Vec::new();
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
                    extract_exe_from_display_icon(&display_icon).or_else(|| {
                        let install_location: String =
                            sub.get_value("InstallLocation").unwrap_or_default();
                        guess_exe_from_install_location(&install_location, &display_name)
                    })
                };
                let Some(exe_path) = exe_path else {
                    continue;
                };

                let exe_name = Path::new(&exe_path)
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or_default();
                if is_utility_exe(exe_name) {
                    continue;
                }

                list.push(AppEntry {
                    name: auto_alias(&display_name),
                    display_name,
                    exe_path,
                    source: Source::Registry,
                });
            }
        }
    }
    list
}

#[cfg(windows)]
fn extract_exe_from_display_icon(raw: &str) -> Option<String> {
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
        && p.exists()
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
        if is_utility_exe(&name) {
            continue;
        }
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

#[cfg(windows)]
fn registry_fingerprint() -> String {
    use winreg::{RegKey, enums::*};

    const KEYS: &[&str] = &[
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    let mut out = String::new();
    for (hive_name, hive, is_hkcu) in [
        ("hklm", RegKey::predef(HKEY_LOCAL_MACHINE), false),
        ("hkcu", RegKey::predef(HKEY_CURRENT_USER), true),
    ] {
        for key in KEYS {
            if is_hkcu && key.contains("WOW6432Node") {
                continue;
            }
            let stamp = hive
                .open_subkey(key)
                .ok()
                .and_then(|k| k.query_info().ok())
                .map(|info| {
                    ((info.last_write_time.dwHighDateTime as u64) << 32)
                        | (info.last_write_time.dwLowDateTime as u64)
                })
                .unwrap_or(0);
            out.push_str(hive_name);
            out.push(':');
            out.push_str(key);
            out.push('=');
            out.push_str(&stamp.to_string());
            out.push(';');
        }
    }
    out
}

#[cfg(not(windows))]
fn registry_fingerprint() -> String {
    "non-windows".to_string()
}
