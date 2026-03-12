use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_READ, KEY_SET_VALUE, REG_DWORD,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};

use crate::output::CliError;

const PERSONALIZE_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Light => "亮色",
            Self::Dark => "暗色",
        }
    }
}

pub(crate) fn get_current_theme() -> ThemeMode {
    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        let key_wide = to_wide(PERSONALIZE_KEY);
        if RegOpenKeyExW(HKEY_CURRENT_USER, key_wide.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return ThemeMode::Light;
        }
        let value_name = to_wide("AppsUseLightTheme");
        let mut data = 0u32;
        let mut data_size = std::mem::size_of::<u32>() as u32;
        let _ = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut data as *mut u32 as *mut u8,
            &mut data_size,
        );
        let _ = RegCloseKey(hkey);
        if data == 1 { ThemeMode::Light } else { ThemeMode::Dark }
    }
}

pub(crate) fn set_theme(mode: &ThemeMode) -> Result<(), CliError> {
    let value: u32 = match mode {
        ThemeMode::Light => 1,
        ThemeMode::Dark => 0,
    };
    let value_bytes = value.to_le_bytes();

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        let key_wide = to_wide(PERSONALIZE_KEY);
        if RegOpenKeyExW(HKEY_CURRENT_USER, key_wide.as_ptr(), 0, KEY_SET_VALUE, &mut hkey) != 0 {
            return Err(CliError::new(2, "无法打开主题配置"));
        }

        for name in ["AppsUseLightTheme", "SystemUsesLightTheme"] {
            let name_wide = to_wide(name);
            let _ = RegSetValueExW(
                hkey,
                name_wide.as_ptr(),
                0,
                REG_DWORD,
                value_bytes.as_ptr(),
                value_bytes.len() as u32,
            );
        }
        let _ = RegCloseKey(hkey);

        let setting = to_wide("ImmersiveColorSet");
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            setting.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            200,
            std::ptr::null_mut(),
        );
    }
    Ok(())
}

pub(crate) fn toggle_theme() -> Result<ThemeMode, CliError> {
    let new_mode = match get_current_theme() {
        ThemeMode::Light => ThemeMode::Dark,
        ThemeMode::Dark => ThemeMode::Light,
    };
    set_theme(&new_mode)?;
    Ok(new_mode)
}

pub(crate) struct ThemeSchedule {
    pub(crate) light_at: Option<String>,
    pub(crate) dark_at: Option<String>,
    cancel_tx: Option<Sender<()>>,
}

impl ThemeSchedule {
    pub(crate) fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self { light_at: None, dark_at: None, cancel_tx: None }))
    }

    pub(crate) fn start(
        schedule: &Arc<Mutex<Self>>,
        light_at: String,
        dark_at: String,
    ) -> Result<(), CliError> {
        Self::stop(schedule);

        let light_dur = crate::desktop::awake::parse_expire_at(&light_at)?;
        let dark_dur = crate::desktop::awake::parse_expire_at(&dark_at)?;

        let (tx, rx) = channel::<()>();
        std::thread::spawn(move || {
            let mut events: Vec<(Duration, ThemeMode)> = vec![
                (light_dur, ThemeMode::Light),
                (dark_dur, ThemeMode::Dark),
            ];
            events.sort_by_key(|(d, _)| *d);

            for (dur, mode) in events {
                match rx.recv_timeout(dur) {
                    Ok(_) => return,
                    Err(_) => {
                        let _ = set_theme(&mode);
                    }
                }
            }
        });

        let mut s = schedule.lock().unwrap();
        s.light_at = Some(light_at);
        s.dark_at = Some(dark_at);
        s.cancel_tx = Some(tx);
        Ok(())
    }

    pub(crate) fn stop(schedule: &Arc<Mutex<Self>>) {
        let mut s = schedule.lock().unwrap();
        if let Some(tx) = s.cancel_tx.take() {
            let _ = tx.send(());
        }
        s.light_at = None;
        s.dark_at = None;
    }
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
