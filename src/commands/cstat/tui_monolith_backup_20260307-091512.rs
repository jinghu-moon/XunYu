// commands/cstat/tui.rs
//
// TUI mode for cstat — Stats tab + Issues tab with delete/export.

use std::path::PathBuf;

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
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table,
        TableState,
    },
};

use crate::cstat::report::{Report, fmt_bytes};
use crate::output::{CliError, CliResult};

// ─── Public entry ────────────────────────────────────────────────────────────

pub(crate) fn run_cstat_tui(report: Report, scan_path: &str) -> CliResult {
    enable_raw_mode().map_err(|e| CliError::new(1, format!("TUI init: {}", e)))?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| CliError::new(1, format!("TUI init: {}", e)))?;
    let backend = CrosstermBackend::new(stdout);
    let mut term =
        Terminal::new(backend).map_err(|e| CliError::new(1, format!("TUI init: {}", e)))?;

    let result = App::new(report, scan_path.to_owned()).run(&mut term);

    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = term.show_cursor();

    result
}

// ─── Tab enum ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Stats,
    Issues,
}

// ─── App state ───────────────────────────────────────────────────────────────

struct App {
    report: Report,
    scan_path: String,
    tab: Tab,
    stats_state: TableState,
    issues_state: ListState,
    show_confirm: Option<PathBuf>,
    message: Option<(String, bool)>,
}

impl App {
    fn new(report: Report, scan_path: String) -> Self {
        let mut stats_state = TableState::default();
        if !report.stats.is_empty() {
            stats_state.select(Some(0));
        }
        let issues_state = ListState::default();
        App {
            report,
            scan_path,
            tab: Tab::Stats,
            stats_state,
            issues_state,
            show_confirm: None,
            message: None,
        }
    }

    fn issue_items(&self) -> Vec<(String, Option<PathBuf>)> {
        let mut items: Vec<(String, Option<PathBuf>)> = Vec::new();
        for p in &self.report.issues.empty {
            items.push((format!("[empty]  {}", p), Some(PathBuf::from(p))));
        }
        for (p, lines) in &self.report.issues.large {
            items.push((format!("[large {}L]  {}", lines, p), Some(PathBuf::from(p))));
        }
        for p in &self.report.issues.tmp {
            items.push((format!("[tmp]  {}", p), Some(PathBuf::from(p))));
        }
        for group in &self.report.issues.dup {
            for p in group {
                items.push((format!("[dup]  {}", p), Some(PathBuf::from(p))));
            }
            items.push(("".into(), None));
        }
        items
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

                // Confirm dialog takes priority
                if self.show_confirm.is_some() {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Some(path) = self.show_confirm.take() {
                                match std::fs::remove_file(&path) {
                                    Ok(()) => {
                                        self.remove_from_issues(&path);
                                        self.message = Some(("Deleted.".into(), false));
                                    }
                                    Err(e) => {
                                        self.message = Some((format!("Error: {}", e), true));
                                    }
                                }
                            }
                        }
                        _ => {
                            self.show_confirm = None;
                            self.message = Some(("Cancelled.".into(), false));
                        }
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Tab => {
                        self.tab = match self.tab {
                            Tab::Stats => Tab::Issues,
                            Tab::Issues => Tab::Stats,
                        };
                        self.message = None;
                    }
                    KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
                    KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
                    KeyCode::Char('d') | KeyCode::Char('D') => self.delete_selected(),
                    KeyCode::Char('e') | KeyCode::Char('E') => self.export_json(),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn move_cursor(&mut self, delta: i32) {
        match self.tab {
            Tab::Stats => {
                let n = self.report.stats.len();
                if n == 0 {
                    return;
                }
                let cur = self.stats_state.selected().unwrap_or(0) as i32;
                let next = (cur + delta).rem_euclid(n as i32) as usize;
                self.stats_state.select(Some(next));
            }
            Tab::Issues => {
                let items = self.issue_items();
                let n = items.len();
                if n == 0 {
                    return;
                }
                let cur = self.issues_state.selected().unwrap_or(0) as i32;
                let next = (cur + delta).rem_euclid(n as i32) as usize;
                self.issues_state.select(Some(next));
            }
        }
    }

    fn delete_selected(&mut self) {
        if self.tab != Tab::Issues {
            return;
        }
        let items = self.issue_items();
        if let Some(idx) = self.issues_state.selected() {
            if let Some((_, Some(path))) = items.get(idx) {
                self.show_confirm = Some(path.clone());
                self.message = None;
            }
        }
    }

    fn remove_from_issues(&mut self, path: &PathBuf) {
        let p = path.to_string_lossy().into_owned();
        self.report.issues.empty.retain(|x| x != &p);
        self.report.issues.large.retain(|(x, _)| x != &p);
        self.report.issues.tmp.retain(|x| x != &p);
        for group in &mut self.report.issues.dup {
            group.retain(|x| x != &p);
        }
        self.report.issues.dup.retain(|g| g.len() > 1);
    }

    fn export_json(&mut self) {
        match serde_json::to_string_pretty(&self.report) {
            Ok(json) => {
                let out = "cstat-report.json";
                match std::fs::write(out, json) {
                    Ok(()) => self.message = Some((format!("Exported to {}", out), false)),
                    Err(e) => self.message = Some((format!("Export error: {}", e), true)),
                }
            }
            Err(e) => self.message = Some((format!("Serialise error: {}", e), true)),
        }
    }

    // ─── Render ────────────────────────────────────────────────────────────

    fn render(&mut self, f: &mut ratatui::Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // header
                Constraint::Length(2), // tabs
                Constraint::Min(3),    // body
                Constraint::Length(1), // status
            ])
            .split(area);

        self.render_header(f, chunks[0]);
        self.render_tabs(f, chunks[1]);

        match self.tab {
            Tab::Stats => self.render_stats(f, chunks[2]),
            Tab::Issues => self.render_issues(f, chunks[2]),
        }

        self.render_statusbar(f, chunks[3]);

        if self.show_confirm.is_some() {
            self.render_confirm(f, area);
        }
    }

    fn render_header(&self, f: &mut ratatui::Frame, area: Rect) {
        let total_files: u32 = self.report.stats.iter().map(|s| s.files).sum();
        let total_code: u32 = self.report.stats.iter().map(|s| s.code).sum();
        let total_issues = self.report.issues.empty.len()
            + self.report.issues.large.len()
            + self.report.issues.tmp.len()
            + self
                .report
                .issues
                .dup
                .iter()
                .map(|g| g.len())
                .sum::<usize>();

        let issue_str = if total_issues > 0 {
            format!("  ! {} issues", total_issues)
        } else {
            "  ok".into()
        };

        let title = format!(
            " cstat  |  {}  |  {} files  |  {} code lines{}  ",
            self.scan_path, total_files, total_code, issue_str
        );

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta))
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(block, area);
    }

    fn render_tabs(&self, f: &mut ratatui::Frame, area: Rect) {
        let tabs = Line::from(vec![
            tab_span("  Stats  ", self.tab == Tab::Stats),
            Span::raw("  "),
            tab_span("  Issues ", self.tab == Tab::Issues),
            Span::styled(
                "                          Tab: switch panels",
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        f.render_widget(Paragraph::new(tabs), area);
    }

    fn render_stats(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let header_cells = [
            "Language", "Files", "Code", "Comment", "Blank", "Total", "Bytes",
        ]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let total_code: u32 = self.report.stats.iter().map(|s| s.code).sum();

        let rows: Vec<Row> = self
            .report
            .stats
            .iter()
            .map(|s| {
                let pct = if total_code > 0 {
                    format!("{:.0}%", s.code as f64 / total_code as f64 * 100.0)
                } else {
                    "0%".into()
                };
                Row::new(vec![
                    Cell::from(s.name.clone()).style(Style::default().fg(Color::White)),
                    Cell::from(s.files.to_string()).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(format!("{} ({})", s.code, pct))
                        .style(Style::default().fg(Color::Green)),
                    Cell::from(s.comment.to_string()).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(s.blank.to_string()).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(s.total_lines().to_string())
                        .style(Style::default().fg(Color::White)),
                    Cell::from(fmt_bytes(s.bytes)).style(Style::default().fg(Color::DarkGray)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(14),
                Constraint::Length(7),
                Constraint::Length(16),
                Constraint::Length(9),
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Length(10),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(25, 25, 45))
                .add_modifier(Modifier::BOLD),
        );

        f.render_stateful_widget(table, area, &mut self.stats_state);
    }

    fn render_issues(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let items_data = self.issue_items();

        if items_data.is_empty() {
            let p = Paragraph::new(Line::from(vec![Span::styled(
                "  No issues found.",
                Style::default().fg(Color::Green),
            )]))
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(p, area);
            return;
        }

        let items: Vec<ListItem> = items_data
            .iter()
            .map(|(label, _path)| {
                if label.is_empty() {
                    return ListItem::new(Line::from(Span::styled(
                        "  ────────────────────",
                        Style::default().fg(Color::Rgb(40, 40, 40)),
                    )));
                }
                let color = if label.starts_with("[dup]") {
                    Color::Red
                } else if label.starts_with("[large") {
                    Color::Yellow
                } else if label.starts_with("[tmp]") {
                    Color::Magenta
                } else {
                    Color::DarkGray
                };
                ListItem::new(Line::from(Span::styled(
                    format!("  {}", label),
                    Style::default().fg(color),
                )))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(
                        " Issues: {} empty  {} large  {} tmp  {} dup groups ",
                        self.report.issues.empty.len(),
                        self.report.issues.large.len(),
                        self.report.issues.tmp.len(),
                        self.report.issues.dup.len(),
                    ))
                    .title_style(Style::default().fg(Color::Yellow))
                    .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 20, 20))
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.issues_state);
    }

    fn render_statusbar(&self, f: &mut ratatui::Frame, area: Rect) {
        let msg = self.message.as_ref().map(|(m, is_err)| {
            let color = if *is_err { Color::Red } else { Color::Green };
            Span::styled(format!("  {}  ", m), Style::default().fg(color))
        });

        let mut spans = vec![
            Span::styled("  [Tab] ", Style::default().fg(Color::DarkGray)),
            Span::raw("switch  "),
            Span::styled("  [D] ", Style::default().fg(Color::DarkGray)),
            Span::raw("delete  "),
            Span::styled("  [E] ", Style::default().fg(Color::DarkGray)),
            Span::raw("export  "),
            Span::styled("  [Q] ", Style::default().fg(Color::DarkGray)),
            Span::raw("quit  "),
        ];

        if let Some(m) = msg {
            spans.push(m);
        }

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn render_confirm(&self, f: &mut ratatui::Frame, area: Rect) {
        let path_str = self
            .show_confirm
            .as_ref()
            .map(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_owned()
            })
            .unwrap_or_default();

        let popup_area = centered_rect(54, 6, area);
        f.render_widget(Clear, popup_area);

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  Delete: {}", path_str),
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "[Y]",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" confirm   "),
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
                .title(" Delete File ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Red)),
        );
        f.render_widget(popup, popup_area);
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn tab_span(label: &str, active: bool) -> Span<'_> {
    if active {
        Span::styled(
            label,
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(30, 30, 60))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(label, Style::default().fg(Color::DarkGray))
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
