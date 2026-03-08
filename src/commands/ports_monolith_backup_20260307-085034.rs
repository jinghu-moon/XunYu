use std::collections::{HashMap, HashSet};

use comfy_table::{Attribute, Cell, Color, Table};
use console::Term;
use dialoguer::{Confirm, MultiSelect, theme::ColorfulTheme};

use crate::cli::{KillCmd, PkillCmd, PortsCmd, PsCmd};
use crate::model::{ListFormat, parse_list_format};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, can_interact, prefer_table_output, print_table};
use crate::ports::{PortInfo, Protocol, list_tcp_listeners, list_udp_endpoints, terminate_pid};
use crate::proc::{self, KillResult, ProcInfo};

fn is_dev_port(port: u16) -> bool {
    (3000..=3999).contains(&port)
        || (5000..=5999).contains(&port)
        || (8000..=8999).contains(&port)
        || port == 4173
        || port == 5173
}

fn trunc(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    let start = chars.len().saturating_sub(max - 3);
    format!("...{}", chars[start..].iter().collect::<String>())
}

fn proto_rank(p: Protocol) -> u8 {
    match p {
        Protocol::Tcp => 0,
        Protocol::Udp => 1,
    }
}

fn parse_range(raw: &str) -> Option<(u16, u16)> {
    let mut parts = raw.split('-').map(str::trim).filter(|s| !s.is_empty());
    let start = parts.next()?.parse::<u16>().ok()?;
    let end = parts.next()?.parse::<u16>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if start <= end {
        Some((start, end))
    } else {
        Some((end, start))
    }
}

pub(crate) fn cmd_ports(args: PortsCmd) -> CliResult {
    let mut items = if args.udp {
        list_udp_endpoints()
    } else {
        list_tcp_listeners()
    };

    if !args.udp && !args.all {
        items.retain(|p| is_dev_port(p.port));
    }

    if let Some(ref raw) = args.range {
        let Some((start, end)) = parse_range(raw) else {
            return Err(CliError::with_details(
                2,
                format!("Invalid range: {}.", raw),
                &["Fix: Use START-END (e.g. 3000-4000)."],
            ));
        };
        items.retain(|p| p.port >= start && p.port <= end);
    }
    if let Some(pid) = args.pid {
        items.retain(|p| p.pid == pid);
    }
    if let Some(ref name) = args.name {
        let needle = name.to_lowercase();
        items.retain(|p| p.name.to_lowercase().contains(&needle));
    }

    if items.is_empty() {
        ui_println!("No ports found.");
        return Ok(());
    }

    items.sort_by(|a, b| a.port.cmp(&b.port).then(a.pid.cmp(&b.pid)));

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

    if format == ListFormat::Tsv {
        for p in items {
            out_println!("{}\t{}\t{}\t{}", p.port, p.pid, p.name, p.exe_path);
        }
        return Ok(());
    }
    if format == ListFormat::Json {
        let list: Vec<serde_json::Value> = items
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "port": p.port,
                    "pid": p.pid,
                    "name": p.name,
                    "path": p.exe_path,
                    "protocol": match p.protocol { Protocol::Tcp => "tcp", Protocol::Udp => "udp" },
                })
            })
            .collect();
        out_println!("{}", serde_json::Value::Array(list));
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Port")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("PID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Process")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Path")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
    ]);

    for p in items {
        let path_cell = if p.exe_path.is_empty() {
            Cell::new("<denied>")
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        } else {
            Cell::new(trunc(&p.exe_path, 55))
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        };
        table.add_row(vec![
            Cell::new(p.port).fg(Color::Green),
            Cell::new(p.pid).fg(Color::Cyan),
            Cell::new(p.name)
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
            path_cell,
        ]);
    }

    print_table(&table);
    Ok(())
}

fn render_targets_table(targets: &[PortInfo]) {
    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Proto")
            .add_attribute(Attribute::Bold)
            .fg(Color::DarkGrey),
        Cell::new("Port")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("PID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Process")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Path")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
    ]);

    for p in targets {
        let proto = match p.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        };
        let path_cell = if p.exe_path.is_empty() {
            Cell::new("<denied>")
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        } else {
            Cell::new(trunc(&p.exe_path, 55))
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        };
        table.add_row(vec![
            Cell::new(proto).fg(Color::DarkGrey),
            Cell::new(p.port).fg(Color::Green),
            Cell::new(p.pid).fg(Color::Cyan),
            Cell::new(&p.name)
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
            path_cell,
        ]);
    }
    print_table(&table);
}

pub(crate) fn cmd_kill(args: KillCmd) -> CliResult {
    let mut ports: Vec<u16> = Vec::new();
    for part in args.ports.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        match trimmed.parse::<u16>() {
            Ok(p) => ports.push(p),
            Err(_) => {
                return Err(CliError::with_details(
                    2,
                    format!("Invalid port: {}", trimmed),
                    &["Fix: Provide a comma-separated list of ports (e.g. 3000,5173)."],
                ));
            }
        }
    }

    if ports.is_empty() {
        return Err(CliError::with_details(
            2,
            "No ports provided.".to_string(),
            &["Fix: Pass ports like `xun kill 3000,5173`."],
        ));
    }

    let want_tcp = args.tcp || (!args.tcp && !args.udp);
    let want_udp = args.udp || (!args.tcp && !args.udp);
    let mut items = Vec::new();
    if want_tcp {
        items.extend(list_tcp_listeners());
    }
    if want_udp {
        items.extend(list_udp_endpoints());
    }

    let port_set: HashSet<u16> = ports.into_iter().collect();
    let mut targets: Vec<PortInfo> = items
        .into_iter()
        .filter(|p| port_set.contains(&p.port))
        .collect();

    if targets.is_empty() {
        ui_println!("No matching ports found.");
        return Ok(());
    }

    targets.sort_by(|a, b| {
        a.port
            .cmp(&b.port)
            .then(proto_rank(a.protocol).cmp(&proto_rank(b.protocol)))
            .then(a.pid.cmp(&b.pid))
    });

    render_targets_table(&targets);

    let mut pid_map: HashMap<u32, Vec<usize>> = HashMap::new();
    for (idx, t) in targets.iter().enumerate() {
        pid_map.entry(t.pid).or_default().push(idx);
    }

    let unique: Vec<u32> = pid_map.keys().cloned().collect();
    let mut kill_pids: Vec<u32> = Vec::new();

    if args.force {
        kill_pids = unique;
    } else if !can_interact() {
        return Err(CliError::with_details(
            2,
            "Non-interactive shell.".to_string(),
            &["Fix: Re-run with --force to kill without prompts."],
        ));
    } else if unique.len() == 1 {
        let pid = unique[0];
        let name = targets[pid_map[&pid][0]].name.clone();
        let ans = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Kill {} (PID {})?", name, pid))
            .default(false)
            .interact_on(&Term::stderr());
        if matches!(ans, Ok(true)) {
            kill_pids.push(pid);
        } else {
            return Err(CliError::new(3, "Cancelled."));
        }
    } else {
        let mut display: Vec<String> = Vec::new();
        let mut pid_order: Vec<u32> = Vec::new();
        for pid in unique {
            let idx = pid_map[&pid][0];
            let name = &targets[idx].name;
            display.push(format!("{:<20} PID {}", name, pid));
            pid_order.push(pid);
        }

        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select processes to kill  (Space=toggle, Enter=confirm)")
            .items(&display)
            .interact_on(&Term::stderr());

        match selections {
            Ok(sel) if !sel.is_empty() => {
                for i in sel {
                    kill_pids.push(pid_order[i]);
                }
            }
            Ok(_) => {
                return Err(CliError::new(3, "No selection. Cancelled."));
            }
            Err(_) => {
                return Err(CliError::new(3, "Cancelled."));
            }
        }
    }

    for pid in kill_pids {
        match terminate_pid(pid) {
            Ok(()) => ui_println!("ok: pid {}", pid),
            Err(e) => ui_println!("fail: pid {} ({})", pid, e),
        }
    }

    Ok(())
}

pub(crate) fn cmd_ps(args: PsCmd) -> CliResult {
    if let Some(pid) = args.pid {
        if let Some(proc_info) = proc::find_by_pid(pid) {
            render_processes_table(&[proc_info], true);
        } else {
            ui_println!("No process found for PID {}.", pid);
        }
        return Ok(());
    }

    if let Some(ref window) = args.win {
        let procs = proc::find_by_window_title(window);
        if procs.is_empty() {
            ui_println!("No windows matching '{}'.", window);
        } else {
            render_processes_table(&procs, true);
            ui_println!("Matched processes: {}", procs.len());
        }
        return Ok(());
    }

    if let Some(ref pattern) = args.pattern {
        let procs = proc::find_by_name(pattern);
        if procs.is_empty() {
            ui_println!("No processes matching '{}'.", pattern);
        } else {
            render_processes_table(&procs, true);
            ui_println!("Matched processes: {}", procs.len());
        }
        return Ok(());
    }

    let mut procs = proc::list_all(false);
    procs.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.pid.cmp(&b.pid))
    });
    if procs.is_empty() {
        ui_println!("No processes found.");
        return Ok(());
    }
    render_processes_table(&procs, false);
    ui_println!("Total processes: {}", procs.len());
    Ok(())
}

pub(crate) fn cmd_pkill(args: PkillCmd) -> CliResult {
    let targets = if args.window {
        let procs = proc::find_by_window_title(&args.target);
        if procs.is_empty() {
            ui_println!("No windows matching '{}'.", args.target);
            return Ok(());
        }
        procs
    } else if let Ok(pid) = args.target.trim().parse::<u32>() {
        match proc::find_by_pid(pid) {
            Some(p) => vec![p],
            None => {
                ui_println!("No process found for PID {}.", pid);
                return Ok(());
            }
        }
    } else {
        let procs = proc::find_by_name(&args.target);
        if procs.is_empty() {
            ui_println!("No processes matching '{}'.", args.target);
            return Ok(());
        }
        procs
    };

    render_processes_table(&targets, true);

    let selected = if args.force {
        targets
    } else if !can_interact() {
        return Err(CliError::with_details(
            2,
            "Non-interactive shell.".to_string(),
            &["Fix: Re-run with --force to kill without prompts."],
        ));
    } else if targets.len() == 1 {
        let target = &targets[0];
        let ans = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Kill {} (PID {})?", target.name, target.pid))
            .default(false)
            .interact_on(&Term::stderr());
        if matches!(ans, Ok(true)) {
            targets
        } else {
            return Err(CliError::new(3, "Cancelled."));
        }
    } else {
        let items: Vec<String> = targets
            .iter()
            .map(|p| {
                let extra = if !p.window_title.is_empty() {
                    format!(" [{}]", trunc(&p.window_title, 40))
                } else if !p.exe_path.is_empty() {
                    format!(" {}", trunc(&p.exe_path, 45))
                } else {
                    String::new()
                };
                format!("{:<20} PID {:>6}{}", p.name, p.pid, extra)
            })
            .collect();

        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select processes to kill (Space=toggle, Enter=confirm)")
            .items(&items)
            .interact_on(&Term::stderr());

        match selections {
            Ok(sel) if !sel.is_empty() => sel.into_iter().map(|idx| targets[idx].clone()).collect(),
            Ok(_) => return Err(CliError::new(3, "No selection. Cancelled.")),
            Err(_) => return Err(CliError::new(3, "Cancelled.")),
        }
    };

    kill_processes(&selected);
    Ok(())
}

fn render_processes_table(procs: &[ProcInfo], show_path: bool) {
    let has_titles = procs.iter().any(|p| !p.window_title.is_empty());
    let has_paths = show_path && procs.iter().any(|p| !p.exe_path.is_empty());

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    let mut headers = vec![
        Cell::new("PID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("PPID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Name")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Threads")
            .add_attribute(Attribute::Bold)
            .fg(Color::DarkGrey),
    ];
    if has_titles {
        headers.push(
            Cell::new("Window")
                .add_attribute(Attribute::Bold)
                .fg(Color::Magenta),
        );
    }
    if has_paths {
        headers.push(
            Cell::new("Path")
                .add_attribute(Attribute::Bold)
                .fg(Color::DarkGrey),
        );
    }
    table.set_header(headers);

    for p in procs {
        let mut row = vec![
            Cell::new(p.pid).fg(Color::Green),
            Cell::new(p.ppid).fg(Color::Cyan),
            Cell::new(&p.name).fg(Color::Yellow),
            Cell::new(p.thread_cnt).fg(Color::DarkGrey),
        ];
        if has_titles {
            if p.window_title.is_empty() {
                row.push(
                    Cell::new("-")
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                );
            } else {
                row.push(Cell::new(trunc(&p.window_title, 50)).fg(Color::Magenta));
            }
        }
        if has_paths {
            if p.exe_path.is_empty() {
                row.push(
                    Cell::new("<denied>")
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                );
            } else {
                row.push(
                    Cell::new(trunc(&p.exe_path, 55))
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                );
            }
        }
        table.add_row(row);
    }
    print_table(&table);
}

fn kill_processes(processes: &[ProcInfo]) {
    for p in processes {
        match proc::kill_pid(p.pid) {
            KillResult::Ok => ui_println!("ok: {} (pid {})", p.name, p.pid),
            KillResult::AccessDenied => {
                ui_println!("fail: {} (pid {}) access denied", p.name, p.pid)
            }
            KillResult::NotFound => ui_println!("skip: {} (pid {}) already gone", p.name, p.pid),
            KillResult::Error(code) => {
                ui_println!("fail: {} (pid {}) error {}", p.name, p.pid, code)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_dev_port_matches_expected_ranges() {
        assert!(is_dev_port(3000));
        assert!(is_dev_port(3999));
        assert!(is_dev_port(5000));
        assert!(is_dev_port(5999));
        assert!(is_dev_port(8000));
        assert!(is_dev_port(8999));
        assert!(is_dev_port(4173));
        assert!(is_dev_port(5173));

        assert!(!is_dev_port(2999));
        assert!(!is_dev_port(4000));
        assert!(!is_dev_port(6000));
        assert!(!is_dev_port(9000));
    }

    #[test]
    fn parse_range_parses_and_normalizes() {
        assert_eq!(parse_range("3000-4000"), Some((3000, 4000)));
        assert_eq!(parse_range("4000-3000"), Some((3000, 4000)));
        assert_eq!(parse_range(" 3000 - 4000 "), Some((3000, 4000)));
        assert_eq!(parse_range("3000"), None);
        assert_eq!(parse_range("a-b"), None);
        assert_eq!(parse_range("1-2-3"), None);
    }

    #[test]
    fn trunc_short_strings_are_unchanged_and_long_strings_keep_suffix() {
        assert_eq!(trunc("abc", 10), "abc");
        assert_eq!(trunc("0123456789", 6), "...789");
    }
}
