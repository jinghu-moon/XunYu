use super::common::proto_rank;
use super::render::render_targets_table;
use super::*;

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
