use super::*;
use super::probe::run_proxy_tests;
use super::set_del::read_cargo_proxy;

pub(crate) fn cmd_proxy_status(_args: ProxyStatusCmd) {
    let env_proxy = env::var("HTTP_PROXY")
        .or_else(|_| env::var("http_proxy"))
        .ok();
    let env_noproxy = env::var("NO_PROXY")
        .or_else(|_| env::var("no_proxy"))
        .ok();

    let git_proxy = if has_cmd("git") {
        Command::new("git")
            .args(["config", "--global", "--get", "http.proxy"])
            .output()
            .ok()
            .and_then(|o| {
                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if v.is_empty() || v == "null" { None } else { Some(v) }
            })
    } else {
        None
    };

    let npm_proxy = if has_cmd("npm") {
        Command::new("npm")
            .args(["config", "get", "proxy"])
            .output()
            .ok()
            .and_then(|o| {
                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if v.is_empty() || v == "null" { None } else { Some(v) }
            })
    } else {
        None
    };

    let cargo_proxy = read_cargo_proxy();

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Tool").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("Status").add_attribute(Attribute::Bold).fg(Color::Green),
        Cell::new("Address").add_attribute(Attribute::Bold).fg(Color::Magenta),
        Cell::new("Note").add_attribute(Attribute::Bold).fg(Color::Yellow),
    ]);

    let env_state = env_proxy.is_some();
    table.add_row(vec![
        Cell::new("Env"),
        Cell::new(if env_state { "ON" } else { "OFF" })
            .fg(if env_state { Color::Green } else { Color::DarkGrey }),
        Cell::new(env_proxy.clone().unwrap_or_else(|| "-".into()))
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(env_noproxy.unwrap_or_else(|| "-".into()))
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);

    let git_state = git_proxy.is_some();
    table.add_row(vec![
        Cell::new("Git"),
        Cell::new(if git_state { "ON" } else { "OFF" })
            .fg(if git_state { Color::Green } else { Color::DarkGrey }),
        Cell::new(git_proxy.clone().unwrap_or_else(|| "-".into()))
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(if has_cmd("git") { "" } else { "not found" })
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);

    let npm_state = npm_proxy.is_some();
    table.add_row(vec![
        Cell::new("npm"),
        Cell::new(if npm_state { "ON" } else { "OFF" })
            .fg(if npm_state { Color::Green } else { Color::DarkGrey }),
        Cell::new(npm_proxy.clone().unwrap_or_else(|| "-".into()))
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(if has_cmd("npm") { "" } else { "not found" })
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);

    let cargo_state = cargo_proxy.is_some();
    table.add_row(vec![
        Cell::new("Cargo"),
        Cell::new(if cargo_state { "ON" } else { "OFF" })
            .fg(if cargo_state { Color::Green } else { Color::DarkGrey }),
        Cell::new(cargo_proxy.clone().unwrap_or_else(|| "-".into()))
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        Cell::new(if cargo_state { "config.toml" } else { "" })
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    ]);

    ui_println!("{}", table);

    if let Some(proxy_url) = env_proxy {
        let results = run_proxy_tests(&proxy_url);
        let mut t = Table::new();
        apply_pretty_table_style(&mut t);
        t.set_header(vec![
            Cell::new("Target").add_attribute(Attribute::Bold).fg(Color::Cyan),
            Cell::new("Latency").add_attribute(Attribute::Bold).fg(Color::Green),
            Cell::new("Detail").add_attribute(Attribute::Bold).fg(Color::Yellow),
        ]);
        for (label, result) in results {
            match result {
                Ok(ms) => {
                    t.add_row(vec![
                        Cell::new(label),
                        Cell::new(format!("{}ms", ms)).fg(Color::Green),
                        Cell::new("ok").fg(Color::DarkGrey).add_attribute(Attribute::Dim),
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
        ui_println!("{}", t);
    }
}

