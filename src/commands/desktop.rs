use crate::cli::{
    DesktopAppCmd, DesktopAppListCmd, DesktopAppSubCommand, DesktopAwakeCmd, DesktopAwakeOffCmd,
    DesktopAwakeOnCmd, DesktopAwakeStatusCmd, DesktopAwakeSubCommand, DesktopCmd, DesktopColorCmd,
    DesktopDaemonCmd, DesktopDaemonReloadCmd, DesktopDaemonStartCmd, DesktopDaemonStatusCmd,
    DesktopDaemonStopCmd, DesktopDaemonSubCommand, DesktopHostsAddCmd, DesktopHostsCmd,
    DesktopHostsListCmd, DesktopHostsRemoveCmd, DesktopHostsSubCommand, DesktopHotkeyBindCmd,
    DesktopHotkeyCmd, DesktopHotkeyListCmd, DesktopHotkeySubCommand, DesktopHotkeyUnbindCmd,
    DesktopLayoutApplyCmd, DesktopLayoutCmd, DesktopLayoutListCmd, DesktopLayoutNewCmd,
    DesktopLayoutPreviewCmd, DesktopLayoutRemoveCmd, DesktopLayoutSubCommand, DesktopRemapAddCmd,
    DesktopRemapClearCmd, DesktopRemapCmd, DesktopRemapListCmd, DesktopRemapRemoveCmd,
    DesktopRemapSubCommand, DesktopRunCmd, DesktopSnippetAddCmd, DesktopSnippetClearCmd,
    DesktopSnippetCmd, DesktopSnippetListCmd, DesktopSnippetRemoveCmd, DesktopSnippetSubCommand,
    DesktopSubCommand, DesktopThemeCmd, DesktopThemeScheduleCmd, DesktopThemeSetCmd,
    DesktopThemeStatusCmd, DesktopThemeSubCommand, DesktopThemeToggleCmd, DesktopWindowCmd,
    DesktopWindowFocusCmd, DesktopWindowMoveCmd, DesktopWindowResizeCmd, DesktopWindowSubCommand,
    DesktopWindowTopCmd, DesktopWindowTransparentCmd, DesktopWorkspaceCmd, DesktopWorkspaceLaunchCmd,
    DesktopWorkspaceListCmd, DesktopWorkspaceRemoveCmd, DesktopWorkspaceSaveCmd,
    DesktopWorkspaceSubCommand,
};
use crate::desktop;
use crate::output::{CliError, CliResult};
use crate::windows::window_api;

pub(crate) fn cmd_desktop(args: DesktopCmd) -> CliResult {
    match args.cmd {
        DesktopSubCommand::Daemon(cmd) => cmd_daemon(cmd),
        DesktopSubCommand::Hotkey(cmd) => cmd_hotkey(cmd),
        DesktopSubCommand::Remap(cmd) => cmd_remap(cmd),
        DesktopSubCommand::Snippet(cmd) => cmd_snippet(cmd),
        DesktopSubCommand::Layout(cmd) => cmd_layout(cmd),
        DesktopSubCommand::Workspace(cmd) => cmd_workspace(cmd),
        DesktopSubCommand::Window(cmd) => cmd_window(cmd),
        DesktopSubCommand::Theme(cmd) => cmd_theme(cmd),
        DesktopSubCommand::Awake(cmd) => cmd_awake(cmd),
        DesktopSubCommand::Color(cmd) => cmd_color(cmd),
        DesktopSubCommand::Hosts(cmd) => cmd_hosts(cmd),
        DesktopSubCommand::App(cmd) => cmd_app(cmd),
        DesktopSubCommand::Tui(_) => cmd_tui(),
        DesktopSubCommand::Run(cmd) => cmd_run(cmd),
    }
}

fn cmd_daemon(args: DesktopDaemonCmd) -> CliResult {
    match args.cmd {
        DesktopDaemonSubCommand::Start(a) => cmd_daemon_start(a),
        DesktopDaemonSubCommand::Stop(a) => cmd_daemon_stop(a),
        DesktopDaemonSubCommand::Status(a) => cmd_daemon_status(a),
        DesktopDaemonSubCommand::Reload(a) => cmd_daemon_reload(a),
    }
}

fn cmd_daemon_start(args: DesktopDaemonStartCmd) -> CliResult {
    if args.elevated && !crate::env_core::uac::is_elevated() {
        let exe = std::env::current_exe()
            .map_err(|e| CliError::new(1, format!("Failed to get executable path: {e}")))?;
        let raw_args = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
        crate::env_core::uac::relaunch_elevated(exe.to_string_lossy().as_ref(), &raw_args)
            .map_err(|e| CliError::new(1, format!("Failed to relaunch elevated: {e}")))?;
        ui_println!("Relaunching elevated...");
        return Ok(());
    }

    let opts = desktop::daemon::DaemonOptions {
        quiet: args.quiet,
        no_tray: args.no_tray,
    };
    desktop::daemon::run_daemon(opts)
}

fn cmd_daemon_stop(_args: DesktopDaemonStopCmd) -> CliResult {
    Err(CliError::with_details(
        2,
        "daemon stop is not supported.".to_string(),
        &["Fix: stop the foreground daemon with Ctrl+C."],
    ))
}

fn cmd_daemon_status(_args: DesktopDaemonStatusCmd) -> CliResult {
    ui_println!("Daemon runs in foreground; status query not available.");
    Ok(())
}

fn cmd_daemon_reload(_args: DesktopDaemonReloadCmd) -> CliResult {
    Err(CliError::with_details(
        2,
        "daemon reload is not supported.".to_string(),
        &["Fix: restart the daemon to reload config."],
    ))
}

fn cmd_hotkey(args: DesktopHotkeyCmd) -> CliResult {
    match args.cmd {
        DesktopHotkeySubCommand::Bind(a) => cmd_hotkey_bind(a),
        DesktopHotkeySubCommand::Unbind(a) => cmd_hotkey_unbind(a),
        DesktopHotkeySubCommand::List(a) => cmd_hotkey_list(a),
    }
}

fn cmd_hotkey_bind(args: DesktopHotkeyBindCmd) -> CliResult {
    let hotkey = args.hotkey.trim();
    if hotkey.is_empty() {
        return Err(CliError::new(2, "Hotkey is required."));
    }
    if let Some(reason) = unremappable_reason(hotkey) {
        return Err(CliError::with_details(
            2,
            format!("Hotkey not allowed: {hotkey}"),
            &[reason],
        ));
    }
    if desktop::hotkey::parse_hotkey(hotkey).is_none() {
        return Err(CliError::with_details(
            2,
            format!("Invalid hotkey: {hotkey}"),
            &["Fix: use format like ctrl+alt+t."],
        ));
    }

    let action = args.action.trim();
    if action.is_empty() {
        return Err(CliError::new(2, "Action is required."));
    }

    let mut cfg = crate::config::load_config();
    let app = normalize_option(args.app);
    let mut updated = false;
    for binding in cfg.desktop.bindings.iter_mut() {
        if hotkey_eq(&binding.hotkey, hotkey) && option_eq(&binding.app, &app) {
            binding.action = action.to_string();
            binding.app = app.clone();
            updated = true;
            break;
        }
    }
    if !updated {
        cfg.desktop.bindings.push(crate::config::DesktopBinding {
            hotkey: hotkey.to_string(),
            action: action.to_string(),
            app: app.clone(),
        });
    }

    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;

    let app_label = app.as_deref().unwrap_or("any");
    if updated {
        ui_println!("Hotkey updated: {} -> {} [{}]", hotkey, action, app_label);
    } else {
        ui_println!("Hotkey added: {} -> {} [{}]", hotkey, action, app_label);
    }
    Ok(())
}

fn cmd_hotkey_unbind(args: DesktopHotkeyUnbindCmd) -> CliResult {
    let hotkey = args.hotkey.trim();
    if hotkey.is_empty() {
        return Err(CliError::new(2, "Hotkey is required."));
    }

    let mut cfg = crate::config::load_config();
    let before = cfg.desktop.bindings.len();
    cfg.desktop
        .bindings
        .retain(|b| !hotkey_eq(&b.hotkey, hotkey));
    let removed = before - cfg.desktop.bindings.len();
    if removed == 0 {
        return Err(CliError::new(2, format!("Hotkey not found: {hotkey}")));
    }

    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;

    ui_println!("Hotkey removed: {} ({} binding(s))", hotkey, removed);
    Ok(())
}

fn cmd_hotkey_list(_args: DesktopHotkeyListCmd) -> CliResult {
    let cfg = crate::config::load_config();
    if cfg.desktop.bindings.is_empty() {
        ui_println!("No hotkeys configured.");
        return Ok(());
    }

    for binding in &cfg.desktop.bindings {
        let app = binding.app.as_deref().unwrap_or("any");
        ui_println!("{} -> {} [{}]", binding.hotkey, binding.action, app);
    }
    Ok(())
}

fn cmd_remap(args: DesktopRemapCmd) -> CliResult {
    match args.cmd {
        DesktopRemapSubCommand::Add(a) => cmd_remap_add(a),
        DesktopRemapSubCommand::Remove(a) => cmd_remap_remove(a),
        DesktopRemapSubCommand::List(a) => cmd_remap_list(a),
        DesktopRemapSubCommand::Clear(a) => cmd_remap_clear(a),
    }
}

fn cmd_remap_add(args: DesktopRemapAddCmd) -> CliResult {
    let from = args.from.trim();
    if from.is_empty() {
        return Err(CliError::new(2, "Remap source is required."));
    }
    if let Some(reason) = unremappable_reason(from) {
        return Err(CliError::with_details(
            2,
            format!("Remap source not allowed: {from}"),
            &[reason],
        ));
    }
    if desktop::hotkey::parse_hotkey(from).is_none() {
        return Err(CliError::with_details(
            2,
            format!("Invalid remap source: {from}"),
            &["Fix: use format like ctrl+alt+1."],
        ));
    }

    let to = args.to.trim();
    if to.is_empty() {
        return Err(CliError::new(2, "Remap target is required."));
    }
    if let Some(text) = to.strip_prefix("text:") {
        if text.is_empty() {
            return Err(CliError::new(2, "Remap text is empty."));
        }
    } else if !to.eq_ignore_ascii_case("disable")
        && desktop::hotkey::parse_hotkey(to).is_none()
    {
        return Err(CliError::with_details(
            2,
            format!("Invalid remap target: {to}"),
            &["Fix: use a hotkey, disable, or text:<value>."],
        ));
    }

    let mut cfg = crate::config::load_config();
    let app = normalize_option(args.app);
    let mut updated = false;
    for rule in cfg.desktop.remaps.iter_mut() {
        if hotkey_eq(&rule.from, from) && option_eq(&rule.app, &app) {
            if !args.dry_run {
                rule.to = to.to_string();
                rule.app = app.clone();
                rule.exact = args.exact;
            }
            updated = true;
            break;
        }
    }
    let app_label = app.as_deref().unwrap_or("any");
    let suffix = if args.exact { "exact" } else { "partial" };
    if args.dry_run {
        if updated {
            ui_println!(
                "Dry-run: Remap would be updated: {} -> {} [{} | {}]",
                from,
                to,
                app_label,
                suffix
            );
        } else {
            ui_println!(
                "Dry-run: Remap would be added: {} -> {} [{} | {}]",
                from,
                to,
                app_label,
                suffix
            );
        }
        return Ok(());
    }

    if !updated {
        cfg.desktop.remaps.push(crate::config::DesktopRemap {
            from: from.to_string(),
            to: to.to_string(),
            app: app.clone(),
            exact: args.exact,
        });
    }

    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;

    if updated {
        ui_println!(
            "Remap updated: {} -> {} [{} | {}]",
            from,
            to,
            app_label,
            suffix
        );
    } else {
        ui_println!(
            "Remap added: {} -> {} [{} | {}]",
            from,
            to,
            app_label,
            suffix
        );
    }
    Ok(())
}

fn cmd_remap_remove(args: DesktopRemapRemoveCmd) -> CliResult {
    let from = args.from.trim();
    if from.is_empty() {
        return Err(CliError::new(2, "Remap source is required."));
    }
    let to = args.to.as_ref().map(|s| s.trim().to_string());

    let mut cfg = crate::config::load_config();
    let removed = cfg
        .desktop
        .remaps
        .iter()
        .filter(|r| {
            if !hotkey_eq(&r.from, from) {
                return false;
            }
            if let Some(ref target) = to {
                return hotkey_eq(&r.to, target);
            }
            true
        })
        .count();
    if removed == 0 {
        return Err(CliError::new(2, format!("Remap not found: {from}")));
    }

    if args.dry_run {
        ui_println!("Dry-run: Remap would be removed: {} ({} rule(s))", from, removed);
        return Ok(());
    }

    cfg.desktop.remaps.retain(|r| {
        if !hotkey_eq(&r.from, from) {
            return true;
        }
        if let Some(ref target) = to {
            return !hotkey_eq(&r.to, target);
        }
        false
    });

    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;

    ui_println!("Remap removed: {} ({} rule(s))", from, removed);
    Ok(())
}

fn cmd_remap_list(_args: DesktopRemapListCmd) -> CliResult {
    let cfg = crate::config::load_config();
    if cfg.desktop.remaps.is_empty() {
        ui_println!("No remap rules configured.");
        return Ok(());
    }

    for rule in &cfg.desktop.remaps {
        let app = rule.app.as_deref().unwrap_or("any");
        let exact = if rule.exact { "exact" } else { "partial" };
        ui_println!("{} -> {} [{} | {}]", rule.from, rule.to, app, exact);
    }
    Ok(())
}

fn cmd_remap_clear(args: DesktopRemapClearCmd) -> CliResult {
    let mut cfg = crate::config::load_config();
    if cfg.desktop.remaps.is_empty() {
        ui_println!("No remap rules configured.");
        return Ok(());
    }

    let count = cfg.desktop.remaps.len();
    if args.dry_run {
        ui_println!("Dry-run: Remap rules would be cleared: {}", count);
        return Ok(());
    }
    cfg.desktop.remaps.clear();
    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
    ui_println!("Remap rules cleared: {}", count);
    Ok(())
}

fn cmd_snippet(args: DesktopSnippetCmd) -> CliResult {
    match args.cmd {
        DesktopSnippetSubCommand::Add(a) => cmd_snippet_add(a),
        DesktopSnippetSubCommand::Remove(a) => cmd_snippet_remove(a),
        DesktopSnippetSubCommand::List(a) => cmd_snippet_list(a),
        DesktopSnippetSubCommand::Clear(a) => cmd_snippet_clear(a),
    }
}

fn cmd_snippet_add(args: DesktopSnippetAddCmd) -> CliResult {
    let trigger = args.trigger.trim();
    if trigger.is_empty() {
        return Err(CliError::new(2, "Snippet trigger is required."));
    }
    let expand = args.expand;
    let app = normalize_option(args.app);
    let paste = if args.clipboard {
        Some("clipboard".to_string())
    } else {
        None
    };

    let mut cfg = crate::config::load_config();
    let mut updated = false;
    for snippet in cfg.desktop.snippets.iter_mut() {
        if trigger_eq(&snippet.trigger, trigger) && option_eq(&snippet.app, &app) {
            snippet.expand = expand.clone();
            snippet.immediate = args.immediate;
            snippet.app = app.clone();
            snippet.paste = paste.clone();
            updated = true;
            break;
        }
    }
    if !updated {
        cfg.desktop.snippets.push(crate::config::DesktopSnippet {
            trigger: trigger.to_string(),
            expand,
            app: app.clone(),
            immediate: args.immediate,
            paste: paste.clone(),
        });
    }

    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;

    let app_label = app.as_deref().unwrap_or("any");
    let mode = if args.immediate { "immediate" } else { "terminator" };
    let paste_label = paste.as_deref().unwrap_or("sendinput");
    if updated {
        ui_println!(
            "Snippet updated: {} [{} | {} | {}]",
            trigger,
            app_label,
            mode,
            paste_label
        );
    } else {
        ui_println!(
            "Snippet added: {} [{} | {} | {}]",
            trigger,
            app_label,
            mode,
            paste_label
        );
    }
    Ok(())
}

fn cmd_snippet_remove(args: DesktopSnippetRemoveCmd) -> CliResult {
    let trigger = args.trigger.trim();
    if trigger.is_empty() {
        return Err(CliError::new(2, "Snippet trigger is required."));
    }

    let mut cfg = crate::config::load_config();
    let before = cfg.desktop.snippets.len();
    cfg.desktop
        .snippets
        .retain(|s| !trigger_eq(&s.trigger, trigger));
    let removed = before - cfg.desktop.snippets.len();
    if removed == 0 {
        return Err(CliError::new(2, format!("Snippet not found: {trigger}")));
    }

    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;

    ui_println!("Snippet removed: {} ({} item(s))", trigger, removed);
    Ok(())
}

fn cmd_snippet_list(_args: DesktopSnippetListCmd) -> CliResult {
    let cfg = crate::config::load_config();
    if cfg.desktop.snippets.is_empty() {
        ui_println!("No snippets configured.");
        return Ok(());
    }

    for snippet in &cfg.desktop.snippets {
        let app = snippet.app.as_deref().unwrap_or("any");
        let mode = if snippet.immediate { "immediate" } else { "terminator" };
        let paste = snippet.paste.as_deref().unwrap_or("sendinput");
        ui_println!(
            "{} => {} [{} | {} | {}]",
            snippet.trigger,
            snippet.expand,
            app,
            mode,
            paste
        );
    }
    Ok(())
}

fn cmd_snippet_clear(_args: DesktopSnippetClearCmd) -> CliResult {
    let mut cfg = crate::config::load_config();
    if cfg.desktop.snippets.is_empty() {
        ui_println!("No snippets configured.");
        return Ok(());
    }

    let count = cfg.desktop.snippets.len();
    cfg.desktop.snippets.clear();
    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
    ui_println!("Snippets cleared: {}", count);
    Ok(())
}

fn unremappable_reason(hotkey: &str) -> Option<&'static str> {
    let normalized = normalize_hotkey(hotkey);
    for (key, reason) in desktop::hotkey::UNREMAPPABLE_KEYS {
        if normalized == *key {
            return Some(*reason);
        }
    }
    None
}

fn normalize_hotkey(raw: &str) -> String {
    raw.to_lowercase().replace(' ', "")
}

fn hotkey_eq(a: &str, b: &str) -> bool {
    normalize_hotkey(a) == normalize_hotkey(b)
}

fn trigger_eq(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn normalize_option(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn option_eq(left: &Option<String>, right: &Option<String>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => false,
    }
}

fn cmd_layout(args: DesktopLayoutCmd) -> CliResult {
    match args.cmd {
        DesktopLayoutSubCommand::New(a) => cmd_layout_new(a),
        DesktopLayoutSubCommand::Apply(a) => cmd_layout_apply(a),
        DesktopLayoutSubCommand::Preview(a) => cmd_layout_preview(a),
        DesktopLayoutSubCommand::List(a) => cmd_layout_list(a),
        DesktopLayoutSubCommand::Remove(a) => cmd_layout_remove(a),
    }
}

fn build_grid_layout(
    template: &crate::config::DesktopLayoutTemplate,
) -> crate::desktop::layout::GridLayout {
    crate::desktop::layout::GridLayout {
        rows: template.rows.unwrap_or(2).max(1) as usize,
        cols: template.cols.unwrap_or(2).max(1) as usize,
        gap: template.gap.unwrap_or(8) as i32,
        padding: 0,
        monitor: 0,
    }
}

fn map_window_api_error(
    action: &str,
    err: window_api::WindowApiError,
) -> CliError {
    match err {
        window_api::WindowApiError::NotFound => {
            CliError::new(2, "No matching window found.")
        }
        window_api::WindowApiError::OsError { action: op, code } => {
            CliError::with_details(
                2,
                format!("Failed to {action} window."),
                &[
                    format!("Win32: {op} (code={code})"),
                    "Fix: retry with admin rights.".to_string(),
                ],
            )
        }
    }
}

fn cmd_layout_new(args: DesktopLayoutNewCmd) -> CliResult {
    let layout_type = args.layout_type.to_lowercase();
    if layout_type != "grid" {
        return Err(CliError::with_details(
            2,
            format!("Unsupported layout type: {layout_type}"),
            &["Fix: use grid in Phase 1."],
        ));
    }

    let mut cfg = crate::config::load_config();
    let name = args.name.trim();
    if name.is_empty() {
        return Err(CliError::new(2, "Layout name is required."));
    }

    let rows = args.rows.unwrap_or(2).max(1) as u32;
    let cols = args.cols.unwrap_or(2).max(1) as u32;
    let gap = args.gap.unwrap_or(8) as u32;

    if let Some(existing) = cfg.desktop.layouts.iter_mut().find(|l| l.name == name) {
        existing.template.layout_type = "grid".to_string();
        existing.template.rows = Some(rows);
        existing.template.cols = Some(cols);
        existing.template.gap = Some(gap);
    } else {
        cfg.desktop.layouts.push(crate::config::DesktopLayout {
            name: name.to_string(),
            template: crate::config::DesktopLayoutTemplate {
                layout_type: "grid".to_string(),
                rows: Some(rows),
                cols: Some(cols),
                gap: Some(gap),
            },
            bindings: std::collections::BTreeMap::new(),
        });
    }

    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;

    ui_println!(
        "Layout {} created (type=grid rows={} cols={} gap={}).",
        name,
        rows,
        cols,
        gap
    );
    Ok(())
}

fn cmd_layout_apply(args: DesktopLayoutApplyCmd) -> CliResult {
    use std::collections::HashSet;

    let cfg = crate::config::load_config();
    let name = args.name.trim();
    let layout = cfg
        .desktop
        .layouts
        .iter()
        .find(|l| l.name == name)
        .ok_or_else(|| CliError::new(2, format!("Layout not found: {name}")))?;

    if layout.template.layout_type.to_lowercase() != "grid" {
        return Err(CliError::with_details(
            2,
            format!("Unsupported layout type: {}", layout.template.layout_type),
            &["Fix: use grid in Phase 1."],
        ));
    }

    let grid = build_grid_layout(&layout.template);
    let template = crate::desktop::layout::LayoutTemplate::Grid(grid);

    let monitor = crate::desktop::layout::get_monitor_area(0)
        .ok_or_else(|| CliError::new(2, "Monitor 0 not found."))?;
    let zones = crate::desktop::layout::compute_zones(&template, &monitor);

    if zones.is_empty() {
        return Err(CliError::new(2, "No zones computed."));
    }

    let mut assignments: Vec<(u32, crate::desktop::layout::ZoneRect)> = Vec::new();
    let mut used_pids: HashSet<u32> = HashSet::new();
    let mut used_zones: HashSet<usize> = HashSet::new();

    if !layout.bindings.is_empty() {
        for (app, zone_index) in &layout.bindings {
            if *zone_index >= zones.len() {
                return Err(CliError::new(
                    2,
                    format!("Zone index out of range: {zone_index}"),
                ));
            }
            if used_zones.contains(zone_index) {
                let detail = format!("Zone already assigned: {zone_index}");
                crate::output::emit_warning("Layout binding skipped.", &[detail.as_str()]);
                continue;
            }
            if let Some(proc) = crate::proc::find_by_name(app)
                .into_iter()
                .find(|p| !p.window_title.trim().is_empty())
            {
                if used_pids.insert(proc.pid) {
                    used_zones.insert(*zone_index);
                    assignments.push((proc.pid, zones[*zone_index].clone()));
                }
            } else {
                let detail = format!("App not found: {app}");
                crate::output::emit_warning("Layout binding skipped.", &[detail.as_str()]);
            }
        }

        if args.move_existing {
            let mut remaining: Vec<usize> = (0..zones.len())
                .filter(|i| !used_zones.contains(i))
                .collect();
            let windows = crate::proc::list_all(false);
            for proc in windows.into_iter().filter(|p| !p.window_title.trim().is_empty()) {
                if remaining.is_empty() {
                    break;
                }
                if used_pids.contains(&proc.pid) {
                    continue;
                }
                let zone_index = remaining.remove(0);
                used_pids.insert(proc.pid);
                assignments.push((proc.pid, zones[zone_index].clone()));
            }
        }
    } else {
        if !args.move_existing {
            return Err(CliError::with_details(
                2,
                "Layout has no bindings.".to_string(),
                &["Fix: add bindings or use --move-existing."],
            ));
        }
        let windows: Vec<_> = crate::proc::list_all(false)
            .into_iter()
            .filter(|p| !p.window_title.trim().is_empty())
            .collect();
        for (zone, proc) in zones.iter().zip(windows.iter()) {
            assignments.push((proc.pid, zone.clone()));
        }
    }

    if assignments.is_empty() {
        return Err(CliError::new(2, "No windows matched for layout."));
    }

    let mut moved = 0usize;
    for (pid, zone) in assignments {
        let hwnd = window_api::find_hwnd_by_pid(pid)
            .map_err(|e| map_window_api_error("apply", e))?;
        let rect = window_api::WindowRect {
            left: zone.x,
            top: zone.y,
            right: zone.x + zone.w,
            bottom: zone.y + zone.h,
        };
        window_api::apply_window_rect(hwnd, rect)
            .map_err(|e| map_window_api_error("apply", e))?;
        moved += 1;
    }

    ui_println!("Layout {} applied (windows moved: {}).", name, moved);
    if args.move_existing {
        ui_println!("Move existing windows enabled.");
    }
    Ok(())
}

fn cmd_layout_preview(args: DesktopLayoutPreviewCmd) -> CliResult {
    let cfg = crate::config::load_config();
    let name = args.name.trim();
    let layout = cfg
        .desktop
        .layouts
        .iter()
        .find(|l| l.name == name)
        .ok_or_else(|| CliError::new(2, format!("Layout not found: {name}")))?;

    if layout.template.layout_type.to_lowercase() != "grid" {
        return Err(CliError::with_details(
            2,
            format!("Unsupported layout type: {}", layout.template.layout_type),
            &["Fix: use grid in Phase 1."],
        ));
    }

    let grid = build_grid_layout(&layout.template);
    let template = crate::desktop::layout::LayoutTemplate::Grid(grid);
    let monitor = crate::desktop::layout::get_monitor_area(0)
        .ok_or_else(|| CliError::new(2, "Monitor 0 not found."))?;
    let zones = crate::desktop::layout::compute_zones(&template, &monitor);
    let preview = crate::desktop::layout::preview_ascii(&zones, &monitor);
    ui_println!("{preview}");
    Ok(())
}

fn cmd_layout_list(_args: DesktopLayoutListCmd) -> CliResult {
    let cfg = crate::config::load_config();
    if cfg.desktop.layouts.is_empty() {
        ui_println!("No layouts configured.");
        return Ok(());
    }

    for layout in &cfg.desktop.layouts {
        let tpl = layout.template.layout_type.clone();
        let rows = layout.template.rows.unwrap_or(0);
        let cols = layout.template.cols.unwrap_or(0);
        let gap = layout.template.gap.unwrap_or(0);
        ui_println!("{}    {}    {}x{}    gap={}", layout.name, tpl, rows, cols, gap);
    }
    Ok(())
}

fn cmd_layout_remove(args: DesktopLayoutRemoveCmd) -> CliResult {
    let mut cfg = crate::config::load_config();
    let name = args.name.trim();
    let before = cfg.desktop.layouts.len();
    cfg.desktop.layouts.retain(|l| l.name != name);
    if cfg.desktop.layouts.len() == before {
        return Err(CliError::new(2, format!("Layout not found: {name}")));
    }
    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
    ui_println!("Layout removed: {}", name);
    Ok(())
}

fn cmd_workspace(args: DesktopWorkspaceCmd) -> CliResult {
    match args.cmd {
        DesktopWorkspaceSubCommand::Save(a) => cmd_workspace_save(a),
        DesktopWorkspaceSubCommand::Launch(a) => cmd_workspace_launch(a),
        DesktopWorkspaceSubCommand::List(a) => cmd_workspace_list(a),
        DesktopWorkspaceSubCommand::Remove(a) => cmd_workspace_remove(a),
    }
}

fn cmd_workspace_save(args: DesktopWorkspaceSaveCmd) -> CliResult {
    let mut cfg = crate::config::load_config();
    let name = args.name.trim();
    if name.is_empty() {
        return Err(CliError::new(2, "Workspace name is required."));
    }

    let mut apps: Vec<crate::config::DesktopWorkspaceApp> = Vec::new();
    if !args.name_only {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for proc in crate::proc::list_all(true)
            .into_iter()
            .filter(|p| !p.window_title.trim().is_empty())
        {
            let exe = if proc.exe_path.is_empty() { proc.name } else { proc.exe_path };
            let key = exe.to_lowercase();
            if !seen.insert(key) {
                continue;
            }
            let rect = window_api::find_hwnd_by_pid(proc.pid)
                .ok()
                .and_then(|hwnd| window_api::get_window_rect(hwnd).ok())
                .map(|rect| [rect.left, rect.top, rect.right, rect.bottom]);
            apps.push(crate::config::DesktopWorkspaceApp {
                path: exe,
                args: None,
                rect,
            });
        }
    }

    if let Some(existing) = cfg.desktop.workspaces.iter_mut().find(|w| w.name == name) {
        existing.apps = apps;
    } else {
        cfg.desktop.workspaces.push(crate::config::DesktopWorkspace {
            name: name.to_string(),
            apps,
        });
    }

    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;

    ui_println!("Workspace saved: {}", name);
    if args.name_only {
        ui_println!("Name-only mode: no app snapshot stored.");
    }
    Ok(())
}

fn cmd_workspace_launch(args: DesktopWorkspaceLaunchCmd) -> CliResult {
    use std::collections::HashSet;

    let cfg = crate::config::load_config();
    let name = args.name.trim();
    let workspace = cfg
        .desktop
        .workspaces
        .iter()
        .find(|w| w.name == name)
        .ok_or_else(|| CliError::new(2, format!("Workspace not found: {name}")))?;

    if workspace.apps.is_empty() {
        return Err(CliError::new(2, "Workspace has no apps to launch."));
    }

    let offset = args.monitor_offset.unwrap_or(0);
    let mut moved = 0usize;
    let mut launched = 0usize;
    let mut seen: HashSet<String> = HashSet::new();

    for app in &workspace.apps {
        if !seen.insert(app.path.to_lowercase()) {
            continue;
        }

        let existing = crate::proc::find_by_name(&app.path)
            .into_iter()
            .find(|p| !p.window_title.trim().is_empty());

        if args.move_existing {
            if let Some(proc) = existing {
                if let Some(hwnd) = window_api::find_hwnd_by_pid(proc.pid).ok() {
                    if let Some(rect) = app.rect {
                        let rect = window_api::WindowRect {
                            left: rect[0] + offset,
                            top: rect[1],
                            right: rect[2] + offset,
                            bottom: rect[3],
                        };
                        window_api::apply_window_rect(hwnd, rect)
                            .map_err(|e| map_window_api_error("apply", e))?;
                    }
                    moved += 1;
                    continue;
                }
            }
        }

        let mut cmd = std::process::Command::new(&app.path);
        if let Some(args_raw) = &app.args {
            cmd.args(args_raw.split_whitespace());
        }
        let _ = cmd.spawn().map_err(|e| CliError::new(2, format!("Failed to launch {}: {e}", app.path)))?;
        launched += 1;
    }

    ui_println!("Workspace launched: {} (moved {}, launched {})", name, moved, launched);
    if args.move_existing {
        ui_println!("Move existing windows enabled.");
    }
    if offset != 0 {
        ui_println!("Monitor offset: {}", offset);
    }
    Ok(())
}

fn cmd_workspace_list(_args: DesktopWorkspaceListCmd) -> CliResult {
    let cfg = crate::config::load_config();
    if cfg.desktop.workspaces.is_empty() {
        ui_println!("No workspaces configured.");
        return Ok(());
    }

    for ws in &cfg.desktop.workspaces {
        ui_println!("{}    {} apps", ws.name, ws.apps.len());
    }
    Ok(())
}

fn cmd_workspace_remove(args: DesktopWorkspaceRemoveCmd) -> CliResult {
    let mut cfg = crate::config::load_config();
    let name = args.name.trim();
    let before = cfg.desktop.workspaces.len();
    cfg.desktop.workspaces.retain(|w| w.name != name);
    if cfg.desktop.workspaces.len() == before {
        return Err(CliError::new(2, format!("Workspace not found: {name}")));
    }
    crate::config::save_config(&cfg)
        .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
    ui_println!("Workspace removed: {}", name);
    Ok(())
}

fn cmd_window(args: DesktopWindowCmd) -> CliResult {
    match args.cmd {
        DesktopWindowSubCommand::Focus(a) => cmd_window_focus(a),
        DesktopWindowSubCommand::Move(a) => cmd_window_move(a),
        DesktopWindowSubCommand::Resize(a) => cmd_window_resize(a),
        DesktopWindowSubCommand::Transparent(a) => cmd_window_transparent(a),
        DesktopWindowSubCommand::Top(a) => cmd_window_top(a),
    }
}

fn cmd_window_focus(args: DesktopWindowFocusCmd) -> CliResult {
    let target = desktop::window::resolve_window_target(args.app.as_deref(), args.title.as_deref())?;
    desktop::window::focus_window(&target)
}

fn cmd_window_move(args: DesktopWindowMoveCmd) -> CliResult {
    let target = desktop::window::resolve_window_target(args.app.as_deref(), None)?;
    desktop::window::move_window(&target, args.x, args.y)
}

fn cmd_window_resize(args: DesktopWindowResizeCmd) -> CliResult {
    let target = desktop::window::resolve_window_target(args.app.as_deref(), None)?;
    desktop::window::resize_window(&target, args.width, args.height)
}

fn cmd_window_transparent(args: DesktopWindowTransparentCmd) -> CliResult {
    let target = desktop::window::resolve_window_target(args.app.as_deref(), None)?;
    desktop::window::set_transparency(&target, args.alpha)
}

fn cmd_window_top(args: DesktopWindowTopCmd) -> CliResult {
    let target = desktop::window::resolve_window_target(args.app.as_deref(), None)?;
    let mode = if args.enable {
        desktop::window::TopmostMode::Enable
    } else if args.disable {
        desktop::window::TopmostMode::Disable
    } else {
        return Err(CliError::with_details(
            2,
            "Missing --enable or --disable.".to_string(),
            &["Fix: use --enable or --disable."],
        ));
    };
    desktop::window::set_topmost(&target, mode)
}

fn cmd_theme(args: DesktopThemeCmd) -> CliResult {
    match args.cmd {
        DesktopThemeSubCommand::Set(a) => cmd_theme_set(a),
        DesktopThemeSubCommand::Toggle(a) => cmd_theme_toggle(a),
        DesktopThemeSubCommand::Schedule(a) => cmd_theme_schedule(a),
        DesktopThemeSubCommand::Status(a) => cmd_theme_status(a),
    }
}

fn cmd_theme_set(args: DesktopThemeSetCmd) -> CliResult {
    let mode = match args.mode.to_lowercase().as_str() {
        "light" => desktop::theme::ThemeMode::Light,
        "dark" => desktop::theme::ThemeMode::Dark,
        _ => {
            return Err(CliError::with_details(
                2,
                format!("Invalid theme mode: {}", args.mode),
                &["Fix: use light or dark."],
            ))
        }
    };
    desktop::theme::set_theme(&mode)?;
    ui_println!("主题已切换为 {}", mode.label());
    Ok(())
}

fn cmd_theme_toggle(_args: DesktopThemeToggleCmd) -> CliResult {
    let mode = desktop::theme::toggle_theme()?;
    ui_println!("主题已切换为 {}", mode.label());
    Ok(())
}

fn cmd_theme_schedule(args: DesktopThemeScheduleCmd) -> CliResult {
    let Some(light_at) = args.light else {
        return Err(CliError::with_details(
            2,
            "Missing --light time.".to_string(),
            &["Fix: provide --light HH:MM."],
        ));
    };
    let Some(dark_at) = args.dark else {
        return Err(CliError::with_details(
            2,
            "Missing --dark time.".to_string(),
            &["Fix: provide --dark HH:MM."],
        ));
    };
    let schedule = desktop::theme::ThemeSchedule::new();
    desktop::theme::ThemeSchedule::start(&schedule, light_at.clone(), dark_at.clone())?;
    ui_println!("主题定时已启动：{} / {}", light_at, dark_at);
    Ok(())
}

fn cmd_theme_status(_args: DesktopThemeStatusCmd) -> CliResult {
    let mode = desktop::theme::get_current_theme();
    ui_println!("当前主题：{}", mode.label());
    Ok(())
}

fn cmd_awake(args: DesktopAwakeCmd) -> CliResult {
    match args.cmd {
        DesktopAwakeSubCommand::On(a) => cmd_awake_on(a),
        DesktopAwakeSubCommand::Off(a) => cmd_awake_off(a),
        DesktopAwakeSubCommand::Status(a) => cmd_awake_status(a),
    }
}

fn cmd_awake_on(args: DesktopAwakeOnCmd) -> CliResult {
    let state = desktop::awake::AwakeState::new();
    if let Some(duration) = args.duration.as_deref() {
        let dur = desktop::awake::parse_duration(duration)?;
        desktop::awake::awake_timed(args.display_on, dur, &state, || {})?;
        ui_println!("Awake 已启动（定时 {}）", duration);
        return Ok(());
    }
    if let Some(expire_at) = args.expire_at.as_deref() {
        let dur = desktop::awake::parse_expire_at(expire_at)?;
        desktop::awake::awake_timed(args.display_on, dur, &state, || {})?;
        ui_println!("Awake 已启动（到 {} 结束）", expire_at);
        return Ok(());
    }
    desktop::awake::awake_indefinite(args.display_on, &state)?;
    ui_println!("Awake 已启动（持续唤醒）");
    Ok(())
}

fn cmd_awake_off(_args: DesktopAwakeOffCmd) -> CliResult {
    ui_println!("Awake 取消需由 daemon 管理（Phase 2）。");
    Ok(())
}

fn cmd_awake_status(_args: DesktopAwakeStatusCmd) -> CliResult {
    ui_println!("Awake 状态需由 daemon 查询（Phase 2）。");
    Ok(())
}

fn cmd_color(args: DesktopColorCmd) -> CliResult {
    let color = desktop::color::pick_color()?;
    let hex = desktop::color::color_to_hex(color);
    ui_println!("{hex}");
    if args.copy {
        if desktop::color::copy_to_clipboard(&hex) {
            ui_println!("Copied to clipboard.");
        } else {
            crate::output::emit_warning("Failed to copy to clipboard.", &[]);
        }
    }
    Ok(())
}

fn cmd_hosts(args: DesktopHostsCmd) -> CliResult {
    match args.cmd {
        DesktopHostsSubCommand::Add(a) => cmd_hosts_add(a),
        DesktopHostsSubCommand::Remove(a) => cmd_hosts_remove(a),
        DesktopHostsSubCommand::List(a) => cmd_hosts_list(a),
    }
}

fn cmd_hosts_add(args: DesktopHostsAddCmd) -> CliResult {
    let host = args.host.trim();
    if host.is_empty() {
        return Err(CliError::new(2, "Host is required."));
    }
    let ip = args.ip.trim();
    if ip.is_empty() {
        return Err(CliError::new(2, "IP is required."));
    }
    if args.dry_run {
        let entry = desktop::hosts::preview_add_entry(ip, host)?;
        ui_println!("Dry-run: Hosts entry would be added: {} {}", entry.ip, entry.host);
        return Ok(());
    }
    desktop::hosts::add_entry(ip, host)?;
    ui_println!("Hosts entry added: {} {}", ip, host);
    Ok(())
}

fn cmd_hosts_remove(args: DesktopHostsRemoveCmd) -> CliResult {
    let host = args.host.trim();
    if host.is_empty() {
        return Err(CliError::new(2, "Host is required."));
    }
    if args.dry_run {
        let removed = desktop::hosts::preview_remove_entry(host)?;
        if !removed {
            return Err(CliError::new(2, format!("Host not found: {host}")));
        }
        ui_println!("Dry-run: Hosts entry would be removed: {}", host);
        return Ok(());
    }
    let removed = desktop::hosts::remove_entry(host)?;
    if !removed {
        return Err(CliError::new(2, format!("Host not found: {host}")));
    }
    ui_println!("Hosts entry removed: {}", host);
    Ok(())
}

fn cmd_hosts_list(_args: DesktopHostsListCmd) -> CliResult {
    let entries = desktop::hosts::list_entries()?;
    if entries.is_empty() {
        ui_println!("No hosts entries.");
        return Ok(());
    }
    for entry in entries {
        if let Some(comment) = entry.comment.as_deref() {
            ui_println!("{}\t{}\t# {}", entry.ip, entry.host, comment);
        } else {
            ui_println!("{}\t{}", entry.ip, entry.host);
        }
    }
    Ok(())
}

fn cmd_run(args: DesktopRunCmd) -> CliResult {
    let command = args.command.trim();
    if command.is_empty() {
        return Err(CliError::new(2, "Command is required."));
    }
    desktop::shell::run_command(command)?;
    ui_println!("Command launched.");
    Ok(())
}

fn cmd_tui() -> CliResult {
    #[cfg(not(feature = "tui"))]
    {
        return Err(CliError::with_details(
            2,
            "tui feature is not enabled".to_string(),
            &["Fix: Run `cargo run --features desktop,tui -- desktop tui`."],
        ));
    }
    #[cfg(feature = "tui")]
    {
        return crate::commands::desktop_tui::run_desktop_tui();
    }
}

fn cmd_app(args: DesktopAppCmd) -> CliResult {
    match args.cmd {
        DesktopAppSubCommand::List(a) => cmd_app_list(a),
    }
}

fn cmd_app_list(_args: DesktopAppListCmd) -> CliResult {
    let apps = desktop::apps::list_installed_apps();
    if apps.is_empty() {
        ui_println!("No apps found.");
        return Ok(());
    }

    for (idx, app) in apps.iter().enumerate() {
        ui_println!("{:>3}. {}", idx + 1, app.name);
        ui_println!("     {}", app.path);
    }
    Ok(())
}


