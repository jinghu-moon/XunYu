use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

pub(super) fn out_env_set(key: &str, value: &str) {
    out_println!("__ENV_SET__:{}={}", key, value);
}

pub(super) fn out_env_unset(key: &str) {
    out_println!("__ENV_UNSET__:{}", key);
}

fn resolve_proxy_url_from_registry_values(
    enabled: u32,
    server: &str,
    fallback: &str,
) -> (String, bool) {
    if enabled != 1 || server.trim().is_empty() {
        return (fallback.to_string(), true);
    }

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
    (url, false)
}

pub(super) fn get_system_proxy_url(fallback: &str) -> (String, bool) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    let key = hkcu.open_subkey(path);

    if let Ok(k) = key {
        let enabled: u32 = k.get_value("ProxyEnable").unwrap_or(0);
        let server: String = k.get_value("ProxyServer").unwrap_or_default();
        return resolve_proxy_url_from_registry_values(enabled, &server, fallback);
    }

    resolve_proxy_url_from_registry_values(0, "", fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_proxy_url_enabled_returns_url_and_no_fallback() {
        let (url, used_fallback) =
            resolve_proxy_url_from_registry_values(1, "127.0.0.1:7897", "http://fallback:1");
        assert_eq!(url, "http://127.0.0.1:7897");
        assert!(!used_fallback);
    }

    #[test]
    fn resolve_proxy_url_disabled_returns_fallback_and_used_fallback_true() {
        let (url, used_fallback) =
            resolve_proxy_url_from_registry_values(0, "127.0.0.1:7897", "http://fallback:1");
        assert_eq!(url, "http://fallback:1");
        assert!(used_fallback);
    }

    #[test]
    fn resolve_proxy_url_multi_protocol_prefers_http_part() {
        let (url, used_fallback) = resolve_proxy_url_from_registry_values(
            1,
            "http=proxy.local:7897;https=proxy.local:7898",
            "http://fallback:1",
        );
        assert_eq!(url, "http://proxy.local:7897");
        assert!(!used_fallback);
    }
}
