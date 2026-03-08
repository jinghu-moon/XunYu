use super::*;

pub(super) fn draw_ui(f: &mut ratatui::Frame, app: &mut App) {
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
                .map(|v| ListItem::new(format!("{} = {}", v.name, trim_for_ui(&v.raw_value, 96))))
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
