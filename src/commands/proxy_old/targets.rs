use super::*;

#[derive(Clone)]
pub(super) struct ProxyTarget {
    pub(super) label: String,
    pub(super) target: Option<String>,
}

pub(super) fn default_proxy_targets() -> Vec<ProxyTarget> {
    vec![
        ProxyTarget {
            label: "proxy".to_string(),
            target: None,
        },
        ProxyTarget {
            label: "8.8.8.8".to_string(),
            target: Some("8.8.8.8:80".to_string()),
        },
        ProxyTarget {
            label: "1.1.1.1".to_string(),
            target: Some("1.1.1.1:80".to_string()),
        },
    ]
}

pub(super) fn parse_proxy_targets(raw: Option<&str>) -> Vec<ProxyTarget> {
    let raw = raw.unwrap_or("");
    let mut out = Vec::new();
    for part in raw.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        if t.eq_ignore_ascii_case("proxy") {
            out.push(ProxyTarget {
                label: "proxy".to_string(),
                target: None,
            });
            continue;
        }
        let target = if t.contains(':') {
            t.to_string()
        } else {
            format!("{}:80", t)
        };
        let label = t.split(':').next().unwrap_or(t).to_string();
        out.push(ProxyTarget {
            label,
            target: Some(target),
        });
    }
    if out.is_empty() {
        default_proxy_targets()
    } else {
        out
    }
}

pub(super) fn parse_proxy_only(raw: Option<&str>) -> Result<Option<HashSet<String>>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let mut set = HashSet::new();
    for part in raw.split(',') {
        let t = part.trim().to_lowercase();
        if t.is_empty() {
            continue;
        }
        if t == "all" {
            return Ok(None);
        }
        match t.as_str() {
            "cargo" | "git" | "npm" | "msys2" => {
                set.insert(t);
            }
            _ => return Err(format!("Invalid --only value: {}", t)),
        }
    }
    if set.is_empty() {
        Ok(None)
    } else {
        Ok(Some(set))
    }
}

pub(super) fn want_only(only: Option<&HashSet<String>>, key: &str) -> bool {
    only.map(|s| s.contains(key)).unwrap_or(true)
}

