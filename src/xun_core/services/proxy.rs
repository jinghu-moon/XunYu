//! Proxy 业务逻辑服务
//!
//! 封装代理配置的读写操作，供 CommandSpec 实现调用。

use std::process::Command;

use crate::commands::proxy::config::{
    del_proxy, load_proxy_state, parse_proxy_only, save_proxy_state, set_proxy,
};
use crate::util::has_cmd;
use crate::xun_core::error::XunError;
use crate::xun_core::proxy_cmd::ProxyInfo;

/// 读取当前代理配置。
pub fn show_proxy() -> Result<ProxyInfo, XunError> {
    // 优先从 git 读取
    if has_cmd("git") {
        if let Ok(o) = Command::new("git")
            .args(["config", "--global", "--get", "http.proxy"])
            .output()
        {
            let url = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !url.is_empty() {
                return Ok(ProxyInfo::new(url, "localhost,127.0.0.1", "git"));
            }
        }
    }

    // 回退到保存的状态
    if let Some(state) = load_proxy_state() {
        return Ok(ProxyInfo::new(
            state.url,
            state.noproxy.unwrap_or_else(|| "localhost,127.0.0.1".into()),
            "saved",
        ));
    }

    Ok(ProxyInfo::new("", "", "none"))
}

/// 设置代理配置。
pub fn set_proxy_service(
    url: &str,
    noproxy: &str,
    only: Option<&str>,
) -> Result<(), XunError> {
    let only_set = parse_proxy_only(only).map_err(|e| XunError::user(e))?;
    set_proxy(url, noproxy, None, only_set.as_ref());
    save_proxy_state(url, noproxy);
    Ok(())
}

/// 删除代理配置。
pub fn rm_proxy_service(only: Option<&str>) -> Result<(), XunError> {
    let only_set = parse_proxy_only(only).map_err(|e| XunError::user(e))?;
    del_proxy(None, only_set.as_ref());
    Ok(())
}

/// 获取当前代理 URL（用于 Operation preview）。
pub fn current_proxy_url() -> String {
    load_proxy_state()
        .map(|s| s.url)
        .unwrap_or_default()
}
