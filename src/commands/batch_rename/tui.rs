// commands/batch_rename/tui.rs
//
// TUI mode for batch rename — checkbox selection + confirm + apply.

use std::path::Path;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::batch_rename::types::RenameOp;
use crate::batch_rename::undo::{UndoRecord, write_undo};
use crate::output::{CliError, CliResult};

// ─── Public entry ────────────────────────────────────────────────────────────

pub(crate) fn run_brn_tui(ops: Vec<RenameOp>) -> CliResult {
    enable_raw_mode().map_err(|e| CliError::new(1, format!("TUI init: {}", e)))?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| CliError::new(1, format!("TUI init: {}", e)))?;
    let backend = CrosstermBackend::new(stdout);
    let mut term =
        Terminal::new(backend).map_err(|e| CliError::new(1, format!("TUI init: {}", e)))?;

    let result = App::new(ops).run(&mut term);

    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = term.show_cursor();

    result
}

// ─── App state ───────────────────────────────────────────────────────────────

struct App {
    ops: Vec<RenameOp>,
    selected: Vec<bool>,
    list_state: ListState,
    show_confirm: bool,
    message: Option<(String, bool)>,
}

impl App {
    fn new(ops: Vec<RenameOp>) -> Self {
        let n = ops.len();
        let mut list_state = ListState::default();
        if n > 0 {
            list_state.select(Some(0));
        }
        Self {
            selected: vec![true; n],
            ops,
            list_state,
            show_confirm: false,
            message: None,
        }
    }

    fn selected_count(&self) -> usize {
        self.selected.iter().filter(|&&s| s).count()
    }

    fn run(&mut self, term: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> CliResult {
        loop {
            term.draw(|f| self.render(f))
                .map_err(|e| CliError::new(1, format!("TUI draw: {}", e)))?;

            let ev = event::read().map_err(|e| CliError::new(1, format!("TUI event: {}", e)))?;

            if let Event::Key(key) = ev {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if self.show_confirm {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            self.apply_renames();
                            self.show_confirm = false;
                        }
                        _ => {
                            self.show_confirm = false;
                            self.message = Some(("Cancelled.".into(), false));
                        }
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
                    KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
                    KeyCode::Char(' ') => self.toggle_current(),
                    KeyCode::Char('a') | KeyCode::Char('A') => self.toggle_all(),
                    KeyCode::Enter => {
                        if self.selected_count() > 0 {
                            self.show_confirm = true;
                            self.message = None;
                        } else {
                            self.message = Some(("No files selected.".into(), true));
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn move_cursor(&mut self, delta: i32) {
        let n = self.ops.len();
        if n == 0 {
            return;
        }
        let cur = self.list_state.selected().unwrap_or(0) as i32;
        let next = (cur + delta).rem_euclid(n as i32) as usize;
        self.list_state.select(Some(next));
    }

    fn toggle_current(&mut self) {
        if let Some(i) = self.list_state.selected() {
            self.selected[i] = !self.selected[i];
        }
    }

    fn toggle_all(&mut self) {
        let all = self.selected.iter().all(|&s| s);
        self.selected.fill(!all);
    }

    fn apply_renames(&mut self) {
        let mut records: Vec<UndoRecord> = Vec::new();
        let mut errors = 0usize;
        let mut success = 0usize;

        for (i, op) in self.ops.iter().enumerate() {
            if !self.selected[i] {
                continue;
            }
            match std::fs::rename(&op.from, &op.to) {
                Ok(()) => {
                    success += 1;
                    records.push(UndoRecord {
                        from: op.to.to_string_lossy().into_owned(),
                        to: op.from.to_string_lossy().into_owned(),
                    });
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }

        if !records.is_empty() {
            let _ = write_undo(&records);
        }

        self.message = Some((
            format!("{} renamed, {} failed. Undo: xun brn undo", success, errors),
            errors > 0,
        ));
    }

    // ─── Render ────────────────────────────────────────────────────────────

    fn render(&mut self, f: &mut ratatui::Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(area);

        self.render_header(f, chunks[0]);
        self.render_list(f, chunks[1]);
        self.render_statusbar(f, chunks[2]);

        if self.show_confirm {
            self.render_confirm(f, area);
        }
    }

    fn render_header(&self, f: &mut ratatui::Frame, area: Rect) {
        let selected = self.selected_count();
        let total = self.ops.len();
        let title = format!(" brn  |  {} / {} selected ", selected, total);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan))
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(block, area);
    }

    fn render_list(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let max_from = self
            .ops
            .iter()
            .map(|o| filename_str(&o.from).len())
            .max()
            .unwrap_or(10);

        let items: Vec<ListItem> = self
            .ops
            .iter()
            .enumerate()
            .map(|(i, op)| {
                let checked = self.selected[i];
                let check_sym = if checked { "+" } else { "o" };
                let from_name = filename_str(&op.from);
                let to_name = filename_str(&op.to);

                let check_style = if checked {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let line = Line::from(vec![
                    Span::styled(format!(" {} ", check_sym), check_style),
                    Span::styled(
                        format!("{:<width$}", from_name, width = max_from),
                        Style::default().fg(if checked { Color::Red } else { Color::DarkGray }),
                    ),
                    Span::styled("  ->  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        to_name,
                        Style::default().fg(if checked {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }),
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let block_title = self
            .message
            .as_ref()
            .map(|(msg, _)| format!(" {} ", msg))
            .unwrap_or_else(|| " Files ".into());

        let msg_style = self
            .message
            .as_ref()
            .map(|(_, is_err)| {
                if *is_err {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Green)
                }
            })
            .unwrap_or_default();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(block_title)
                    .title_style(msg_style)
                    .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(30, 30, 50))
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_statusbar(&self, f: &mut ratatui::Frame, area: Rect) {
        let keys = Line::from(vec![
            Span::styled("  [Space] ", Style::default().fg(Color::DarkGray)),
            Span::raw("select  "),
            Span::styled("  [A] ", Style::default().fg(Color::DarkGray)),
            Span::raw("all  "),
            Span::styled("  [Enter] ", Style::default().fg(Color::DarkGray)),
            Span::raw("apply  "),
            Span::styled("  [Q] ", Style::default().fg(Color::DarkGray)),
            Span::raw("quit  "),
        ]);
        f.render_widget(Paragraph::new(keys), area);
    }

    fn render_confirm(&self, f: &mut ratatui::Frame, area: Rect) {
        let count = self.selected_count();
        let popup_area = centered_rect(44, 5, area);
        f.render_widget(Clear, popup_area);

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  Rename "),
                Span::styled(
                    format!("{}", count),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" file(s)?  "),
                Span::styled(
                    "[Y]",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" yes  "),
                Span::styled(
                    "[N]",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" cancel"),
            ]),
            Line::from(""),
        ];

        let popup = Paragraph::new(text).block(
            Block::default()
                .title(" Confirm ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        f.render_widget(popup, popup_area);
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn filename_str(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_owned()
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
