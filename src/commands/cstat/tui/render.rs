use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table},
};

use crate::cstat::report::fmt_bytes;

use super::app_state::{App, Tab};

impl App {
    pub(super) fn render(&mut self, f: &mut ratatui::Frame) {
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
