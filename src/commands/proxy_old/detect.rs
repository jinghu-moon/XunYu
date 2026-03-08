use super::*;

pub(super) fn get_system_proxy_url(fallback: &str) -> (String, bool) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    let key = hkcu.open_subkey(path);

    if let Ok(k) = key {
        let enabled: u32 = k.get_value("ProxyEnable").unwrap_or(0);
        let server: String = k.get_value("ProxyServer").unwrap_or_default();
        if enabled == 1 && !server.trim().is_empty() {
            let raw = server.trim();
            let candidate = if raw.contains("http=") {
                raw.split(';')
                    .find_map(|part| part.strip_prefix("http="))
                    .unwrap_or(raw)
                    .to_string()
            } else {
                raw.split(';').next().unwrap_or(raw).to_string()
            };
            let mut url = candidate;
            if !url.starts_with("http://") && !url.starts_with("https://") {
                url = format!("http://{}", url);
            }
            return (url, false);
        }
    }

    (fallback.to_string(), true)
}

pub(crate) fn cmd_proxy_detect() {
    let fallback = "http://127.0.0.1:7897";
    let (url, used_fallback) = get_system_proxy_url(fallback);
    if used_fallback {
        out_println!("disabled\t");
    } else {
        out_println!("enabled\t{}", url);
    }
}

