use super::*;
use super::detect::get_system_proxy_url;

pub(crate) fn cmd_proxy_exec(args: ProxyExecCmd) {
    if args.cmd.is_empty() {
        legacy_error(
            2,
            "Missing command for px.",
            &["Fix: Usage: px <command> [args]."],
        );
    }

    let fallback = "http://127.0.0.1:7897";
    let (auto_url, _) = get_system_proxy_url(fallback);
    let mut proxy_url = args.url.unwrap_or(auto_url);
    if !proxy_url.starts_with("http://") && !proxy_url.starts_with("https://") {
        proxy_url = format!("http://{}", proxy_url);
    }

    let mut cmd = Command::new(&args.cmd[0]);
    if args.cmd.len() > 1 {
        cmd.args(&args.cmd[1..]);
    }
    cmd.env("HTTP_PROXY", &proxy_url)
        .env("HTTPS_PROXY", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .env("NO_PROXY", &args.noproxy)
        .env("http_proxy", &proxy_url)
        .env("https_proxy", &proxy_url)
        .env("all_proxy", &proxy_url)
        .env("no_proxy", &args.noproxy);

    match cmd.status() {
        Ok(status) => {
            legacy_exit(status.code().unwrap_or(1));
        }
        Err(e) => {
            legacy_error(1, format!("px failed: {e}"), &["Hint: Check the command path and arguments."]);
        }
    }
}

