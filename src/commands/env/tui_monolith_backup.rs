#[cfg(not(feature = "tui"))]
use crate::output::{CliError, CliResult};

#[cfg(not(feature = "tui"))]
pub(crate) fn run_env_tui() -> CliResult {
    Err(CliError::with_details(
        2,
        "tui feature is not enabled".to_string(),
        &["Fix: Run `cargo run --features tui -- env tui`."],
    ))
}

#[cfg(feature = "tui")]
mod imp {
    use std::collections::HashMap;
    use std::io;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use dialoguer::{Confirm, Input, Select};
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Constraint, Direction, Layout, Rect};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph,
    };

    use crate::env_core::types::{
        DoctorReport, EnvAuditEntry, EnvProfileMeta, EnvScope, EnvVar, ExportFormat,
        ImportStrategy, SnapshotMeta,
    };
    use crate::env_core::{EnvManager, doctor, uac};
    use crate::output::{CliError, CliResult};

    static FORCE_FULL_REDRAW: AtomicBool = AtomicBool::new(false);

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Panel {
        Vars,
        Path,
        Snapshots,
        Profiles,
        History,
        Doctor,
        Io,
    }

    impl Panel {
        fn next(self) -> Self {
            match self {
                Self::Vars => Self::Path,
                Self::Path => Self::Snapshots,
                Self::Snapshots => Self::Profiles,
                Self::Profiles => Self::History,
                Self::History => Self::Doctor,
                Self::Doctor => Self::Io,
                Self::Io => Self::Vars,
            }
        }

        fn label(self) -> &'static str {
            match self {
                Self::Vars => "Vars",
                Self::Path => "PATH",
                Self::Snapshots => "Snapshots",
                Self::Profiles => "Profiles",
                Self::History => "History",
                Self::Doctor => "Doctor",
                Self::Io => "Import/Export",
            }
        }
    }

    pub(crate) fn run_env_tui() -> CliResult {
        let mut app = App::new();

        enable_raw_mode().map_err(|e| CliError::new(1, format!("tui init: {}", e)))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|e| CliError::new(1, format!("{e}")))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|e| CliError::new(1, format!("{e}")))?;

        let result = run_loop(&mut terminal, &mut app);

        let _ = disable_raw_mode();
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
        let _ = terminal.show_cursor();

        result
    }

    fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> CliResult {
        loop {
            if FORCE_FULL_REDRAW.swap(false, Ordering::SeqCst) {
                terminal
                    .clear()
                    .map_err(|e| CliError::new(1, format!("tui clear: {}", e)))?;
            }
            terminal
                .draw(|f| draw_ui(f, app))
                .map_err(|e| CliError::new(1, format!("tui draw: {}", e)))?;

            let ev = event::read().map_err(|e| CliError::new(1, format!("tui event: {}", e)))?;
            if let Event::Key(key) = ev {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if app.show_help {
                    if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
                        app.show_help = false;
                    }
                    continue;
                }
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                    break;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('z') | KeyCode::Char('Z'))
                {
                    handle_undo(app)?;
                    continue;
                }
                if matches!(key.code, KeyCode::Char('?')) {
                    app.show_help = true;
                    continue;
                }
                if matches!(key.code, KeyCode::Tab) {
                    app.panel = app.panel.next();
                    continue;
                }
                if matches!(key.code, KeyCode::Char('h') | KeyCode::Char('H')) {
                    app.panel = Panel::History;
                    continue;
                }
                if matches!(key.code, KeyCode::Char('p') | KeyCode::Char('P')) {
                    app.panel = Panel::Profiles;
                    continue;
                }
                if matches!(key.code, KeyCode::Char('r') | KeyCode::F(5)) {
                    app.refresh_all();
                    continue;
                }
                if matches!(key.code, KeyCode::Char('s')) {
                    app.scope = if app.scope == EnvScope::User {
                        EnvScope::System
                    } else {
                        EnvScope::User
                    };
                    if app.scope == EnvScope::System && !app.is_elevated {
                        app.status = "warning: system scope write requires Administrator token"
                            .to_string();
                    }
                    app.refresh_all();
                    continue;
                }
                if matches!(key.code, KeyCode::Up | KeyCode::Char('k')) {
                    app.move_cursor(-1);
                    continue;
                }
                if matches!(key.code, KeyCode::Down | KeyCode::Char('j')) {
                    app.move_cursor(1);
                    continue;
                }
                handle_panel_key(app, key.code)?;
            }
        }
        Ok(())
    }

    fn handle_panel_key(app: &mut App, key: KeyCode) -> CliResult {
        match app.panel {
            Panel::Vars => handle_vars_key(app, key),
            Panel::Path => handle_path_key(app, key),
            Panel::Snapshots => handle_snapshot_key(app, key),
            Panel::Profiles => handle_profiles_key(app, key),
            Panel::History => handle_history_key(app, key),
            Panel::Doctor => handle_doctor_key(app, key),
            Panel::Io => handle_io_key(app, key),
        }
    }

    fn handle_vars_key(app: &mut App, key: KeyCode) -> CliResult {
        match key {
            KeyCode::Char('/') => {
                let query = prompt_text("Search vars (name/value, empty=all)", &app.var_query)?;
                app.var_query = query.trim().to_string();
                app.rebuild_var_filter();
                app.status = format!(
                    "vars filtered: {}/{}",
                    app.filtered_vars.len(),
                    app.vars.len()
                );
            }
            KeyCode::Char('C') => {
                app.var_query.clear();
                app.rebuild_var_filter();
                app.status = "vars filter cleared".to_string();
            }
            KeyCode::Char('n') => {
                if let Some((name, value)) = prompt_new_var()? {
                    app.manager
                        .set_var(app.scope, &name, &value, false)
                        .map_err(map_env_err)?;
                    app.status = format!("set {}", name);
                    app.refresh_all();
                }
            }
            KeyCode::Char('e') => {
                if let Some((name, current)) = app.current_var().map(|v| (v.name, v.raw_value)) {
                    if let Some(value) = prompt_edit_var(&name, &current)? {
                        app.manager
                            .set_var(app.scope, &name, &value, false)
                            .map_err(map_env_err)?;
                        app.status = format!("updated {}", name);
                        app.refresh_all();
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(var) = app.current_var() {
                    if prompt_yes_no(&format!("Delete {}?", var.name))? {
                        app.manager
                            .delete_var(app.scope, &var.name)
                            .map_err(map_env_err)?;
                        app.status = format!("deleted {}", var.name);
                        app.refresh_all();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_path_key(app: &mut App, key: KeyCode) -> CliResult {
        match key {
            KeyCode::Char('a') => {
                if let Some(entry) = prompt_path_entry("Add PATH entry (tail)")? {
                    app.manager
                        .path_add(app.scope, &entry, false)
                        .map_err(map_env_err)?;
                    app.status = "PATH appended".to_string();
                    app.refresh_all();
                }
            }
            KeyCode::Char('A') => {
                if let Some(entry) = prompt_path_entry("Add PATH entry (head)")? {
                    app.manager
                        .path_add(app.scope, &entry, true)
                        .map_err(map_env_err)?;
                    app.status = "PATH prepended".to_string();
                    app.refresh_all();
                }
            }
            KeyCode::Char('d') => {
                if let Some(entry) = app.current_path() {
                    if prompt_yes_no(&format!("Remove PATH entry?\n{}", entry))? {
                        app.manager
                            .path_remove(app.scope, &entry)
                            .map_err(map_env_err)?;
                        app.status = "PATH removed".to_string();
                        app.refresh_all();
                    }
                }
            }
            KeyCode::Char('H') => {
                if let Some(entry) = app.current_path() {
                    app.manager
                        .path_remove(app.scope, &entry)
                        .map_err(map_env_err)?;
                    app.manager
                        .path_add(app.scope, &entry, true)
                        .map_err(map_env_err)?;
                    app.status = "PATH entry moved to head".to_string();
                    app.refresh_all();
                }
            }
            KeyCode::Char('T') => {
                if let Some(entry) = app.current_path() {
                    app.manager
                        .path_remove(app.scope, &entry)
                        .map_err(map_env_err)?;
                    app.manager
                        .path_add(app.scope, &entry, false)
                        .map_err(map_env_err)?;
                    app.status = "PATH entry moved to tail".to_string();
                    app.refresh_all();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_snapshot_key(app: &mut App, key: KeyCode) -> CliResult {
        match key {
            KeyCode::Char('c') => {
                let desc = prompt_text("Snapshot description", "manual snapshot")?;
                let meta = app
                    .manager
                    .snapshot_create(Some(&desc))
                    .map_err(map_env_err)?;
                app.status = format!("snapshot {}", meta.id);
                app.refresh_all();
            }
            KeyCode::Char('R') => {
                if let Some(id) = app.current_snapshot_id() {
                    if prompt_yes_no(&format!("Restore snapshot {} ?", id))? {
                        app.manager
                            .snapshot_restore(app.scope, Some(&id), false)
                            .map_err(map_env_err)?;
                        app.status = format!("restored {}", id);
                        app.refresh_all();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_doctor_key(app: &mut App, key: KeyCode) -> CliResult {
        match key {
            KeyCode::Char('g') => {
                let report = app.manager.doctor_run(app.scope).map_err(map_env_err)?;
                app.status = format!("doctor issues={}", report.issues.len());
                app.doctor = Some(report);
            }
            KeyCode::Char('f') => {
                if prompt_yes_no("Apply doctor fixes?")? {
                    let fixed = app.manager.doctor_fix(app.scope).map_err(map_env_err)?;
                    app.status = format!("fixed {}", fixed.fixed);
                    app.doctor = app.manager.doctor_run(app.scope).ok();
                    app.refresh_all();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_profiles_key(app: &mut App, key: KeyCode) -> CliResult {
        match key {
            KeyCode::Char('c') => {
                let default_name = default_profile_name();
                let name = prompt_text("Profile name", &default_name)?;
                if !name.trim().is_empty() {
                    let meta = app
                        .manager
                        .profile_capture(name.trim(), app.scope)
                        .map_err(map_env_err)?;
                    app.status = format!("captured profile {}", meta.name);
                    app.refresh_all();
                }
            }
            KeyCode::Char('a') | KeyCode::Enter => {
                if let Some(name) = app.current_profile_name() {
                    let prompt = format!("Apply profile {} to {} scope?", name, app.scope);
                    if prompt_yes_no(&prompt)? {
                        let meta = app
                            .manager
                            .profile_apply(&name, Some(app.scope))
                            .map_err(map_env_err)?;
                        app.status = format!("applied profile {} ({})", meta.name, meta.var_count);
                        app.refresh_all();
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(name) = app.current_profile_name() {
                    let prompt = format!("Delete profile {}?", name);
                    if prompt_yes_no(&prompt)? {
                        let deleted = app.manager.profile_delete(&name).map_err(map_env_err)?;
                        app.status = if deleted {
                            format!("deleted profile {}", name)
                        } else {
                            format!("profile {} not found", name)
                        };
                        app.refresh_all();
                    }
                }
            }
            KeyCode::Char('v') => {
                if let Some(name) = app.current_profile_name() {
                    let diff = app
                        .manager
                        .profile_diff(&name, Some(app.scope))
                        .map_err(map_env_err)?;
                    app.status = format!("profile {} diff changes={}", name, diff.total_changes());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_history_key(app: &mut App, key: KeyCode) -> CliResult {
        match key {
            KeyCode::Char('u') => {
                handle_undo(app)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_undo(app: &mut App) -> CliResult {
        if !prompt_yes_no(&format!(
            "Undo last change by restoring latest snapshot for {} scope?",
            app.scope
        ))? {
            app.status = "undo cancelled".to_string();
            return Ok(());
        }
        let restored = app
            .manager
            .snapshot_restore(app.scope, None, true)
            .map_err(map_env_err)?;
        app.status = format!("undo restored {}", restored.id);
        app.refresh_all();
        Ok(())
    }

    fn handle_io_key(app: &mut App, key: KeyCode) -> CliResult {
        match key {
            KeyCode::Char('x') => {
                if let Some((format, path)) = prompt_export_target()? {
                    let data = app
                        .manager
                        .export_vars(app.scope, format)
                        .map_err(map_env_err)?;
                    std::fs::write(&path, data).map_err(|e| CliError::new(1, format!("{e}")))?;
                    app.status = format!("exported {}", path.display());
                }
            }
            KeyCode::Char('i') => {
                if let Some((path, strategy, dry_run)) = prompt_import_source()? {
                    let preview = app
                        .manager
                        .import_file(app.scope, &path, strategy, true)
                        .map_err(map_env_err)?;

                    if dry_run {
                        app.status = format!(
                            "import preview added={} updated={} skipped={}",
                            preview.added, preview.updated, preview.skipped
                        );
                    } else {
                        let prompt = format!(
                            "Import preview: added={} updated={} skipped={}. Apply now?",
                            preview.added, preview.updated, preview.skipped
                        );
                        if prompt_yes_no(&prompt)? {
                            let res = app
                                .manager
                                .import_file(app.scope, &path, strategy, false)
                                .map_err(map_env_err)?;
                            app.status = format!(
                                "import applied added={} updated={} skipped={}",
                                res.added, res.updated, res.skipped
                            );
                            app.refresh_all();
                        } else {
                            app.status = "import cancelled after preview".to_string();
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn draw_ui(f: &mut ratatui::Frame, app: &mut App) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Min(4),
                Constraint::Length(1),
            ])
            .split(area);

        let header = Block::default()
            .title(format!(
                " env tui | scope={} | panel={} ",
                app.scope,
                app.panel.label()
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));
        f.render_widget(header, chunks[0]);

        let tabs = [
            Panel::Vars,
            Panel::Path,
            Panel::Snapshots,
            Panel::Profiles,
            Panel::History,
            Panel::Doctor,
            Panel::Io,
        ];
        let tab_line = Line::from(
            tabs.iter()
                .flat_map(|p| {
                    let active = *p == app.panel;
                    vec![
                        Span::styled(
                            format!(" {} ", p.label()),
                            Style::default()
                                .fg(if active {
                                    Color::Green
                                } else {
                                    Color::DarkGray
                                })
                                .add_modifier(if active {
                                    Modifier::BOLD
                                } else {
                                    Modifier::empty()
                                }),
                        ),
                        Span::raw(" "),
                    ]
                })
                .collect::<Vec<_>>(),
        );
        f.render_widget(Paragraph::new(tab_line), chunks[1]);

        match app.panel {
            Panel::Vars => {
                let items: Vec<ListItem> = app
                    .filtered_vars
                    .iter()
                    .filter_map(|idx| app.vars.get(*idx))
                    .map(|v| {
                        ListItem::new(format!("{} = {}", v.name, trim_for_ui(&v.raw_value, 96)))
                    })
                    .collect();
                let title = if app.var_query.is_empty() {
                    format!(
                        " Vars [N:new E:edit D:delete /:search C:clear] ({}) ",
                        app.vars.len()
                    )
                } else {
                    format!(
                        " Vars [N:new E:edit D:delete /:search C:clear] ({}/{}, q={}) ",
                        app.filtered_vars.len(),
                        app.vars.len(),
                        app.var_query
                    )
                };
                let list = List::new(items)
                    .block(Block::default().title(title).borders(Borders::ALL))
                    .highlight_style(Style::default().bg(Color::Rgb(32, 42, 56)));
                f.render_stateful_widget(list, chunks[2], &mut app.var_state);
            }
            Panel::Path => {
                let mut dup_count: HashMap<String, usize> = HashMap::new();
                for p in &app.paths {
                    let key = normalize_path_key_for_ui(p);
                    *dup_count.entry(key).or_insert(0) += 1;
                }
                let items: Vec<ListItem> = app
                    .paths
                    .iter()
                    .enumerate()
                    .map(|(idx, p)| {
                        let key = normalize_path_key_for_ui(p);
                        let duplicate = dup_count.get(&key).copied().unwrap_or(0) > 1;
                        let missing = !path_entry_exists(p);
                        let tag = match (duplicate, missing) {
                            (true, true) => " [dup|missing]",
                            (true, false) => " [dup]",
                            (false, true) => " [missing]",
                            (false, false) => "",
                        };
                        ListItem::new(format!("{:02}. {}{}", idx + 1, p, tag))
                    })
                    .collect();
                let list = List::new(items)
                    .block(
                        Block::default()
                            .title(" PATH [a:tail A:head d:remove H:to-head T:to-tail] ")
                            .borders(Borders::ALL),
                    )
                    .highlight_style(Style::default().bg(Color::Rgb(32, 42, 56)));
                f.render_stateful_widget(list, chunks[2], &mut app.path_state);
            }
            Panel::Snapshots => {
                let inner = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
                    .split(chunks[2]);
                let items: Vec<ListItem> = app
                    .snapshots
                    .iter()
                    .map(|s| ListItem::new(format!("{} | {}", s.id, s.description)))
                    .collect();
                let list = List::new(items)
                    .block(
                        Block::default()
                            .title(" Snapshots [c:create R:restore] ")
                            .borders(Borders::ALL),
                    )
                    .highlight_style(Style::default().bg(Color::Rgb(32, 42, 56)));
                f.render_stateful_widget(list, inner[0], &mut app.snapshot_state);

                let detail = if let Some(s) = app.current_snapshot() {
                    format!(
                        "id: {}\ncreated_at: {}\ndesc: {}\nfile: {}",
                        s.id,
                        s.created_at,
                        s.description,
                        s.path.display()
                    )
                } else {
                    "No snapshot selected.".to_string()
                };
                f.render_widget(
                    Paragraph::new(detail).block(
                        Block::default()
                            .title(" Snapshot Preview ")
                            .borders(Borders::ALL),
                    ),
                    inner[1],
                );
            }
            Panel::Profiles => {
                let items: Vec<ListItem> = app
                    .profiles
                    .iter()
                    .map(|p| {
                        ListItem::new(format!(
                            "{} | {} | vars={} | {}",
                            p.name, p.scope, p.var_count, p.created_at
                        ))
                    })
                    .collect();
                let list = List::new(items)
                    .block(
                        Block::default()
                            .title(" Profiles [c:capture a/apply d:delete v:diff] ")
                            .borders(Borders::ALL),
                    )
                    .highlight_style(Style::default().bg(Color::Rgb(32, 42, 56)));
                f.render_stateful_widget(list, chunks[2], &mut app.profile_state);
            }
            Panel::History => {
                let items: Vec<ListItem> = app
                    .audit_entries
                    .iter()
                    .map(|e| {
                        let name = e.name.clone().unwrap_or_else(|| "-".to_string());
                        let msg = e.message.clone().unwrap_or_default();
                        ListItem::new(format!(
                            "{} | {} | {} | {} | {}",
                            e.at,
                            e.action,
                            e.result,
                            name,
                            trim_for_ui(&msg, 56)
                        ))
                    })
                    .collect();
                let list = List::new(items)
                    .block(
                        Block::default()
                            .title(" History [u:undo latest snapshot] ")
                            .borders(Borders::ALL),
                    )
                    .highlight_style(Style::default().bg(Color::Rgb(32, 42, 56)));
                f.render_stateful_widget(list, chunks[2], &mut app.audit_state);
            }
            Panel::Doctor => {
                let text = if let Some(report) = &app.doctor {
                    if report.issues.is_empty() {
                        "OK: no issues".to_string()
                    } else {
                        doctor::report_text(report)
                    }
                } else {
                    "Press [G] run doctor, [F] fix.".to_string()
                };
                f.render_widget(
                    Paragraph::new(text).block(
                        Block::default()
                            .title(" Doctor [g:run f:fix] ")
                            .borders(Borders::ALL),
                    ),
                    chunks[2],
                );
            }
            Panel::Io => {
                let text = [
                    "Import/Export actions:",
                    "[X] Export current scope",
                    "[I] Import file (json/env/reg/csv)",
                ]
                .join("\n");
                f.render_widget(
                    Paragraph::new(text).block(
                        Block::default()
                            .title(" Import/Export [x:export i:import] ")
                            .borders(Borders::ALL),
                    ),
                    chunks[2],
                );
            }
        }

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::DarkGray)),
            Span::raw(" panel  "),
            Span::styled("S", Style::default().fg(Color::DarkGray)),
            Span::raw(" scope  "),
            Span::styled("R", Style::default().fg(Color::DarkGray)),
            Span::raw(" refresh  "),
            Span::styled("Ctrl+Z", Style::default().fg(Color::DarkGray)),
            Span::raw(" undo  "),
            Span::styled("?", Style::default().fg(Color::DarkGray)),
            Span::raw(" help  "),
            Span::styled("Q", Style::default().fg(Color::DarkGray)),
            Span::raw(" quit  |  "),
            Span::raw(app.status.as_str()),
            if app.scope == EnvScope::System && !app.is_elevated {
                Span::styled(
                    "  [system scope needs Administrator token]",
                    Style::default().fg(Color::Yellow),
                )
            } else {
                Span::raw("")
            },
        ]));
        f.render_widget(footer, chunks[3]);

        if app.show_help {
            let popup = centered_rect(78, 72, area);
            let help = [
                "Env TUI Help",
                "",
                "Global:",
                "  Tab switch panel, H history, P profiles",
                "  S switch scope, R/F5 refresh, Ctrl+Z undo, Q quit",
                "  Up/Down or J/K move cursor, ? show/hide this help",
                "",
                "Vars: / search, C clear, N new, E edit, D delete",
                "PATH: a tail add, A head add, d remove, H/T move",
                "Snapshots: c create, R restore",
                "Profiles: c capture, a/apply, d delete, v diff",
                "History: read audit trail, u undo latest snapshot",
                "Doctor: g run, f fix",
                "Import/Export: x export, i import (with preview)",
                "",
                "Press Esc or ? to close help.",
            ]
            .join("\n");
            f.render_widget(Clear, popup);
            f.render_widget(
                Paragraph::new(help).block(
                    Block::default()
                        .title(" Help ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow)),
                ),
                popup,
            );
        }
    }

    struct App {
        manager: EnvManager,
        scope: EnvScope,
        panel: Panel,
        vars: Vec<EnvVar>,
        filtered_vars: Vec<usize>,
        var_query: String,
        var_state: ListState,
        paths: Vec<String>,
        path_state: ListState,
        snapshots: Vec<SnapshotMeta>,
        snapshot_state: ListState,
        profiles: Vec<EnvProfileMeta>,
        profile_state: ListState,
        audit_entries: Vec<EnvAuditEntry>,
        audit_state: ListState,
        doctor: Option<DoctorReport>,
        status: String,
        is_elevated: bool,
        show_help: bool,
    }

    impl App {
        fn new() -> Self {
            let mut app = Self {
                manager: EnvManager::new(),
                scope: EnvScope::User,
                panel: Panel::Vars,
                vars: Vec::new(),
                filtered_vars: Vec::new(),
                var_query: String::new(),
                var_state: ListState::default(),
                paths: Vec::new(),
                path_state: ListState::default(),
                snapshots: Vec::new(),
                snapshot_state: ListState::default(),
                profiles: Vec::new(),
                profile_state: ListState::default(),
                audit_entries: Vec::new(),
                audit_state: ListState::default(),
                doctor: None,
                status: "ready".to_string(),
                is_elevated: uac::is_elevated(),
                show_help: false,
            };
            app.refresh_all();
            app
        }

        fn refresh_all(&mut self) {
            match self.manager.list_vars(self.scope) {
                Ok(v) => {
                    self.vars = v;
                    self.rebuild_var_filter();
                }
                Err(e) => self.status = e.to_string(),
            }
            match self.manager.path_entries(self.scope) {
                Ok(v) => {
                    self.paths = v;
                    sync_selection(&mut self.path_state, self.paths.len());
                }
                Err(e) => self.status = e.to_string(),
            }
            match self.manager.snapshot_list() {
                Ok(v) => {
                    self.snapshots = v;
                    sync_selection(&mut self.snapshot_state, self.snapshots.len());
                }
                Err(e) => self.status = e.to_string(),
            }
            match self.manager.profile_list() {
                Ok(v) => {
                    self.profiles = v;
                    sync_selection(&mut self.profile_state, self.profiles.len());
                }
                Err(e) => self.status = e.to_string(),
            }
            match self.manager.audit_list(200) {
                Ok(mut v) => {
                    v.reverse();
                    self.audit_entries = v;
                    sync_selection(&mut self.audit_state, self.audit_entries.len());
                }
                Err(e) => self.status = e.to_string(),
            }
        }

        fn rebuild_var_filter(&mut self) {
            self.filtered_vars.clear();
            if self.var_query.trim().is_empty() {
                self.filtered_vars.extend(0..self.vars.len());
            } else {
                let query = self.var_query.to_ascii_lowercase();
                self.filtered_vars
                    .extend(self.vars.iter().enumerate().filter_map(|(idx, v)| {
                        let name = v.name.to_ascii_lowercase();
                        let value = v.raw_value.to_ascii_lowercase();
                        if name.contains(&query) || value.contains(&query) {
                            Some(idx)
                        } else {
                            None
                        }
                    }));
            }
            sync_selection(&mut self.var_state, self.filtered_vars.len());
        }

        fn move_cursor(&mut self, delta: i32) {
            let (len, state) = match self.panel {
                Panel::Vars => (self.filtered_vars.len(), &mut self.var_state),
                Panel::Path => (self.paths.len(), &mut self.path_state),
                Panel::Snapshots => (self.snapshots.len(), &mut self.snapshot_state),
                Panel::Profiles => (self.profiles.len(), &mut self.profile_state),
                Panel::History => (self.audit_entries.len(), &mut self.audit_state),
                Panel::Doctor | Panel::Io => return,
            };
            if len == 0 {
                return;
            }
            let cur = state.selected().unwrap_or(0) as i32;
            let next = (cur + delta).rem_euclid(len as i32) as usize;
            state.select(Some(next));
        }

        fn current_var(&self) -> Option<EnvVar> {
            let view_idx = self.var_state.selected()?;
            let idx = *self.filtered_vars.get(view_idx)?;
            self.vars.get(idx).cloned()
        }

        fn current_path(&self) -> Option<String> {
            let idx = self.path_state.selected()?;
            self.paths.get(idx).cloned()
        }

        fn current_snapshot_id(&self) -> Option<String> {
            let idx = self.snapshot_state.selected()?;
            self.snapshots.get(idx).map(|s| s.id.clone())
        }

        fn current_snapshot(&self) -> Option<SnapshotMeta> {
            let idx = self.snapshot_state.selected()?;
            self.snapshots.get(idx).cloned()
        }

        fn current_profile_name(&self) -> Option<String> {
            let idx = self.profile_state.selected()?;
            self.profiles.get(idx).map(|p| p.name.clone())
        }
    }

    fn sync_selection(state: &mut ListState, len: usize) {
        if len == 0 {
            state.select(None);
            return;
        }
        let idx = state.selected().unwrap_or(0).min(len - 1);
        state.select(Some(idx));
    }

    fn default_profile_name() -> String {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("profile-{}", ts)
    }

    fn prompt_new_var() -> CliResult<Option<(String, String)>> {
        prompt_interactive(|| {
            let name: String = Input::new().with_prompt("Variable name").interact_text()?;
            if name.trim().is_empty() {
                return Ok(None);
            }
            let value: String = Input::new().with_prompt("Variable value").interact_text()?;
            Ok(Some((name, value)))
        })
    }

    fn prompt_edit_var(name: &str, current: &str) -> CliResult<Option<String>> {
        prompt_interactive(|| {
            let value: String = Input::new()
                .with_prompt(format!("Edit {}", name))
                .default(current.to_string())
                .interact_text()?;
            Ok(Some(value))
        })
    }

    fn prompt_path_entry(prompt: &str) -> CliResult<Option<String>> {
        prompt_interactive(|| {
            let entry: String = Input::new().with_prompt(prompt).interact_text()?;
            if entry.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(entry))
            }
        })
    }

    fn prompt_text(prompt: &str, default: &str) -> CliResult<String> {
        prompt_interactive(|| {
            Input::new()
                .with_prompt(prompt)
                .default(default.to_string())
                .interact_text()
        })
    }

    fn prompt_yes_no(prompt: &str) -> CliResult<bool> {
        prompt_interactive(|| Confirm::new().with_prompt(prompt).default(false).interact())
    }

    fn prompt_export_target() -> CliResult<Option<(ExportFormat, PathBuf)>> {
        prompt_interactive(|| {
            let formats = ["json", "env", "reg", "csv"];
            let idx = Select::new()
                .with_prompt("Export format")
                .items(&formats)
                .default(0)
                .interact()?;
            let format = match formats[idx] {
                "json" => ExportFormat::Json,
                "env" => ExportFormat::Env,
                "reg" => ExportFormat::Reg,
                _ => ExportFormat::Csv,
            };
            let path: String = Input::new()
                .with_prompt("Output file path")
                .interact_text()?;
            if path.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some((format, PathBuf::from(path))))
            }
        })
    }

    fn prompt_import_source() -> CliResult<Option<(PathBuf, ImportStrategy, bool)>> {
        prompt_interactive(|| {
            let path: String = Input::new()
                .with_prompt("Import file path")
                .interact_text()?;
            if path.trim().is_empty() {
                return Ok(None);
            }
            let modes = ["merge", "overwrite"];
            let mode_idx = Select::new()
                .with_prompt("Import mode")
                .items(&modes)
                .default(0)
                .interact()?;
            let strategy = if mode_idx == 0 {
                ImportStrategy::Merge
            } else {
                ImportStrategy::Overwrite
            };
            let dry_run = Confirm::new()
                .with_prompt("Dry run only?")
                .default(true)
                .interact()?;
            Ok(Some((PathBuf::from(path), strategy, dry_run)))
        })
    }

    fn prompt_interactive<T, F>(f: F) -> CliResult<T>
    where
        F: FnOnce() -> Result<T, dialoguer::Error>,
    {
        disable_raw_mode().map_err(|e| CliError::new(1, format!("{e}")))?;
        execute!(io::stdout(), LeaveAlternateScreen)
            .map_err(|e| CliError::new(1, format!("{e}")))?;
        let result = f().map_err(|e| CliError::new(1, format!("prompt failed: {}", e)));
        execute!(io::stdout(), EnterAlternateScreen)
            .map_err(|e| CliError::new(1, format!("{e}")))?;
        enable_raw_mode().map_err(|e| CliError::new(1, format!("{e}")))?;
        FORCE_FULL_REDRAW.store(true, Ordering::SeqCst);
        result
    }

    fn map_env_err(err: crate::env_core::types::EnvError) -> CliError {
        CliError::new(err.exit_code(), err.to_string())
    }

    fn trim_for_ui(value: &str, max: usize) -> String {
        if value.chars().count() <= max {
            return value.to_string();
        }
        let mut out = String::new();
        for ch in value.chars().take(max.saturating_sub(1)) {
            out.push(ch);
        }
        out.push('…');
        out
    }

    fn normalize_path_key_for_ui(value: &str) -> String {
        expand_percent_vars(value).to_ascii_lowercase()
    }

    fn path_entry_exists(value: &str) -> bool {
        let expanded = expand_percent_vars(value);
        std::path::Path::new(&expanded).exists()
    }

    fn expand_percent_vars(value: &str) -> String {
        let mut out = String::with_capacity(value.len());
        let chars: Vec<char> = value.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            if chars[i] == '%' {
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && chars[j] != '%' {
                    j += 1;
                }
                if j < chars.len() && j > start {
                    let key: String = chars[start..j].iter().collect();
                    if let Ok(v) = std::env::var(&key) {
                        out.push_str(&v);
                    } else {
                        out.push('%');
                        out.push_str(&key);
                        out.push('%');
                    }
                    i = j + 1;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

#[cfg(feature = "tui")]
pub(crate) use imp::run_env_tui;
