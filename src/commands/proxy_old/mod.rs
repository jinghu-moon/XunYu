use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use comfy_table::{Attribute, Cell, Color, Table};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

use crate::cli::{
    ProxyCmd, ProxyExecCmd, ProxyOffCmd, ProxyOnCmd, ProxyStatusCmd, ProxySubCommand,
};
use crate::output::{apply_pretty_table_style, emit_warning};
use crate::util::has_cmd;

mod detect;
mod exec;
mod probe;
mod set_del;
mod status;
mod targets;

use detect::get_system_proxy_url;
use probe::run_proxy_tests_with;
use set_del::{del_proxy, set_proxy};
use targets::{parse_proxy_only, parse_proxy_targets};

pub(crate) use detect::cmd_proxy_detect;
pub(crate) use exec::cmd_proxy_exec;
pub(crate) use status::cmd_proxy_status;

#[allow(dead_code)]
pub(super) fn legacy_exit(code: i32) -> ! {
    panic!("legacy_exit:{code}");
}

pub(super) fn legacy_error(code: i32, message: impl AsRef<str>, details: &[&str]) -> ! {
    let msg = message.as_ref();
    ui_println!("Error: {}", msg);
    if details.is_empty() {
        ui_println!("Hint: Run `xun --help` for usage.");
    } else {
        for d in details {
            ui_println!("{d}");
        }
    }
    legacy_exit(code);
}

fn out_env_set(key: &str, value: &str) {
    out_println!("__ENV_SET__:{}={}", key, value);
}

fn out_env_unset(key: &str) {
    out_println!("__ENV_UNSET__:{}", key);
}

pub(crate) fn cmd_proxy_on(args: ProxyOnCmd) {
    let fallback = "http://127.0.0.1:7897";
    let (auto_url, used_fallback) = get_system_proxy_url(fallback);
    let user_url = args.url;
    let user_provided = user_url.is_some();
    let mut proxy_url = user_url.unwrap_or(auto_url);
    if !proxy_url.starts_with("http://") && !proxy_url.starts_with("https://") {
        proxy_url = format!("http://{}", proxy_url);
    }

    if used_fallback && !user_provided {
        ui_println!("~ system proxy disabled, using fallback {}", proxy_url);
    } else if !user_provided {
        ui_println!("~ detected system proxy {}", proxy_url);
    }

    let noproxy = args.noproxy;
    out_env_set("HTTP_PROXY", &proxy_url);
    out_env_set("HTTPS_PROXY", &proxy_url);
    out_env_set("ALL_PROXY", &proxy_url);
    out_env_set("NO_PROXY", &noproxy);
    out_env_set("http_proxy", &proxy_url);
    out_env_set("https_proxy", &proxy_url);
    out_env_set("all_proxy", &proxy_url);
    out_env_set("no_proxy", &noproxy);

    set_proxy(&proxy_url, &noproxy, args.msys2.as_deref(), None);
}

pub(crate) fn cmd_proxy_off(args: ProxyOffCmd) {
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
}

pub(crate) fn cmd_proxy(args: ProxyCmd) {
    match args.cmd {
        ProxySubCommand::Set(a) => {
            let only = match parse_proxy_only(a.only.as_deref()) {
                Ok(v) => v,
                Err(e) => {
                    legacy_error(2, e, &["Fix: Use cargo|git|npm|msys2|all."]);
                }
            };
            set_proxy(&a.url, &a.noproxy, a.msys2.as_deref(), only.as_ref());
        }
        ProxySubCommand::Rm(a) => {
            let only = match parse_proxy_only(a.only.as_deref()) {
                Ok(v) => v,
                Err(e) => {
                    legacy_error(2, e, &["Fix: Use cargo|git|npm|msys2|all."]);
                }
            };
            del_proxy(a.msys2.as_deref(), only.as_ref());
        }
        ProxySubCommand::Show(_a) => {
            if has_cmd("git") {
                if let Ok(o) = Command::new("git")
                    .args(["config", "--global", "--get", "http.proxy"])
                    .output()
                {
                    out_println!("{}", String::from_utf8_lossy(&o.stdout).trim());
                }
            } else {
                emit_warning(
                    "git not found; skipping proxy get.",
                    &["Hint: Install Git to enable `proxy get`."],
                );
            }
        }
        ProxySubCommand::Detect(_a) => cmd_proxy_detect(),
        ProxySubCommand::Test(a) => {
            let targets = parse_proxy_targets(a.targets.as_deref());
            let timeout = Duration::from_secs(a.timeout.max(1));
            let jobs = a.jobs.max(1);
            for (label, result) in run_proxy_tests_with(&a.url, targets, timeout, jobs) {
                match result {
                    Ok(ms) => out_println!("{}\t{}\tok", label, ms),
                    Err(e) => out_println!("{}\t-1\t{}", label, e),
                }
            }
        }
    }
}
