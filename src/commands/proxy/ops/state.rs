use std::env;
use std::process::Command;

use crate::config::GlobalConfig;
use crate::util::has_cmd;

use super::super::config::{ProxyState, read_cargo_proxy};
use super::super::env::get_system_proxy_url;

pub(super) const FALLBACK_PROXY: &str = "http://127.0.0.1:7897";

#[derive(Clone)]
pub(super) struct ToolStatus {
    pub(super) tool: &'static str,
    pub(super) enabled: bool,
    pub(super) address: String,
    pub(super) note: String,
}

pub(super) struct ProxyStatusSnapshot {
    pub(super) env_proxy: Option<String>,
    pub(super) rows: Vec<ToolStatus>,
}

pub(super) fn resolve_proxy_url_and_noproxy(
    user_url: Option<String>,
    user_noproxy: &str,
    cfg: &GlobalConfig,
    saved: &Option<ProxyState>,
) -> (String, String, bool) {
    const DEFAULT_NOPROXY: &str = "localhost,127.0.0.1,::1,.local";
    let mut used_fallback = false;
    let mut proxy_url = if let Some(url) = user_url {
        url
    } else if let Some(state) = saved {
        state.url.clone()
    } else if let Some(url) = &cfg.proxy.default_url {
        url.clone()
    } else {
        let (auto_url, fallback_used) = get_system_proxy_url(FALLBACK_PROXY);
        used_fallback = fallback_used;
        auto_url
    };
    if !proxy_url.starts_with("http://") && !proxy_url.starts_with("https://") {
        proxy_url = format!("http://{}", proxy_url);
    }

    let mut noproxy = user_noproxy.to_string();
    if noproxy == DEFAULT_NOPROXY {
        if let Some(state) = saved {
            if let Some(np) = &state.noproxy {
                noproxy = np.clone();
            }
        } else if let Some(np) = &cfg.proxy.noproxy {
            noproxy = np.clone();
        }
    }

    (proxy_url, noproxy, used_fallback)
}

pub(super) fn collect_proxy_status() -> ProxyStatusSnapshot {
    let env_proxy = env::var("HTTP_PROXY")
        .or_else(|_| env::var("http_proxy"))
        .ok();
    let env_noproxy = env::var("NO_PROXY").or_else(|_| env::var("no_proxy")).ok();

    let has_git = has_cmd("git");
    let has_npm = has_cmd("npm");

    let git_proxy = if has_git {
        Command::new("git")
            .args(["config", "--global", "--get", "http.proxy"])
            .output()
            .ok()
            .and_then(|o| {
                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if v.is_empty() || v == "null" {
                    None
                } else {
                    Some(v)
                }
            })
    } else {
        None
    };

    let npm_proxy = if has_npm {
        Command::new("npm")
            .args(["config", "get", "proxy"])
            .output()
            .ok()
            .and_then(|o| {
                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if v.is_empty() || v == "null" {
                    None
                } else {
                    Some(v)
                }
            })
    } else {
        None
    };

    let cargo_proxy = read_cargo_proxy();

    let rows = vec![
        ToolStatus {
            tool: "Env",
            enabled: env_proxy.is_some(),
            address: env_proxy.clone().unwrap_or_else(|| "-".into()),
            note: env_noproxy.unwrap_or_else(|| "-".into()),
        },
        ToolStatus {
            tool: "Git",
            enabled: git_proxy.is_some(),
            address: git_proxy.clone().unwrap_or_else(|| "-".into()),
            note: if has_git {
                "".to_string()
            } else {
                "not found".to_string()
            },
        },
        ToolStatus {
            tool: "npm",
            enabled: npm_proxy.is_some(),
            address: npm_proxy.clone().unwrap_or_else(|| "-".into()),
            note: if has_npm {
                "".to_string()
            } else {
                "not found".to_string()
            },
        },
        ToolStatus {
            tool: "Cargo",
            enabled: cargo_proxy.is_some(),
            address: cargo_proxy.clone().unwrap_or_else(|| "-".into()),
            note: if cargo_proxy.is_some() {
                "config.toml".to_string()
            } else {
                "".to_string()
            },
        },
    ];

    ProxyStatusSnapshot { env_proxy, rows }
}
