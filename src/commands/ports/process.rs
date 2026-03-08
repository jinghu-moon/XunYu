use super::common::trunc;
use super::render::render_processes_table;
use super::*;

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
