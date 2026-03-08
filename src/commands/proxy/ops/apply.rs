use std::process::Command;
use std::time::Duration;

use crate::cli::{ProxyCmd, ProxyExecCmd, ProxyOffCmd, ProxyOnCmd, ProxySubCommand};
use crate::config;
use crate::output::emit_warning;
use crate::output::{CliError, CliResult};
use crate::util::has_cmd;

use super::super::config::{
    del_proxy, load_proxy_state, parse_proxy_only, save_proxy_state, set_proxy,
};
use super::super::env::{out_env_set, out_env_unset};
use super::super::test::{parse_proxy_targets, run_proxy_tests_with};
use super::detect::cmd_proxy_detect;
use super::state::resolve_proxy_url_and_noproxy;

pub(crate) fn cmd_proxy_on(args: ProxyOnCmd) -> CliResult {
    let cfg = config::load_config();
    let saved = load_proxy_state();
    let user_provided = args.url.is_some();

    let (proxy_url, noproxy, used_fallback) =
        resolve_proxy_url_and_noproxy(args.url, &args.noproxy, &cfg, &saved);

    if used_fallback && !user_provided {
        ui_println!("~ system proxy disabled, using fallback {}", proxy_url);
    } else if !user_provided && saved.is_some() {
        ui_println!("~ using saved proxy {}", proxy_url);
    } else if !user_provided && cfg.proxy.default_url.is_some() {
        ui_println!("~ using config proxy {}", proxy_url);
    } else if !user_provided {
        ui_println!("~ detected system proxy {}", proxy_url);
    }

    out_env_set("HTTP_PROXY", &proxy_url);
    out_env_set("HTTPS_PROXY", &proxy_url);
    out_env_set("ALL_PROXY", &proxy_url);
    out_env_set("NO_PROXY", &noproxy);
    out_env_set("http_proxy", &proxy_url);
    out_env_set("https_proxy", &proxy_url);
    out_env_set("all_proxy", &proxy_url);
    out_env_set("no_proxy", &noproxy);

    set_proxy(&proxy_url, &noproxy, args.msys2.as_deref(), None);

    if !args.no_test {
        let targets = parse_proxy_targets(Some("proxy"));
        let res = run_proxy_tests_with(&proxy_url, targets, Duration::from_secs(2), 1);
        let ok = res.get(0).and_then(|(_, r)| r.as_ref().ok()).copied();
        match ok {
            Some(ms) => ui_println!("Proxy connectivity: ✓ (latency: {ms}ms)"),
            None => {
                let msg = res
                    .get(0)
                    .and_then(|(_, r)| r.as_ref().err())
                    .map(|s| s.as_str())
                    .unwrap_or("failed");
                let detail = format!("Details: {msg}");
                emit_warning(
                    "Proxy connectivity check failed.",
                    &[
                        detail.as_str(),
                        "Hint: Re-run with `xun proxy test` for more details, or add `--no-test` to skip.",
                    ],
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn cmd_proxy_off(args: ProxyOffCmd) -> CliResult {
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
        out_env_unset(key);
    }
    del_proxy(args.msys2.as_deref(), None);
    Ok(())
}

pub(crate) fn cmd_proxy_exec(args: ProxyExecCmd) -> CliResult {
    if args.cmd.is_empty() {
        return Err(CliError::with_details(
            2,
            "usage: px <command> [args]".to_string(),
            &["Fix: Example: `xun px cargo test`."],
        ));
    }
    let cfg = config::load_config();
    let saved = load_proxy_state();
    let (proxy_url, noproxy, _) =
        resolve_proxy_url_and_noproxy(args.url, &args.noproxy, &cfg, &saved);

    let mut cmd = Command::new(&args.cmd[0]);
    if args.cmd.len() > 1 {
        cmd.args(&args.cmd[1..]);
    }
    cmd.env("HTTP_PROXY", &proxy_url)
        .env("HTTPS_PROXY", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .env("NO_PROXY", &noproxy)
        .env("http_proxy", &proxy_url)
        .env("https_proxy", &proxy_url)
        .env("all_proxy", &proxy_url)
        .env("no_proxy", &noproxy);

    let status = cmd
        .status()
        .map_err(|e| CliError::new(1, format!("px failed: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(CliError::new(
            status.code().unwrap_or(1),
            "px command failed",
        ))
    }
}

pub(crate) fn cmd_proxy(args: ProxyCmd) -> CliResult {
    match args.cmd {
        ProxySubCommand::Set(a) => {
            let only = parse_proxy_only(a.only.as_deref()).map_err(|e| CliError::new(2, e))?;
            set_proxy(&a.url, &a.noproxy, a.msys2.as_deref(), only.as_ref());
            save_proxy_state(&a.url, &a.noproxy);
            Ok(())
        }
        ProxySubCommand::Del(a) => {
            let only = parse_proxy_only(a.only.as_deref()).map_err(|e| CliError::new(2, e))?;
            del_proxy(a.msys2.as_deref(), only.as_ref());
            Ok(())
        }
        ProxySubCommand::Get(_a) => {
            if has_cmd("git") {
                if let Ok(o) = Command::new("git")
                    .args(["config", "--global", "--get", "http.proxy"])
                    .output()
                {
                    out_println!("{}", String::from_utf8_lossy(&o.stdout).trim());
                }
            } else {
                ui_println!("skip:git (not found)");
            }
            Ok(())
        }
        ProxySubCommand::Detect(a) => cmd_proxy_detect(a),
        ProxySubCommand::Test(a) => {
            let targets = parse_proxy_targets(a.targets.as_deref());
            let timeout = Duration::from_secs(a.timeout.max(1));
            let jobs = a.jobs.max(1);
            for (label, result) in run_proxy_tests_with(&a.url, targets, timeout, jobs) {
                match result {
                    Ok(ms) => out_println!("{}\t{}\tok", label, ms),
                    Err(e) => {
                        out_println!("{}\t-1\t{}", label, e);
                    }
                }
            }
            Ok(())
        }
    }
}
