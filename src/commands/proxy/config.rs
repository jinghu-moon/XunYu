use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::util::has_cmd;

pub(crate) fn parse_proxy_only(raw: Option<&str>) -> Result<Option<HashSet<String>>, String> {
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

fn want_only(only: Option<&HashSet<String>>, key: &str) -> bool {
    only.map(|s| s.contains(key)).unwrap_or(true)
}

fn cargo_config_path() -> PathBuf {
    let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    Path::new(&home).join(".cargo/config.toml")
}

pub(crate) fn read_cargo_proxy() -> Option<String> {
    let path = cargo_config_path();
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    let mut in_http = false;
    for line in content.lines() {
        let tr = line.trim();
        if tr == "[http]" {
            in_http = true;
            continue;
        }
        if tr.starts_with('[') {
            in_http = false;
        }
        if in_http && tr.starts_with("proxy") {
            let parts: Vec<&str> = tr.split('=').collect();
            if parts.len() == 2 {
                return Some(parts[1].trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn msys2_proxy_path(root_override: Option<&str>) -> Option<PathBuf> {
    let userprofile = env::var("USERPROFILE").unwrap_or_default();
    let roots = vec![
        root_override.map(String::from),
        env::var("MSYS2_ROOT").ok(),
        Some(r"C:\msys64".to_string()),
        Some(r"C:\msys32".to_string()),
        Some(format!(r"{}\AppData\Local\msys64", userprofile)),
    ];

    roots
        .into_iter()
        .flatten()
        .map(|s| Path::new(&s).join(r"etc\profile.d\proxy.sh"))
        .find(|p| p.parent().map(|pa| pa.exists()).unwrap_or(false))
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ProxyState {
    pub url: String,
    pub noproxy: Option<String>,
}

fn proxy_state_path() -> PathBuf {
    let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    Path::new(&home).join(".xun.proxy.json")
}

pub(crate) fn load_proxy_state() -> Option<ProxyState> {
    let path = proxy_state_path();
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub(crate) fn save_proxy_state(url: &str, noproxy: &str) {
    let state = ProxyState {
        url: url.to_string(),
        noproxy: Some(noproxy.to_string()),
    };
    if let Ok(json) = serde_json::to_string(&state) {
        let _ = fs::write(proxy_state_path(), json);
    }
}

pub(crate) fn set_proxy(
    url: &str,
    noproxy: &str,
    msys2: Option<&str>,
    only: Option<&HashSet<String>>,
) {
    if want_only(only, "cargo") {
        let cargo_path = cargo_config_path();
        if let Some(p) = cargo_path.parent() {
            fs::create_dir_all(p).ok();
        }

        let content = fs::read_to_string(&cargo_path).unwrap_or_default();
        let new_line = format!("proxy = \"{}\"", url);
        let updated = if content.contains("[http]") {
            let mut lines: Vec<String> = content.lines().map(String::from).collect();
            let mut in_http = false;
            let mut found = false;
            let mut insert_at = None;
            for (i, line) in lines.iter_mut().enumerate() {
                let tr = line.trim();
                if tr == "[http]" {
                    in_http = true;
                    insert_at = Some(i + 1);
                    continue;
                }
                if tr.starts_with('[') {
                    in_http = false;
                }
                if in_http && tr.starts_with("proxy") {
                    *line = new_line.clone();
                    found = true;
                    break;
                }
            }
            if !found {
                if let Some(pos) = insert_at {
                    lines.insert(pos, new_line);
                } else {
                    lines.push("[http]".into());
                    lines.push(new_line);
                }
            }
            lines.join("\n")
        } else {
            format!("{}\n[http]\n{}\n", content, new_line)
        };
        fs::write(&cargo_path, updated).ok();
        ui_println!("ok:cargo");
    } else {
        ui_println!("skip:cargo (only)");
    }

    if want_only(only, "msys2") {
        if let Some(dest) = msys2_proxy_path(msys2) {
            let script = format!(
                "# managed by xun\nexport http_proxy=\"{url}\"\nexport https_proxy=\"{url}\"\nexport ftp_proxy=\"{url}\"\nexport no_proxy=\"{noproxy}\"\nexport HTTP_PROXY=\"{url}\"\nexport HTTPS_PROXY=\"{url}\"\nexport NO_PROXY=\"{noproxy}\"\n"
            );
            fs::write(&dest, script.replace("\r\n", "\n")).ok();
            ui_println!("ok:msys2");
        } else {
            ui_println!("skip:msys2 (not found)");
        }
    } else {
        ui_println!("skip:msys2 (only)");
    }

    if want_only(only, "git") {
        if has_cmd("git") {
            Command::new("git")
                .args(["config", "--global", "http.proxy", url])
                .output()
                .ok();
            Command::new("git")
                .args(["config", "--global", "https.proxy", url])
                .output()
                .ok();
            ui_println!("ok:git");
        } else {
            ui_println!("skip:git (not found)");
        }
    } else {
        ui_println!("skip:git (only)");
    }

    if want_only(only, "npm") {
        if has_cmd("npm") {
            Command::new("npm")
                .args(["config", "set", "proxy", url])
                .output()
                .ok();
            Command::new("npm")
                .args(["config", "set", "https-proxy", url])
                .output()
                .ok();
            ui_println!("ok:npm");
        } else {
            ui_println!("skip:npm (not found)");
        }
    } else {
        ui_println!("skip:npm (only)");
    }
}

pub(crate) fn del_proxy(msys2: Option<&str>, only: Option<&HashSet<String>>) {
    if want_only(only, "cargo") {
        let cargo_path = cargo_config_path();
        if cargo_path.exists() {
            let content = fs::read_to_string(&cargo_path).unwrap_or_default();
            let updated: String = content
                .lines()
                .filter(|l| !l.trim().starts_with("proxy"))
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(&cargo_path, updated).ok();
            ui_println!("ok:cargo");
        }
    } else {
        ui_println!("skip:cargo (only)");
    }

    if want_only(only, "msys2") {
        if let Some(dest) = msys2_proxy_path(msys2) {
            fs::remove_file(dest).ok();
            ui_println!("ok:msys2");
        }
    } else {
        ui_println!("skip:msys2 (only)");
    }

    if want_only(only, "git") {
        if has_cmd("git") {
            Command::new("git")
                .args(["config", "--global", "--unset", "http.proxy"])
                .output()
                .ok();
            Command::new("git")
                .args(["config", "--global", "--unset", "https.proxy"])
                .output()
                .ok();
            ui_println!("ok:git");
        } else {
            ui_println!("skip:git (not found)");
        }
    } else {
        ui_println!("skip:git (only)");
    }

    if want_only(only, "npm") {
        if has_cmd("npm") {
            Command::new("npm")
                .args(["config", "delete", "proxy"])
                .output()
                .ok();
            Command::new("npm")
                .args(["config", "delete", "https-proxy"])
                .output()
                .ok();
            ui_println!("ok:npm");
        } else {
            ui_println!("skip:npm (not found)");
        }
    } else {
        ui_println!("skip:npm (only)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proxy_only_parses_known_values() {
        let only = parse_proxy_only(Some("cargo,git,npm,msys2"))
            .expect("ok")
            .expect("some");
        assert!(only.contains("cargo"));
        assert!(only.contains("git"));
        assert!(only.contains("npm"));
        assert!(only.contains("msys2"));
    }

    #[test]
    fn parse_proxy_only_all_returns_none() {
        let only = parse_proxy_only(Some("all")).expect("ok");
        assert!(only.is_none());
    }

    #[test]
    fn parse_proxy_only_invalid_returns_err() {
        let err = parse_proxy_only(Some("nope")).unwrap_err();
        assert!(err.contains("Invalid --only value"));
    }
}
