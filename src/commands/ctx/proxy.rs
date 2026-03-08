use crate::cli::CtxSetCmd;
use crate::ctx_store::{CtxProxy, CtxProxyMode};
use crate::output::{CliError, CliResult};

pub(super) fn apply_proxy_updates(proxy: &mut CtxProxy, args: &CtxSetCmd) -> CliResult {
    if let Some(raw) = &args.proxy {
        let v = raw.trim();
        if v.eq_ignore_ascii_case("keep") {
            if proxy.mode == CtxProxyMode::Keep {
                proxy.url = None;
                proxy.noproxy = None;
            }
        } else if v.eq_ignore_ascii_case("off") {
            proxy.mode = CtxProxyMode::Off;
            proxy.url = None;
            proxy.noproxy = None;
        } else {
            proxy.mode = CtxProxyMode::Set;
            proxy.url = Some(normalize_proxy_url(v));
        }
    }

    if let Some(noproxy) = &args.noproxy {
        if !matches!(proxy.mode, CtxProxyMode::Set) {
            return Err(CliError::with_details(
                2,
                "--noproxy requires --proxy <url>.".to_string(),
                &["Fix: Use `--proxy <url> --noproxy <hosts>`."],
            ));
        }
        proxy.noproxy = Some(noproxy.to_string());
    }

    Ok(())
}

pub(super) fn proxy_summary(proxy: &CtxProxy) -> String {
    match proxy.mode {
        CtxProxyMode::Keep => "keep".to_string(),
        CtxProxyMode::Off => "off".to_string(),
        CtxProxyMode::Set => proxy.url.clone().unwrap_or_else(|| "set".to_string()),
    }
}

pub(super) fn normalize_proxy_url(raw: &str) -> String {
    if raw.starts_with("http://") || raw.starts_with("https://") {
        raw.to_string()
    } else {
        format!("http://{}", raw)
    }
}

pub(super) fn emit_proxy_set(url: &str, noproxy: &str) {
    for (k, v) in [
        ("HTTP_PROXY", url),
        ("HTTPS_PROXY", url),
        ("ALL_PROXY", url),
        ("NO_PROXY", noproxy),
        ("http_proxy", url),
        ("https_proxy", url),
        ("all_proxy", url),
        ("no_proxy", noproxy),
    ] {
        out_println!("__ENV_SET__:{k}={v}");
    }
}

pub(super) fn emit_proxy_off() {
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
    ] {
        out_println!("__ENV_UNSET__:{key}");
    }
}
