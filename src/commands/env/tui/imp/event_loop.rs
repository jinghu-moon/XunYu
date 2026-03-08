use super::*;

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
                    app.status =
                        "warning: system scope write requires Administrator token".to_string();
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
