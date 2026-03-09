use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::alias::config::Config;

const APP_PATHS_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\App Paths";
const MANAGED_MARKER: &str = "XUN_ALIAS_MANAGED";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegisteredEntry {
    pub(crate) name: String,
    pub(crate) exe_path: String,
}

pub(crate) fn register(name: &str, exe_path: &str) -> Result<()> {
    #[cfg(windows)]
    {
        use winreg::{RegKey, enums::*};

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let subkey = format!(r"{}\{}.exe", APP_PATHS_SUBKEY, name);
        let (key, _) = hkcu
            .create_subkey(&subkey)
            .with_context(|| format!("Failed to create registry subkey: {subkey}"))?;

        key.set_value("", &exe_path)
            .context("Failed to set App Paths default value")?;
        if let Some(parent) = Path::new(exe_path).parent() {
            let parent_str = parent.to_string_lossy().to_string();
            key.set_value("Path", &parent_str)
                .context("Failed to set App Paths Path value")?;
        }
        key.set_value(MANAGED_MARKER, &"1")
            .context("Failed to set xun App Paths marker")?;
    }
    #[cfg(not(windows))]
    {
        let _ = (name, exe_path);
    }
    Ok(())
}

pub(crate) fn unregister(name: &str) -> Result<()> {
    #[cfg(windows)]
    {
        use winreg::{RegKey, enums::*};

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let subkey = format!(r"{}\{}.exe", APP_PATHS_SUBKEY, name);
        let Ok(key) = hkcu.open_subkey(&subkey) else {
            return Ok(());
        };

        let marker: String = key.get_value(MANAGED_MARKER).unwrap_or_default();
        if marker != "1" {
            bail!("Refusing to delete non-xun App Paths entry: {name}");
        }
        hkcu.delete_subkey_all(&subkey)
            .with_context(|| format!("Failed to delete App Paths subkey: {subkey}"))?;
    }
    #[cfg(not(windows))]
    {
        let _ = name;
    }
    Ok(())
}

pub(crate) fn list_registered() -> Result<Vec<RegisteredEntry>> {
    #[cfg(windows)]
    {
        use winreg::{RegKey, enums::*};

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(root) = hkcu.open_subkey(APP_PATHS_SUBKEY) else {
            return Ok(Vec::new());
        };

        let mut entries = Vec::new();
        for key_name in root.enum_keys().flatten() {
            let Ok(sub) = root.open_subkey(&key_name) else {
                continue;
            };
            let marker: String = sub.get_value(MANAGED_MARKER).unwrap_or_default();
            if marker != "1" {
                continue;
            }
            let exe_path: String = sub.get_value("").unwrap_or_default();
            let name = key_name.trim_end_matches(".exe").to_string();
            entries.push(RegisteredEntry { name, exe_path });
        }
        Ok(entries)
    }
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}

pub(crate) fn sync_apppaths(cfg: &Config) -> Result<(usize, usize)> {
    let mut registered = 0usize;
    let mut removed = 0usize;

    for (name, app) in &cfg.app {
        if app.register_apppaths {
            register(name, &app.exe)
                .with_context(|| format!("register app path failed: {name}"))?;
            registered += 1;
        }
    }

    for entry in list_registered()? {
        let keep = cfg
            .app
            .get(&entry.name)
            .map(|a| a.register_apppaths)
            .unwrap_or(false);
        if !keep {
            unregister(&entry.name)
                .with_context(|| format!("unregister app path failed: {}", entry.name))?;
            removed += 1;
        }
    }
    Ok((registered, removed))
}
