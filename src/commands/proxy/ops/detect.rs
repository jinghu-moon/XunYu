use crate::cli::{ProxyDetectCmd, ProxyStatusCmd};
use crate::output::CliResult;

use super::super::env::get_system_proxy_url;
use super::super::test::run_proxy_tests;
use super::format::{render_detect, render_proxy_tests, render_status, resolve_format};
use super::state::{FALLBACK_PROXY, collect_proxy_status};

pub(crate) fn cmd_proxy_detect(args: ProxyDetectCmd) -> CliResult {
    let (url, used_fallback) = get_system_proxy_url(FALLBACK_PROXY);
    let format = resolve_format(&args.format)?;
    let enabled = !used_fallback;
    render_detect(format, enabled, &url)
}

pub(crate) fn cmd_proxy_status(args: ProxyStatusCmd) -> CliResult {
    let format = resolve_format(&args.format)?;
    let snapshot = collect_proxy_status();
    render_status(format, &snapshot.rows)?;

    if let Some(proxy_url) = snapshot.env_proxy {
        let results = run_proxy_tests(&proxy_url);
        render_proxy_tests(results);
    }

    Ok(())
}
