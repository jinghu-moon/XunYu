use std::env;
use std::process::Command;
use std::time::Duration;

use comfy_table::{Attribute, Cell, Color, Table};

use crate::cli::{
    ProxyCmd, ProxyDetectCmd, ProxyExecCmd, ProxyOffCmd, ProxyOnCmd, ProxyStatusCmd,
    ProxySubCommand,
};
use crate::config;
use crate::model::{ListFormat, parse_list_format};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, emit_warning, prefer_table_output, print_table};
use crate::util::has_cmd;

use super::config::{
    del_proxy, load_proxy_state, parse_proxy_only, read_cargo_proxy, save_proxy_state, set_proxy,
};
use super::env::{get_system_proxy_url, out_env_set, out_env_unset};
use super::test::{parse_proxy_targets, run_proxy_tests, run_proxy_tests_with};

const FALLBACK_PROXY: &str = "http://127.0.0.1:7897";

fn resolve_proxy_url_and_noproxy(
    user_url: Option<String>,
    user_noproxy: &str,
    cfg: &crate::config::GlobalConfig,
    saved: &Option<super::config::ProxyState>,
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

pub(crate) fn cmd_proxy_detect(args: ProxyDetectCmd) -> CliResult {
    let (url, used_fallback) = get_system_proxy_url(FALLBACK_PROXY);
    let mut format = parse_list_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }
    let enabled = !used_fallback;
    if format == ListFormat::Json {
        let obj = serde_json::json!({
            "enabled": enabled,
            "url": if enabled { url.clone() } else { String::new() }
        });
        out_println!("{}", obj);
        return Ok(());
    }
    if format == ListFormat::Tsv {
        if enabled {
            out_println!("enabled\t{}", url);
        } else {
            out_println!("disabled\t");
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Status")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Address")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
    ]);
    table.add_row(vec![
        Cell::new(if enabled { "ENABLED" } else { "DISABLED" }).fg(if enabled {
            Color::Green
        } else {
            Color::DarkGrey
        }),
        Cell::new(if enabled { url.as_str() } else { "-" }).fg(Color::DarkGrey),
    ]);
    print_table(&table);
    Ok(())
}

pub(crate) fn cmd_proxy_status(args: ProxyStatusCmd) -> CliResult {
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

    let mut format = parse_list_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }

    let env_state = env_proxy.is_some();
    let env_addr = env_proxy.clone().unwrap_or_else(|| "-".into());
    let env_note = env_noproxy.unwrap_or_else(|| "-".into());

    let git_state = git_proxy.is_some();
    let git_addr = git_proxy.clone().unwrap_or_else(|| "-".into());
    let git_note = if has_git { "" } else { "not found" };

    let npm_state = npm_proxy.is_some();
    let npm_addr = npm_proxy.clone().unwrap_or_else(|| "-".into());
    let npm_note = if has_npm { "" } else { "not found" };

    let cargo_state = cargo_proxy.is_some();
    let cargo_addr = cargo_proxy.clone().unwrap_or_else(|| "-".into());
    let cargo_note = if cargo_state { "config.toml" } else { "" };

    if format == ListFormat::Json {
        let rows = vec![
            serde_json::json!({ "tool": "Env", "status": if env_state { "ON" } else { "OFF" }, "address": env_addr, "note": env_note }),
            serde_json::json!({ "tool": "Git", "status": if git_state { "ON" } else { "OFF" }, "address": git_addr, "note": git_note }),
            serde_json::json!({ "tool": "npm", "status": if npm_state { "ON" } else { "OFF" }, "address": npm_addr, "note": npm_note }),
            serde_json::json!({ "tool": "Cargo", "status": if cargo_state { "ON" } else { "OFF" }, "address": cargo_addr, "note": cargo_note }),
        ];
        out_println!("{}", serde_json::Value::Array(rows));
        return Ok(());
    }
    if format == ListFormat::Tsv {
        out_println!(
            "Env\t{}\t{}\t{}",
            if env_state { "ON" } else { "OFF" },
            env_addr,
            env_note
        );
        out_println!(
            "Git\t{}\t{}\t{}",
            if git_state { "ON" } else { "OFF" },
            git_addr,
            git_note
        );
        out_println!(
            "npm\t{}\t{}\t{}",
            if npm_state { "ON" } else { "OFF" },
            npm_addr,
            npm_note
        );
        out_println!(
            "Cargo\t{}\t{}\t{}",
            if cargo_state { "ON" } else { "OFF" },
            cargo_addr,
            cargo_note
        );
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Tool")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Status")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Address")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
        Cell::new("Note")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("Env"),
        Cell::new(if env_state { "ON" } else { "OFF" }).fg(if env_state {
            Color::Green
        } else {
            Color::DarkGrey
        }),
        Cell::new(env_addr)
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(env_note)
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);
    table.add_row(vec![
        Cell::new("Git"),
        Cell::new(if git_state { "ON" } else { "OFF" }).fg(if git_state {
            Color::Green
        } else {
            Color::DarkGrey
        }),
        Cell::new(git_addr)
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(git_note)
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);
    table.add_row(vec![
        Cell::new("npm"),
        Cell::new(if npm_state { "ON" } else { "OFF" }).fg(if npm_state {
            Color::Green
        } else {
            Color::DarkGrey
        }),
        Cell::new(npm_addr)
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(npm_note)
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);
    table.add_row(vec![
        Cell::new("Cargo"),
        Cell::new(if cargo_state { "ON" } else { "OFF" }).fg(if cargo_state {
            Color::Green
        } else {
            Color::DarkGrey
        }),
        Cell::new(cargo_addr)
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(cargo_note)
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);

    print_table(&table);

    if let Some(proxy_url) = env_proxy {
        let results = run_proxy_tests(&proxy_url);
        let mut t = Table::new();
        apply_pretty_table_style(&mut t);
        t.set_header(vec![
            Cell::new("Target")
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new("Latency")
                .add_attribute(Attribute::Bold)
                .fg(Color::Green),
            Cell::new("Detail")
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
        ]);
        for (label, result) in results {
            match result {
                Ok(ms) => {
                    t.add_row(vec![
                        Cell::new(label),
                        Cell::new(format!("{}ms", ms)).fg(Color::Green),
                        Cell::new("ok")
                            .fg(Color::DarkGrey)
                            .add_attribute(Attribute::Dim),
                    ]);
                }
                Err(e) => {
                    t.add_row(vec![
                        Cell::new(label).fg(Color::Red),
                        Cell::new("-").fg(Color::Red),
                        Cell::new(e).fg(Color::Red),
                    ]);
                }
            }
        }
        print_table(&t);
    }

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
