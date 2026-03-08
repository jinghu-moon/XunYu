use super::types::EnvVarKind;

pub fn infer_var_kind(name: &str, raw_value: &str) -> Option<EnvVarKind> {
    let value = raw_value.trim();
    if value.is_empty() {
        return None;
    }
    let upper_name = name.trim().to_ascii_uppercase();

    if is_secret_name(&upper_name) {
        return Some(EnvVarKind::Secret);
    }
    if is_boolean(value) {
        return Some(EnvVarKind::Boolean);
    }
    if is_integer(value) {
        return Some(EnvVarKind::Integer);
    }
    if is_float(value) {
        return Some(EnvVarKind::Float);
    }
    if looks_like_json(value) {
        return Some(EnvVarKind::Json);
    }
    if looks_like_email(value) {
        return Some(EnvVarKind::Email);
    }
    if looks_like_url(value) {
        return Some(EnvVarKind::Url);
    }
    if looks_like_version(value) {
        return Some(EnvVarKind::Version);
    }
    if looks_like_path_list(&upper_name, value) {
        return Some(EnvVarKind::PathList);
    }
    if looks_like_path(&upper_name, value) {
        return Some(EnvVarKind::Path);
    }
    None
}

fn is_secret_name(name: &str) -> bool {
    if name.contains("PASSWORD")
        || name.contains("PASSWD")
        || name.contains("SECRET")
        || name.contains("TOKEN")
        || name.contains("PRIVATE_KEY")
        || name.contains("API_KEY")
        || name.contains("ACCESS_KEY")
        || name.contains("AUTH_KEY")
    {
        return true;
    }

    name.ends_with("_KEY") && !name.contains("PATH")
}

fn is_boolean(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "true" | "false" | "1" | "0" | "yes" | "no" | "on" | "off"
    )
}

fn is_integer(value: &str) -> bool {
    value.parse::<i64>().is_ok()
}

fn is_float(value: &str) -> bool {
    value.contains('.') && value.parse::<f64>().is_ok()
}

fn looks_like_json(value: &str) -> bool {
    let t = value.trim();
    if !((t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']'))) {
        return false;
    }
    serde_json::from_str::<serde_json::Value>(t).is_ok()
}

fn looks_like_email(value: &str) -> bool {
    let t = value.trim();
    if t.contains(' ') {
        return false;
    }
    let Some((left, right)) = t.split_once('@') else {
        return false;
    };
    if left.is_empty() || right.is_empty() {
        return false;
    }
    right.contains('.')
}

fn looks_like_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ws://")
        || lower.starts_with("wss://")
        || lower.starts_with("ftp://")
}

fn looks_like_version(value: &str) -> bool {
    let trimmed = value.trim_start_matches('v');
    let core = trimmed
        .split_once('-')
        .map(|(head, _)| head)
        .unwrap_or(trimmed);
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts
        .iter()
        .all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
}

fn looks_like_path_list(upper_name: &str, value: &str) -> bool {
    if upper_name == "PATH" {
        return value.contains(';');
    }
    if !value.contains(';') {
        return false;
    }
    let segs = value
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if segs.len() < 2 {
        return false;
    }
    segs.iter().all(|seg| looks_like_path(upper_name, seg))
}

fn looks_like_path(upper_name: &str, value: &str) -> bool {
    if upper_name.ends_with("_PATH")
        || upper_name.ends_with("_DIR")
        || upper_name == "HOME"
        || upper_name == "USERPROFILE"
        || upper_name == "HOMEDRIVE"
        || upper_name == "HOMEPATH"
    {
        return true;
    }
    let t = value.trim();
    if t.starts_with("\\\\") {
        return true;
    }
    if t.starts_with("~/") || t.starts_with('/') || t.starts_with(".\\") || t.starts_with("./") {
        return true;
    }
    if t.len() >= 3 {
        let b = t.as_bytes();
        if b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') {
            return true;
        }
    }
    t.contains('\\') || t.contains('/')
}

#[cfg(test)]
mod tests {
    use super::{EnvVarKind, infer_var_kind};

    #[test]
    fn infer_secret() {
        assert_eq!(
            infer_var_kind("GITHUB_TOKEN", "abcd"),
            Some(EnvVarKind::Secret)
        );
    }

    #[test]
    fn infer_url() {
        assert_eq!(
            infer_var_kind("SERVICE_URL", "https://example.com"),
            Some(EnvVarKind::Url)
        );
    }

    #[test]
    fn infer_path_list() {
        assert_eq!(
            infer_var_kind("PATH", r"C:\a;C:\b"),
            Some(EnvVarKind::PathList)
        );
    }

    #[test]
    fn infer_json() {
        assert_eq!(
            infer_var_kind("CONFIG_JSON", r#"{"a":1}"#),
            Some(EnvVarKind::Json)
        );
    }

    #[test]
    fn infer_boolean() {
        assert_eq!(
            infer_var_kind("FEATURE_FLAG", "true"),
            Some(EnvVarKind::Boolean)
        );
    }
}
